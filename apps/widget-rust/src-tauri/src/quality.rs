//! Hallucination scoring for transcription results.
//!
//! Whisper invents text most often on silence, noise, and when a repetitive
//! decode path locks in. Three decoder metrics catch the bulk of it:
//! `no_speech_prob` (silence transcribed as speech), `avg_logprob` (model
//! unsure), and the gzip compression ratio of the text (degenerate repetition).
//! Text heuristics catch the rest, mainly the training-data phrases whisper
//! emits over silence ("Thanks for watching").
//!
//! Flags are split into hard and soft. Hard flags condemn a segment on their
//! own. Soft flags — the phrase blocklist — only condemn when no decoder
//! metrics are available, because "Thank you." is both a stock hallucination
//! and a perfectly normal thing to dictate.

use crate::settings::Settings;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

const HALLUCINATION_PHRASES: &[&str] = &[
    "thanks for watching",
    "thank you for watching",
    "thanks for listening",
    "thank you for listening",
    "subscribe",
    "like and subscribe",
    "see you next time",
    "the end",
    "silence",
    "no speech",
    "inaudible",
    "[music]",
    "(music)",
];

const HALLUCINATION_WORDS: &[&str] = &[
    "you", "i", "so", "uh", "um", "hmm", "huh", "ah", "oh", "bye", "goodbye",
];

const HALLUCINATION_FRAGMENTS: &[&str] = &[
    "thank you",
    "thanks for",
    "i'm alone",
    "im alone",
    "subscribe",
    "see you next",
    "bye bye",
];

/// Below this length gzip's ~20 byte header dominates and the ratio is noise,
/// so repetition in short text is left to `has_repetitive_clauses`.
const MIN_CHARS_FOR_COMPRESSION: usize = 60;

/// Used to weight segments when the server omits timestamps.
const CHARS_PER_SECOND_ESTIMATE: f64 = 15.0;

/// Whisper's end-of-stream repetition loop sprays the same rolling phrase out
/// as a long run of near-identical, impossibly short segments. The per-segment
/// gates miss it because each segment is individually short and the duplication
/// sits *between* segments, so a cross-segment check is needed.
const LOOP_RUN_MIN: usize = 4;
const LOOP_RUN_SIMILARITY: f64 = 0.55;
const LOOP_RUN_MAX_SPAN_S: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    NoSpeech,
    LowConfidence,
    Repetition,
    SoundEffects,
    Empty,
    BlockedPhrase,
}

impl Flag {
    pub fn label(self) -> &'static str {
        match self {
            Flag::NoSpeech => "no_speech",
            Flag::LowConfidence => "low_confidence",
            Flag::Repetition => "repetition",
            Flag::SoundEffects => "sound_effects",
            Flag::Empty => "empty",
            Flag::BlockedPhrase => "blocked_phrase",
        }
    }

    fn is_hard(self) -> bool {
        !matches!(self, Flag::BlockedPhrase)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub no_speech_prob: f32,
    pub avg_logprob: f32,
    pub compression_ratio: f32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            no_speech_prob: 0.6,
            avg_logprob: -1.0,
            compression_ratio: 2.4,
        }
    }
}

impl Thresholds {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            no_speech_prob: settings.quality_no_speech_prob,
            avg_logprob: settings.quality_avg_logprob,
            compression_ratio: settings.quality_compression_ratio,
        }
    }
}

/// One decoded segment plus whatever metrics the server reported.
#[derive(Debug, Clone, Default)]
pub struct Segment {
    pub text: String,
    pub start: f64,
    pub end: f64,
    pub no_speech_prob: Option<f32>,
    pub avg_logprob: Option<f32>,
}

impl Segment {
    pub fn text_only(text: &str) -> Self {
        Self {
            text: text.to_string(),
            ..Default::default()
        }
    }

    pub fn has_metrics(&self) -> bool {
        self.no_speech_prob.is_some() || self.avg_logprob.is_some()
    }

    fn weight(&self) -> f64 {
        let duration = self.end - self.start;
        if duration > 0.0 {
            duration
        } else {
            (self.text.trim().len() as f64 / CHARS_PER_SECOND_ESTIMATE).max(0.1)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegmentReport {
    pub text: String,
    pub flags: Vec<Flag>,
    pub bad: bool,
    pub weight: f64,
    pub avg_logprob: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub raw_text: String,
    /// `raw_text` with hallucinated segments removed.
    pub clean_text: String,
    pub segments: Vec<SegmentReport>,
    /// Higher is better. Dominated by the share of good audio, with mean
    /// `avg_logprob` as a tiebreak between otherwise equal candidates.
    pub score: f32,
    pub dropped: usize,
    pub had_metrics: bool,
}

impl Report {
    /// A result with nothing dropped and something left to paste.
    pub fn is_clean(&self) -> bool {
        self.dropped == 0 && !self.clean_text.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.clean_text.is_empty()
    }

    /// Accepts the text as-is, for when the guard is switched off.
    pub fn passthrough(raw_text: &str) -> Self {
        let text = raw_text.trim().to_string();
        Self {
            raw_text: text.clone(),
            clean_text: text,
            segments: Vec::new(),
            score: 0.0,
            dropped: 0,
            had_metrics: false,
        }
    }

    /// The text that was thrown away, for tuning thresholds against real logs.
    pub fn dropped_text(&self) -> String {
        self.segments
            .iter()
            .filter(|s| s.bad && !s.text.is_empty())
            .map(|s| s.text.as_str())
            .collect::<Vec<&str>>()
            .join(" | ")
    }

    /// Comma-separated flags behind the dropped segments, for logs and errors.
    pub fn reason(&self) -> String {
        let mut labels: Vec<&str> = Vec::new();
        for segment in self.segments.iter().filter(|s| s.bad) {
            for flag in &segment.flags {
                let label = flag.label();
                if !labels.contains(&label) {
                    labels.push(label);
                }
            }
        }
        if labels.is_empty() {
            "clean".to_string()
        } else {
            labels.join(", ")
        }
    }
}

/// `len(text) / len(gzip(text))`. Repetitive text compresses far better than
/// natural speech, so a high ratio means the decoder is looping.
pub fn compression_ratio(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    if encoder.write_all(text.as_bytes()).is_err() {
        return 0.0;
    }
    match encoder.finish() {
        Ok(compressed) if !compressed.is_empty() => text.len() as f32 / compressed.len() as f32,
        _ => 0.0,
    }
}

fn looks_like_sound_effect_markup(text: &str) -> bool {
    text.contains('*') && text.chars().filter(|c| *c == '*').count() >= 2
}

fn has_repetitive_clauses(text: &str) -> bool {
    let clauses: Vec<&str> = text
        .split(|c| matches!(c, '.' | '!' | '?'))
        .map(str::trim)
        .filter(|s| s.len() > 5)
        .collect();

    if clauses.len() >= 2 {
        let mut seen = std::collections::HashSet::new();
        for clause in clauses {
            if !seen.insert(clause) {
                return true;
            }
        }
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() >= 6 && words.len() % 2 == 0 {
        let half = words.len() / 2;
        if words[..half] == words[half..] {
            return true;
        }
    }

    false
}

fn matches_blocklist(text: &str) -> bool {
    if HALLUCINATION_WORDS.iter().any(|w| *w == text) {
        return true;
    }
    if text.len() < 40 && HALLUCINATION_PHRASES.iter().any(|p| text.contains(p)) {
        return true;
    }
    if text.len() < 120 && HALLUCINATION_FRAGMENTS.iter().any(|p| text.contains(p)) {
        return true;
    }
    false
}

fn push_unique(flags: &mut Vec<Flag>, flag: Flag) {
    if !flags.contains(&flag) {
        flags.push(flag);
    }
}

/// Dice similarity between the word sets of two texts. Identical rolling loops
/// score ~1.0 because they contain the same words even as each segment shifts.
fn word_set_similarity(a: &str, b: &str) -> f64 {
    let wa: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let wb: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if wa.is_empty() || wb.is_empty() {
        return 0.0;
    }
    let inter = wa.intersection(&wb).count();
    (2.0 * inter as f64) / (wa.len() + wb.len()) as f64
}

/// Flag a run of consecutive near-identical segments packed into a tiny audio
/// window. Whisper's loop emits bursts of impossibly short segments, so only
/// short segments (≤ `LOOP_RUN_MAX_SEG_SECS`) take part in the run, and the
/// whole burst must still fit in `LOOP_RUN_MAX_SPAN_S`. Real-time repetition —
/// even a chant repeated four times — never passes the shortness gate. Returns
/// the number of newly-flagged segments.
fn flag_loop_run(segments: &[Segment], reports: &mut [SegmentReport]) -> usize {
    const SEG_SHORT: f64 = 0.6;
    let n = segments.len().min(reports.len());
    if n < LOOP_RUN_MIN {
        return 0;
    }
    let short = |s: &Segment| s.end - s.start <= SEG_SHORT;
    let mut i = 0;
    let mut flagged = 0;
    while i + LOOP_RUN_MIN <= n {
        if !short(&segments[i])
            || !short(&segments[i + 1])
            || word_set_similarity(&segments[i].text, &segments[i + 1].text)
                < LOOP_RUN_SIMILARITY
        {
            i += 1;
            continue;
        }
        let mut end = i + 2;
        while end < n
            && short(&segments[end])
            && word_set_similarity(&segments[end - 1].text, &segments[end].text)
                >= LOOP_RUN_SIMILARITY
        {
            end += 1;
        }
        if end - i >= LOOP_RUN_MIN
            && segments[end - 1].end - segments[i].start <= LOOP_RUN_MAX_SPAN_S
        {
            for r in &mut reports[i..end] {
                if !r.bad {
                    push_unique(&mut r.flags, Flag::Repetition);
                    r.bad = true;
                    flagged += 1;
                }
            }
        }
        i = end;
    }
    flagged
}

pub fn evaluate_segment(segment: &Segment, thresholds: &Thresholds) -> SegmentReport {
    let text = segment.text.trim().to_string();
    let lower = text.to_lowercase();
    let mut flags = Vec::new();

    if text.len() < 2 {
        push_unique(&mut flags, Flag::Empty);
    }

    if let Some(prob) = segment.no_speech_prob {
        if prob > thresholds.no_speech_prob {
            push_unique(&mut flags, Flag::NoSpeech);
        }
    }

    if let Some(logprob) = segment.avg_logprob {
        if logprob < thresholds.avg_logprob {
            push_unique(&mut flags, Flag::LowConfidence);
        }
    }

    if text.len() >= MIN_CHARS_FOR_COMPRESSION
        && compression_ratio(&text) > thresholds.compression_ratio
    {
        push_unique(&mut flags, Flag::Repetition);
    }

    if has_repetitive_clauses(&lower) {
        push_unique(&mut flags, Flag::Repetition);
    }

    if looks_like_sound_effect_markup(&lower) {
        push_unique(&mut flags, Flag::SoundEffects);
    }

    if matches_blocklist(&lower) {
        push_unique(&mut flags, Flag::BlockedPhrase);
    }

    let has_hard = flags.iter().any(|f| f.is_hard());
    let bad = has_hard || (!flags.is_empty() && !segment.has_metrics());

    SegmentReport {
        text,
        flags,
        bad,
        weight: segment.weight(),
        avg_logprob: segment.avg_logprob,
    }
}

/// Score a full result, dropping hallucinated segments from `clean_text`.
/// When the server reported no segments, the whole text is scored as one.
pub fn evaluate(raw_text: &str, segments: &[Segment], thresholds: &Thresholds) -> Report {
    let fallback = [Segment::text_only(raw_text)];
    let segments = if segments.is_empty() {
        &fallback[..]
    } else {
        segments
    };

    let had_metrics = segments.iter().any(Segment::has_metrics);
    let mut reports: Vec<SegmentReport> = segments
        .iter()
        .map(|s| evaluate_segment(s, thresholds))
        .collect();

    flag_loop_run(segments, &mut reports);

    let dropped = reports.iter().filter(|r| r.bad).count();
    let trimmed_raw = raw_text.trim();
    // Nothing dropped means the server's own spacing and paragraph breaks
    // survive; reassembling from segments would flatten them.
    let clean_text = if dropped == 0 && !trimmed_raw.is_empty() {
        trimmed_raw.to_string()
    } else {
        reports
            .iter()
            .filter(|r| !r.bad && !r.text.is_empty())
            .map(|r| r.text.as_str())
            .collect::<Vec<&str>>()
            .join(" ")
            .trim()
            .to_string()
    };

    let total_weight: f64 = reports.iter().map(|r| r.weight).sum();
    let good_weight: f64 = reports.iter().filter(|r| !r.bad).map(|r| r.weight).sum();
    let good_ratio = if total_weight > 0.0 {
        good_weight / total_weight
    } else {
        0.0
    };

    let good_logprobs: Vec<f32> = reports
        .iter()
        .filter(|r| !r.bad)
        .filter_map(|r| r.avg_logprob)
        .collect();
    let mean_logprob = if good_logprobs.is_empty() {
        -0.5
    } else {
        good_logprobs.iter().sum::<f32>() / good_logprobs.len() as f32
    };

    let score = good_ratio as f32 * 100.0 + mean_logprob.clamp(-4.0, 0.0) * 10.0;

    Report {
        raw_text: raw_text.trim().to_string(),
        clean_text,
        segments: reports,
        score,
        dropped,
        had_metrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scored(text: &str) -> Report {
        evaluate(text, &[], &Thresholds::default())
    }

    #[test]
    fn rejects_silence_hallucination_with_sound_effects() {
        let text =
            "*shriek* *shriek* ... ... I'm alone. I'm alone. I'm alone. Thank you. Thank you.";
        assert!(scored(text).is_empty());
    }

    #[test]
    fn rejects_common_youtube_outro() {
        assert!(scored("Thanks for watching!").is_empty());
    }

    #[test]
    fn accepts_real_short_phrase() {
        let report = scored("Send the report by Friday.");
        assert!(report.is_clean());
        assert_eq!(report.clean_text, "Send the report by Friday.");
    }

    #[test]
    fn keeps_short_utterance_the_blocklist_would_have_dropped() {
        let segment = Segment {
            text: "Thank you.".to_string(),
            start: 0.0,
            end: 1.2,
            no_speech_prob: Some(0.01),
            avg_logprob: Some(-0.2),
        };
        let report = evaluate("Thank you.", &[segment], &Thresholds::default());
        assert!(report.is_clean(), "confident metrics should override blocklist");
    }

    #[test]
    fn drops_only_the_bad_segment() {
        let segments = vec![
            Segment {
                text: "Ship the release notes today.".to_string(),
                start: 0.0,
                end: 3.0,
                no_speech_prob: Some(0.02),
                avg_logprob: Some(-0.3),
            },
            Segment {
                text: "Thanks for watching!".to_string(),
                start: 3.0,
                end: 6.0,
                no_speech_prob: Some(0.94),
                avg_logprob: Some(-1.6),
            },
        ];
        let report = evaluate("ignored", &segments, &Thresholds::default());
        assert_eq!(report.clean_text, "Ship the release notes today.");
        assert_eq!(report.dropped, 1);
        assert!(!report.is_clean());
    }

    #[test]
    fn repetition_loop_compresses_far_better_than_speech() {
        let looped = "I will be there. ".repeat(12);
        assert!(compression_ratio(&looped) > Thresholds::default().compression_ratio);
        assert!(
            compression_ratio("Please review the migration plan before the standup tomorrow.")
                < Thresholds::default().compression_ratio
        );
    }

    #[test]
    fn score_prefers_the_candidate_with_less_dropped_audio() {
        let good = evaluate(
            "Ship it.",
            &[Segment {
                text: "Ship it.".to_string(),
                start: 0.0,
                end: 2.0,
                no_speech_prob: Some(0.01),
                avg_logprob: Some(-0.2),
            }],
            &Thresholds::default(),
        );
        let bad = evaluate(
            "Thanks for watching!",
            &[Segment {
                text: "Thanks for watching!".to_string(),
                start: 0.0,
                end: 2.0,
                no_speech_prob: Some(0.95),
                avg_logprob: Some(-1.4),
            }],
            &Thresholds::default(),
        );
        assert!(good.score > bad.score);
    }

    /// Shape of a real large-v3 failure (content replaced with synthetic
    /// placeholder text): a couple of seconds of real speech followed by
    /// whisper's end-of-stream loop spraying the same rolling phrase out as a
    /// burst of impossibly short, identical segments. The whole loop must be
    /// dropped while the real speech survives.
    #[test]
    fn drops_whisper_repetition_loop_but_keeps_real_speech() {
        let real = [
            Segment {
                text: "and that is why I knew the exact route to the old station".to_string(),
                start: 66.7,
                end: 68.4,
                no_speech_prob: Some(0.01),
                avg_logprob: Some(-0.18),
            },
            Segment {
                text: "and I knew the exact route to the old station so yes I".to_string(),
                start: 68.4,
                end: 70.1,
                no_speech_prob: Some(0.02),
                avg_logprob: Some(-0.15),
            },
        ];
        let words: Vec<&str> = "and I knew the exact route to the old station"
            .split_whitespace()
            .collect();
        let mut segments: Vec<Segment> = real.to_vec();
        for k in 0..8 {
            let text = words
                .iter()
                .cycle()
                .skip(k % words.len())
                .take(words.len())
                .cloned()
                .collect::<Vec<&str>>()
                .join(" ");
            segments.push(Segment {
                text,
                start: 70.2,
                end: 70.3,
                no_speech_prob: Some(0.01),
                avg_logprob: Some(-0.2),
            });
        }

        let report = evaluate("", &segments, &Thresholds::default());
        assert!(report.dropped >= 8, "loop segments must be dropped, got {}", report.dropped);
        assert_eq!(
            report.clean_text.matches("the exact route to the old station").count(),
            2,
            "exactly the two real utterances must survive: {}",
            report.clean_text
        );
        assert!(
            report.clean_text.contains("that is why I knew the exact route"),
            "real speech must survive: {}",
            report.clean_text
        );
    }

    /// A genuinely repeated spoken phrase (each repetition takes real time) must
    /// NOT be flagged — the span gate separates it from a processor loop.
    #[test]
    fn keeps_repeated_speech_spread_over_real_time() {
        let mut segments = Vec::new();
        for i in 0..4u32 {
            segments.push(Segment {
                text: "and I knew the exact route to the old station".to_string(),
                start: i as f64,
                end: (i + 1) as f64,
                no_speech_prob: Some(0.01),
                avg_logprob: Some(-0.2),
            });
        }
        let report = evaluate("", &segments, &Thresholds::default());
        assert_eq!(report.dropped, 0, "real-time repetition must not be dropped");
        assert!(report.is_clean());
    }
}

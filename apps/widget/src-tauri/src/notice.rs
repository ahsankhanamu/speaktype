use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum NoticeKind {
    Pasted,
    CopiedNotPasted,
    NoSpeech,
    Busy,
    LongAudio,
    ServerError,
    MicUnavailable,
    MicFallback,
    ProviderFallback,
    ScreenshotLimit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub kind: NoticeKind,
    pub message: String,
}

impl Notice {
    /// The spec §6.1 copy, verbatim: the widget renders `message` and never
    /// composes its own.
    pub fn of(kind: NoticeKind) -> Self {
        use NoticeKind::*;
        let message = match kind {
            Pasted => "Pasted",
            CopiedNotPasted => "Copied. Press ⌘V to paste",
            NoSpeech => "Didn't catch that",
            Busy => "Still working on the last one",
            LongAudio => "Long recording, this may take a moment",
            ServerError => "Speech model couldn't start",
            MicUnavailable => "Microphone unavailable",
            MicFallback => "Switched to MacBook microphone",
            ProviderFallback => "Switched to the speech model on this Mac",
            ScreenshotLimit => "Screenshot limit reached",
        };
        Self {
            kind,
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_copy_matches_spec() {
        use NoticeKind::*;
        let table = [
            (NoSpeech, "Didn't catch that"),
            (Busy, "Still working on the last one"),
            (CopiedNotPasted, "Copied. Press ⌘V to paste"),
            (ServerError, "Speech model couldn't start"),
            (LongAudio, "Long recording, this may take a moment"),
            (MicUnavailable, "Microphone unavailable"),
            (MicFallback, "Switched to MacBook microphone"),
        ];
        for (kind, message) in table {
            assert_eq!(Notice::of(kind).message, message);
        }
    }

    #[test]
    fn notice_is_one_plain_object_with_a_kind() {
        let as_json = serde_json::to_value(Notice::of(NoticeKind::Pasted)).unwrap();
        assert_eq!(as_json["kind"], "pasted");
        assert_eq!(as_json["message"], "Pasted");
        assert_eq!(
            serde_json::to_value(NoticeKind::ScreenshotLimit).unwrap(),
            "screenshotLimit"
        );
    }
}

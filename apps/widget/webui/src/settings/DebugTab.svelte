<script lang="ts">
  import { getDebugSessions, getDebugAudioSlice, deleteDebugSession, clearDebugSessions } from '../lib/ipc';

  interface Segment {
    start?: number;
    end?: number;
    text?: string;
    flags?: string[];
    bad?: boolean;
  }
  interface Pass {
    pass?: number;
    temperature?: number;
    promptUsed?: boolean;
    hadMetrics?: boolean;
    score?: number;
    dropped?: number;
    rawChars?: number;
    reason?: string;
    segments?: Segment[];
    chunkIdx?: number | null;
  }
  interface Chunk {
    idx?: number;
    status?: string;
    source?: string;
    chars?: string;
    durationSecs?: number;
    startSecs?: number;
    endSecs?: number;
    seededPrompt?: boolean;
    reason?: string;
  }
  interface Session {
    id: number;
    mode?: string;
    startedAt?: string;
    model?: string;
    durationSecs?: number;
    chunks?: Chunk[];
    elapsedMs?: number;
    finalChars?: number;
    finalText?: string;
    passes?: Pass[];
  }

  let { active = false }: { active?: boolean } = $props();

  let sessions: Session[] = $state([]);
  let loaded = $state(false);
  let openSessions: Record<number, boolean> = $state({});
  let openChunks: Record<string, boolean> = $state({});
  let clearArmed = $state(false);

  let listEl = $state<HTMLDivElement | null>(null);
  let clearTimer: number | null = null;
  let clearEl: HTMLButtonElement | null = null;

  // Audio playback (one global debug audio at a time).
  let debugAudio: { audio: HTMLAudioElement; objectUrl: string; key: string } | null = null;
  let activeWaveChunk: HTMLElement | null = null;

  const MODE_LABEL: Record<string, string> = {
    live: 'Live (pipelined)',
    single_shot: 'Single-shot',
    live_pipelined: 'Live (chunked)',
    post_hoc: 'Fallback (post-hoc chunks)',
    post_hoc_fallback: 'Fallback (post-hoc chunks)',
    chunked: 'Chunked (no live session)',
    reprocess: 'Reprocess from history',
  };
  const STATUS_LABEL: Record<string, string> = {
    pipelined: 'queued',
    accepted: 'accepted',
    accepted_cleaned: 'accepted (cleaned)',
    rejected: 'rejected',
    failed: 'failed',
    silence: 'silence',
    info: 'info',
  };

  function truncateText(text?: string, limit = 80): string {
    if (!text) return '';
    return text.length > limit ? text.slice(0, limit) + '…' : text;
  }

  function debugTimeRange(start?: number, end?: number, baseStart = 0): string {
    const a = (start != null ? start : 0) + baseStart;
    const b = (end != null ? end : start || 0) + baseStart;
    return `${a.toFixed(1)}s–${b.toFixed(1)}s`;
  }

  function startedDate(startedAt?: string): string {
    return startedAt ? startedAt.slice(0, 10) : '';
  }

  function startedTime(startedAt?: string): string {
    return startedAt && startedAt.length > 11 ? startedAt.slice(11) : (startedAt || '');
  }

  const groups = $derived.by(() => {
    const order: string[] = [];
    const map: Record<string, Session[]> = {};
    for (const s of sessions) {
      const key = startedDate(s.startedAt) || 'Unknown';
      if (!map[key]) {
        map[key] = [];
        order.push(key);
      }
      map[key].push(s);
    }
    return order.map((date) => ({ date, sessions: map[date] }));
  });

  function chunkBadgeText(chunk: Chunk): string {
    return STATUS_LABEL[chunk.status || ''] || chunk.status || '';
  }

  function playableSegments(base: number, segments?: Segment[]) {
    return (segments || [])
      .filter((s) => s.text && String(s.text).trim())
      .map((s) => ({
        start: (s.start != null ? s.start : 0) + base,
        end: (s.end != null ? s.end : s.start || 0) + base,
      }));
  }

  function toAudioBytes(raw: unknown): Uint8Array<ArrayBuffer> {
    const r = raw as Uint8Array | ArrayBuffer | number[] | null | undefined;
    if (r instanceof Uint8Array) {
      const copy = r.buffer.slice(r.byteOffset, r.byteOffset + r.byteLength) as ArrayBuffer;
      return new Uint8Array(copy);
    }
    if (r instanceof ArrayBuffer) return new Uint8Array(r.slice(0));
    if (Array.isArray(r)) return new Uint8Array(Uint8Array.from(r).buffer);
    return new Uint8Array(0);
  }

  // ---- Waveform ----
  function decodeWavSamples(bytes: Uint8Array): { samples: Float32Array; rate: number } {
    if (!bytes || bytes.byteLength <= 44) return { samples: new Float32Array(0), rate: 16000 };
    const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const rate = dv.getUint32(24, true);
    const n = Math.floor((bytes.byteLength - 44) / 2);
    const samples = new Float32Array(n);
    for (let i = 0; i < n; i++) samples[i] = dv.getInt16(44 + i * 2, true) / 32768;
    return { samples, rate };
  }

  function drawWaveform(canvas: HTMLCanvasElement, samples: Float32Array, progress: number) {
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth || canvas.offsetWidth || 320;
    const h = canvas.clientHeight || canvas.offsetHeight || 44;
    const pxW = Math.round(w * dpr);
    const pxH = Math.round(h * dpr);
    if (canvas.width !== pxW || canvas.height !== pxH) {
      canvas.width = pxW;
      canvas.height = pxH;
    }
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    if (!samples || !samples.length) return;
    if (progress > 0 && progress < 1) {
      ctx.fillStyle = 'rgba(128, 128, 128, 0.25)';
      ctx.fillRect(0, 0, w * progress, h);
    }
    const color = window.getComputedStyle(canvas).color || 'rgba(128,128,128,0.7)';
    ctx.fillStyle = color;
    const mid = h / 2;
    const barCount = Math.max(20, Math.floor(w / 3));
    const per = Math.max(1, Math.floor(samples.length / barCount));
    const barW = Math.max(1, w / barCount - 1);
    for (let i = 0; i < barCount; i++) {
      const from = i * per;
      const to = Math.min(samples.length, from + per);
      let peak = 0;
      for (let j = from; j < to; j++) {
        const a = Math.abs(samples[j]);
        if (a > peak) peak = a;
      }
      const bh = Math.max(1, peak * h * 0.9);
      ctx.fillRect((w / barCount) * i, mid - bh / 2, barW, bh);
    }
  }

  // ---- Playback ----
  function resetActiveWave() {
    if (!activeWaveChunk) return;
    const canvas = activeWaveChunk.querySelector('.debug-chunk-wave canvas') as HTMLCanvasElement | null;
    if (canvas && (canvas as HTMLCanvasElement & { __wave?: { samples: Float32Array } }).__wave) {
      drawWaveform(canvas, (canvas as HTMLCanvasElement & { __wave: { samples: Float32Array } }).__wave.samples, 0);
    }
    activeWaveChunk = null;
  }

  function stopPlayback() {
    if (debugAudio) {
      debugAudio.audio.pause();
      URL.revokeObjectURL(debugAudio.objectUrl);
      debugAudio = null;
    }
    document.querySelectorAll<HTMLElement>('.debug-play.active').forEach((el) => el.classList.remove('active'));
    document.querySelectorAll<HTMLElement>('.debug-session-body').forEach(() => {});
    resetActiveWave();
  }

  function onDebugTimeUpdate() {
    if (!debugAudio || !activeWaveChunk) return;
    const canvas = activeWaveChunk.querySelector('.debug-chunk-wave canvas') as HTMLCanvasElement | null;
    const wave = canvas && (canvas as HTMLCanvasElement & { __wave?: { samples: Float32Array; duration: number } }).__wave;
    if (!canvas || !wave) return;
    const dur = debugAudio.audio.duration || wave.duration || 1;
    const progress = dur > 0 ? debugAudio.audio.currentTime / dur : 0;
    drawWaveform(canvas, wave.samples, Math.min(1, Math.max(0, progress)));
  }

  function playSlice(chunkEl: HTMLElement | null, sessionId: number, startSecs: number, endSecs: number, key: string, btnEl: HTMLElement | null, seekSecs = 0) {
    if (debugAudio && debugAudio.key === key) {
      stopPlayback();
      return Promise.resolve();
    }
    stopPlayback();
    return getDebugAudioSlice(sessionId, startSecs, endSecs)
      .then((raw) => {
        const bytes = toAudioBytes(raw);
        if (!bytes.length) return;
        const audio = new Audio();
        const objectUrl = URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' }));
        audio.src = objectUrl;
        if (seekSecs > 0) {
          const applySeek = () => {
            try {
              audio.currentTime = Math.min(seekSecs, audio.duration || seekSecs);
            } catch (_) {}
          };
          if (audio.readyState >= 1) applySeek();
          else audio.addEventListener('loadedmetadata', applySeek, { once: true });
        }
        debugAudio = { audio, objectUrl, key };
        audio.play();
        audio.addEventListener('timeupdate', onDebugTimeUpdate);
        if (btnEl) btnEl.classList.add('active');
        if (btnEl && btnEl.classList.contains('debug-play-wave')) {
          audio.addEventListener('ended', () => {
            btnEl.classList.remove('active');
            resetActiveWave();
          });
        }
      })
      .catch((err) => console.error('Failed to load debug audio slice:', err));
  }

  function wireWaveform(chunkEl: HTMLElement, sessionId: number, startSecs: number, endSecs: number, key: string) {
    const waveEl = chunkEl.querySelector('.debug-chunk-wave');
    const canvas = waveEl && waveEl.querySelector('canvas') as HTMLCanvasElement | null;
    if (!waveEl || !canvas || (canvas as HTMLCanvasElement & { __waveLoaded?: boolean }).__waveLoaded) return;
    (canvas as HTMLCanvasElement & { __waveLoaded?: boolean }).__waveLoaded = true;

    waveEl.classList.add('loading');
    getDebugAudioSlice(sessionId, startSecs, endSecs)
      .then((raw) => {
        const bytes = toAudioBytes(raw);
        const { samples, rate } = decodeWavSamples(bytes);
        (canvas as HTMLCanvasElement & { __wave?: { samples: Float32Array; duration: number } }).__wave = {
          samples,
          duration: samples.length ? samples.length / rate : (endSecs - startSecs || 1),
        };
        drawWaveform(canvas, samples, 0);
      })
      .catch((err) => console.error('Failed to load chunk waveform:', err))
      .finally(() => waveEl.classList.remove('loading'));

    canvas.addEventListener('click', (ev) => {
      ev.stopPropagation();
      const rect = canvas.getBoundingClientRect();
      const frac = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width));
      const wave = (canvas as HTMLCanvasElement & { __wave?: { duration: number } }).__wave;
      const seek = (wave ? wave.duration : endSecs - startSecs) * frac;
      activeWaveChunk = chunkEl;
      playSlice(chunkEl, sessionId, startSecs, endSecs, key, null, seek);
    });
  }

  function playSegments(segments: { start: number; end: number }[], btnEl: HTMLElement, sessionId: number) {
    if (!segments.length) return;
    if (btnEl.dataset.playing === '1') {
      stopPlayback();
      btnEl.dataset.playing = '';
      return;
    }
    btnEl.dataset.playing = '1';
    btnEl.classList.add('active');
    let index = 0;
    const playNext = () => {
      if (btnEl.dataset.playing !== '1') return;
      const seg = segments[index];
      if (!seg) {
        btnEl.dataset.playing = '';
        btnEl.classList.remove('active');
        if (debugAudio) stopPlayback();
        return;
      }
      index += 1;
      const sStart = Number(seg.start) || 0;
      const sEnd = Number(seg.end) || sStart;
      const key =
        'segs:' + sessionId + ':' + index + ':' + sStart.toFixed(3) + ':' + sEnd.toFixed(3);
      playSlice(null, sessionId, sStart, sEnd, key, btnEl).then(() => {
        if (btnEl.dataset.playing !== '1') return;
        if (!debugAudio) {
          playNext();
          return;
        }
        const audioEl = debugAudio.audio;
        audioEl.addEventListener('ended', playNext, { once: true });
      });
    };
    playNext();
  }

  // Session / chunk toggle + expansion.
  function toggleSession(id: number) {
    openSessions[id] = !openSessions[id];
  }

  function toggleChunk(sessionId: number, idx: number, chunkStart: number, chunkEnd: number, chunkEl: HTMLElement) {
    const key = `${sessionId}:${idx}`;
    const wasOpen = !!openChunks[key];
    openChunks[key] = !wasOpen;
    requestAnimationFrame(() => {
      if (!wasOpen) wireWaveform(chunkEl, sessionId, chunkStart, chunkEnd, `chunk:${sessionId}:${idx}`);
    });
  }

  async function deleteSession(sessionId: number) {
    try {
      await deleteDebugSession(sessionId);
      sessions = sessions.filter((s) => s.id !== sessionId);
      if (sessions.length === 0) stopPlayback();
    } catch (e) {
      console.error('Failed to delete debug session:', e);
    }
  }

  async function onClearClick(btn: HTMLButtonElement) {
    if (!clearArmed) {
      clearArmed = true;
      clearEl = btn;
      btn.textContent = 'Clear log?';
      btn.classList.add('confirm-armed');
      clearTimer = window.setTimeout(() => resetClear(btn), 3000);
      return;
    }
    resetClear(btn);
    stopPlayback();
    try {
      await clearDebugSessions();
      sessions = [];
    } catch (e) {
      console.error(e);
    }
  }

  function resetClear(btn: HTMLButtonElement) {
    if (clearTimer) {
      clearTimeout(clearTimer);
      clearTimer = null;
    }
    clearArmed = false;
    clearEl = null;
    btn.textContent = 'Clear Log';
    btn.classList.remove('confirm-armed');
  }

  async function load() {
    let items: Session[] = [];
    try {
      items = (await getDebugSessions()) || [];
    } catch (e) {
      console.error('Failed to load debug sessions:', e);
      return;
    }
    stopPlayback();
    openSessions = {};
    openChunks = {};
    sessions = items;
    loaded = true;
  }

  let prevActive = false;
  $effect(() => {
    if (active && !prevActive) load();
    prevActive = active;
  });
  $effect(() => {
    if (!active && loaded) stopPlayback();
  });

  export function refresh() {
    return load();
  }

  export async function clearAll() {
    stopPlayback();
    try {
      await clearDebugSessions();
      sessions = [];
    } catch (e) {
      console.error(e);
    }
  }

  function onBodyClick(e: MouseEvent, sessionId: number) {
    const target = e.target as Element;
    const playBtn = target.closest<HTMLElement>('.debug-play');
    if (playBtn) {
      e.stopPropagation();
      const chunkEl = playBtn.closest<HTMLElement>('.debug-chunk');
      if (playBtn.classList.contains('debug-play-wave')) {
        activeWaveChunk = chunkEl || null;
      }
      const session = sessions.find((s) => s.id === sessionId);
      if (playBtn.getAttribute('data-play-segments')) {
        const segs = JSON.parse(playBtn.dataset.playSegments || '[]');
        playSegments(segs, playBtn, sessionId);
      } else {
        playSlice(
          chunkEl,
          sessionId,
          Number(playBtn.dataset.start),
          Number(playBtn.dataset.end),
          playBtn.dataset.key || '',
          playBtn,
        );
      }
      return;
    }
    const head = target.closest<HTMLElement>('.debug-chunk-head');
    if (head) {
      e.stopPropagation();
      const chunkEl = head.closest<HTMLElement>('.debug-chunk');
      if (!chunkEl) return;
      const idx = Number(chunkEl.dataset.index);
      const start = Number(chunkEl.dataset.start);
      const end = Number(chunkEl.dataset.end);
      toggleChunk(sessionId, idx, start, end, chunkEl);
      return;
    }
    const seg = target.closest<HTMLElement>('.debug-play-seg');
    if (seg) {
      e.stopPropagation();
    }
  }

  function onBodyKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    const head = (e.target as Element).closest<HTMLElement>('.debug-chunk-head');
    if (!head) return;
    e.preventDefault();
    head.click();
  }

  function passSegments(pass: Pass): Segment[] {
    return (pass.segments || []).filter((s) => s.text && String(s.text).trim());
  }
</script>

{#if loaded}
  {#if sessions.length === 0}
    <div class="debug-empty" id="debug-empty">No transcription sessions logged yet</div>
  {:else}
    <div class="debug-list" id="debug-list" bind:this={listEl}>
      {#each groups as group (group.date)}
        <div class="debug-date-group">
          <div class="debug-date-label">{group.date}</div>
          {#each group.sessions as session (session.id)}
            <div class="debug-session" class:open={!!openSessions[session.id]}>
              <div class="debug-session-head">
                <button
                  type="button"
                  class="debug-session-toggle"
                  aria-expanded={!!openSessions[session.id]}
                  onclick={() => toggleSession(session.id)}
                >
                  <span class="debug-session-title">
                    <span class="debug-session-id">#{session.id}</span>
                    <span class="debug-session-mode">{MODE_LABEL[session.mode || ''] || session.mode}</span>
                  </span>
                  <span class="debug-session-meta">
                    <span class="debug-session-time">{startedTime(session.startedAt)}</span>
                    <span class="debug-tag">{session.model || '?'}</span>
                    <span class="debug-tag">{session.durationSecs != null ? session.durationSecs.toFixed(1) : '?'}s audio</span>
                    <span class="debug-tag">{session.chunks ? session.chunks.length : 0} chunks</span>
                    <span class="debug-tag">{session.elapsedMs != null ? (session.elapsedMs / 1000).toFixed(1) : '?'}s wall</span>
                    <span class="debug-tag">{session.finalChars != null ? session.finalChars : 0} chars out</span>
                    <span class="debug-caret" aria-hidden="true"></span>
                  </span>
                </button>
                <button
                  type="button"
                  class="debug-session-delete"
                  title="Delete this session"
                  aria-label={'Delete session #' + session.id}
                  onclick={() => deleteSession(session.id)}
                >
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12"/></svg>
                </button>
              </div>
              {#if openSessions[session.id]}
                <div
                  class="debug-session-body"
                  id={'debug-body-' + session.id}
                  role="presentation"
                  onclick={(e) => onBodyClick(e, session.id)}
                  onkeydown={onBodyKeydown}
                >
              <div class="debug-section-label">Chunk timeline</div>
              {#if session.chunks && session.chunks.length}
                {#each session.chunks as chunk}
                  {#if chunk.status === 'silence'}
                    <div class="debug-chunk" data-index={chunk.idx} data-start={chunk.startSecs || 0} data-end={chunk.endSecs || chunk.startSecs || 0}>
                      <div class="debug-chunk-head" role="button" tabindex="0" aria-expanded="false" aria-label="Toggle decode passes">
                        <span class="debug-chunk-chevron" aria-hidden="true"></span>
                        <span class="debug-chip debug-chip-silence">silence</span>
                        <span class="debug-chunk-title">silence</span>
                        <span class="debug-chunk-range">{(chunk.durationSecs != null ? chunk.durationSecs.toFixed(1) : '?')}s · {debugTimeRange(chunk.startSecs, chunk.endSecs, 0)}</span>
                      </div>
                    </div>
                  {:else}
                    {@const passes = (session.passes || []).filter((p) => p.chunkIdx === chunk.idx)}
                    <div
                      class="debug-chunk"
                      class:open={!!openChunks[`${session.id}:${chunk.idx}`]}
                      data-index={chunk.idx}
                      data-start={chunk.startSecs || 0}
                      data-end={chunk.endSecs || chunk.startSecs || 0}
                    >
                      <div class="debug-chunk-head" role="button" tabindex="0" aria-expanded={!!openChunks[`${session.id}:${chunk.idx}`]} aria-label="Toggle decode passes">
                        <span class="debug-chunk-chevron" aria-hidden="true"></span>
                        <span class="debug-chip debug-chip-{chunk.status}">{chunkBadgeText(chunk)}</span>
                        <span class="debug-chunk-title">{chunk.source || ''} · chunk {chunk.idx}</span>
                        {#if chunk.chars}<span class="debug-chunk-chars">{chunk.chars} chars</span>{/if}
                        <span class="debug-chunk-range">{(chunk.durationSecs != null ? chunk.durationSecs.toFixed(1) : '?')}s · {debugTimeRange(chunk.startSecs, chunk.endSecs, 0)}</span>
                        {#if chunk.seededPrompt}<span class="debug-chunk-seeded">seeded next prompt</span>{/if}
                        {#if chunk.reason}<span class="debug-chunk-reason">{chunk.reason}</span>{/if}
                      </div>
                      {#if openChunks[`${session.id}:${chunk.idx}`]}
                        <div class="debug-chunk-body">
                          <div class="debug-chunk-wave">
                            <canvas></canvas>
                            <button
                              type="button"
                              class="debug-play debug-play-wave"
                              draggable="false"
                              title="Play this chunk"
                              data-session={session.id}
                              data-start={(chunk.startSecs || 0).toFixed(3)}
                              data-end={(chunk.endSecs || chunk.startSecs || 0).toFixed(3)}
                              data-key={`chunk:${session.id}:${chunk.idx}`}
                            >▶</button>
                          </div>
                          {#if passes.length}
                            {#each passes as pass}
                              <div class="debug-pass">
                                <div class="debug-pass-head">
                                  <span class="debug-tag">pass {pass.pass ?? ''}</span>
                                  <span class="debug-tag">temp {pass.temperature ?? ''}</span>
                                  <span class="debug-tag">{pass.promptUsed ? 'with' : 'without'} prompt</span>
                                  {#if pass.hadMetrics}<span class="debug-tag debug-tag-metrics">metrics</span>{/if}
                                  <span class="debug-tag">score {(pass.score != null ? pass.score : 0).toFixed(1)}</span>
                                  <span class="debug-tag" class:debug-tag-warn={(pass.dropped || 0) > 0}>{pass.dropped ?? 0} dropped</span>
                                  <span class="debug-tag">raw {pass.rawChars ?? ''} chars</span>
                                  <button
                                    type="button"
                                    class="debug-play debug-play-all"
                                    draggable="false"
                                    title="Play all kept segments in order"
                                    data-session={session.id}
                                    data-play-segments={JSON.stringify(playableSegments(chunk.startSecs || 0, pass.segments))}
                                  >▶ all</button>
                                </div>
                                <div class="debug-pass-reason">{pass.reason || 'ok'}</div>
                                <div class="debug-segments">
                                  {#each passSegments(pass) as seg}
                                    {@const start = (seg.start != null ? seg.start : 0) + (chunk.startSecs || 0)}
                                    {@const end = (seg.end != null ? seg.end : seg.start || 0) + (chunk.startSecs || 0)}
                                    <div class="debug-seg {seg.bad ? 'debug-seg-bad' : 'debug-seg-ok'}">
                                      <button
                                        type="button"
                                        class="debug-play debug-play-seg"
                                        draggable="false"
                                        title="Play this segment"
                                        data-session={session.id}
                                        data-start={start.toFixed(3)}
                                        data-end={end.toFixed(3)}
                                        data-key={`seg:${session.id}:${start.toFixed(3)}:${end.toFixed(3)}`}
                                      >▶</button>
                                      <span class="debug-seg-time">{debugTimeRange(seg.start, seg.end, chunk.startSecs || 0)}</span>
                                      <span class="debug-seg-text">{truncateText(seg.text, 80)}</span>
                                      {#if seg.flags && seg.flags.length}<em class="debug-seg-flags">({seg.flags.join(', ')})</em>{/if}
                                    </div>
                                  {/each}
                                </div>
                              </div>
                            {/each}
                          {:else}
                            <div class="debug-chunk-passnote">No decode passes for this chunk.</div>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/if}
                {/each}
              {:else}
                <div class="debug-muted">No chunk events.</div>
              {/if}

              {#if (session.passes || []).some((p) => p.chunkIdx === null || p.chunkIdx === undefined)}
                <div class="debug-section-label">Recording-level decode</div>
                {#each (session.passes || []).filter((p) => p.chunkIdx === null || p.chunkIdx === undefined) as pass}
                  <div class="debug-pass">
                    <div class="debug-pass-head">
                      <span class="debug-tag">pass {pass.pass ?? ''}</span>
                      <span class="debug-tag">temp {pass.temperature ?? ''}</span>
                      <span class="debug-tag">{pass.promptUsed ? 'with' : 'without'} prompt</span>
                      {#if pass.hadMetrics}<span class="debug-tag debug-tag-metrics">metrics</span>{/if}
                      <span class="debug-tag">score {(pass.score != null ? pass.score : 0).toFixed(1)}</span>
                      <span class="debug-tag" class:debug-tag-warn={(pass.dropped || 0) > 0}>{pass.dropped ?? 0} dropped</span>
                      <span class="debug-tag">raw {pass.rawChars ?? ''} chars</span>
                      <button
                        type="button"
                        class="debug-play debug-play-all"
                        draggable="false"
                        title="Play all kept segments in order"
                        data-session={session.id}
                        data-play-segments={JSON.stringify(playableSegments(0, pass.segments))}
                      >▶ all</button>
                    </div>
                    <div class="debug-pass-reason">{pass.reason || 'ok'}</div>
                    <div class="debug-segments">
                      {#each passSegments(pass) as seg}
                        {@const start = (seg.start != null ? seg.start : 0)}
                        {@const end = (seg.end != null ? seg.end : seg.start || 0)}
                        <div class="debug-seg {seg.bad ? 'debug-seg-bad' : 'debug-seg-ok'}">
                          <button
                            type="button"
                            class="debug-play debug-play-seg"
                            draggable="false"
                            title="Play this segment"
                            data-session={session.id}
                            data-start={start.toFixed(3)}
                            data-end={end.toFixed(3)}
                            data-key={`seg:${session.id}:${start.toFixed(3)}:${end.toFixed(3)}`}
                          >▶</button>
                          <span class="debug-seg-time">{debugTimeRange(seg.start, seg.end, 0)}</span>
                          <span class="debug-seg-text">{truncateText(seg.text, 80)}</span>
                          {#if seg.flags && seg.flags.length}<em class="debug-seg-flags">({seg.flags.join(', ')})</em>{/if}
                        </div>
                      {/each}
                    </div>
                  </div>
                {/each}
              {/if}

              <div class="debug-final">
                <div class="debug-section-label">Final text</div>
                <div class="debug-final-text">{session.finalText || ''}</div>
              </div>
            </div>
          {/if}
        </div>
      {/each}
        </div>
      {/each}
    </div>
  {/if}
{/if}
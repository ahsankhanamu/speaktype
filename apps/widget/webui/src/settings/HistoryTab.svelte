<script lang="ts">
  import { getHistory, getHistoryAudio, deleteHistoryEntry, deleteHistoryAudio, clearHistory, reprocessHistoryEntry } from '../lib/ipc';
  import { formatDuration } from '../lib/format';

  interface HistoryItem {
    text: string;
    timestamp: string;
    audio_file?: boolean;
    duration_secs?: number;
    [key: string]: unknown;
  }

  let { active = false }: { active?: boolean } = $props();

  let entries: HistoryItem[] = $state([]);
  let renderedCount = $state(0);
  let empty = $state(true);
  let loaded = $state(false);
  let reprocessing: Record<number, boolean> = $state({});
  let copiedIndex: Record<number, boolean> = $state({});
  let clearArmed = $state(false);

  const PAGE_SIZE = 30;
  let listRoot = $state<HTMLDivElement | null>(null);
  let sentinelEl = $state<HTMLDivElement | null>(null);
  let observer: IntersectionObserver | null = null;
  let armedBtn: HTMLButtonElement | null = null;
  let armedAction: string | null = null;
  let armedTimer: number | null = null;
  let clearTimer: number | null = null;
  let clearEl: HTMLButtonElement | null = null;

  const players = new Map<number, {
    audio: HTMLAudioElement;
    ensureLoaded: () => Promise<void>;
    setPlaying: (p: boolean) => void;
    reset: () => void;
    release: () => void;
  }>();
  let activePlayer: { audio: HTMLAudioElement; reset: () => void } | null = null;

  const COPY_ICON = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
  const COPIED_ICON = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#4ade80" stroke-width="2.5"><path d="M4.5 12.75l6 6 9-13.5"/></svg>';
  const DELETE_ICON = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>';

  function formatTimestamp(ts: string): string {
    try {
      return new Date(ts).toLocaleString();
    } catch (_) {
      return ts;
    }
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

  function actionIcon(action: string): string {
    switch (action) {
      case 'copy': return COPY_ICON;
      case 'delete': return DELETE_ICON;
      case 'delete-audio': return '♪⌫';
      default: return '';
    }
  }

  // Lazily build player state bound to a row's audio element.
  function getPlayer(index: number, wrap: HTMLDivElement) {
    const cached = players.get(index);
    if (cached) return cached;
    const audio = wrap.querySelector('audio') as HTMLAudioElement | null;
    const seek = wrap.querySelector('.history-seek') as HTMLInputElement | null;
    const cur = wrap.querySelector('.history-time-cur') as HTMLSpanElement | null;
    const total = wrap.querySelector('.history-time-total') as HTMLSpanElement | null;
    const playBtn = wrap.querySelector('.history-play-btn') as HTMLButtonElement | null;
    if (!audio) throw new Error('No audio element');

    let objectUrl: string | null = null;
    let loadedFlag = false;
    let loading = false;
    let duration = Number(wrap.dataset.durationHint) || 0;
    let seeking = false;

    const setProgressUI = (frac: number) => seek?.style.setProperty('--progress', `${Math.round(frac * 100)}%`);
    const setPlayingUI = (playing: boolean) => {
      if (playBtn) {
        playBtn.textContent = playing ? '⏸' : '▶';
        playBtn.classList.toggle('playing', playing);
      }
    };
    const updateTimeUI = () => {
      const current = audio.currentTime || 0;
      const totalD = audio.duration && Number.isFinite(audio.duration) ? audio.duration : duration;
      if (totalD > 0) duration = totalD;
      if (!seeking && duration > 0) setProgressUI(current / duration);
      if (cur) cur.textContent = formatDuration(current);
      if (total) total.textContent = ` / ${formatDuration(duration)}`;
    };
    const resetUI = () => {
      setPlayingUI(false);
      setProgressUI(0);
      updateTimeUI();
    };
    const ensureLoaded = async () => {
      if (loadedFlag || loading) return;
      loading = true;
      if (playBtn) playBtn.disabled = true;
      try {
        const raw = await getHistoryAudio(index);
        const bytes = toAudioBytes(raw);
        objectUrl = URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' }));
        audio.src = objectUrl;
        await new Promise<void>((resolve, reject) => {
          const onReady = () => {
            audio.removeEventListener('loadedmetadata', onReady);
            audio.removeEventListener('error', onErr);
            resolve();
          };
          const onErr = () => {
            audio.removeEventListener('loadedmetadata', onReady);
            audio.removeEventListener('error', onErr);
            reject(new Error('Failed to load audio'));
          };
          audio.addEventListener('loadedmetadata', onReady);
          audio.addEventListener('error', onErr);
          audio.load();
        });
        loadedFlag = true;
        if (seek) seek.disabled = false;
      } finally {
        loading = false;
        if (playBtn) playBtn.disabled = false;
      }
    };
    if (seek) {
      seek.addEventListener('input', () => {
        if (!loadedFlag || !duration) return;
        seeking = true;
        const t = (Number(seek.value) / 1000) * duration;
        setProgressUI(Number(seek.value) / 1000);
        if (cur) cur.textContent = formatDuration(t);
      });
      seek.addEventListener('change', () => {
        if (!loadedFlag || !duration) {
          seeking = false;
          return;
        }
        audio.currentTime = (Number(seek.value) / 1000) * duration;
        seeking = false;
        updateTimeUI();
      });
    }
    audio.addEventListener('timeupdate', updateTimeUI);
    audio.addEventListener('loadedmetadata', () => {
      if (audio.duration && Number.isFinite(audio.duration)) {
        duration = audio.duration;
        updateTimeUI();
      }
    });
    audio.addEventListener('ended', () => {
      resetUI();
      if (activePlayer && activePlayer.audio === audio) activePlayer = null;
    });

    const player = {
      audio,
      ensureLoaded,
      setPlaying: setPlayingUI,
      reset: resetUI,
      release() {
        audio.pause();
        audio.removeAttribute('src');
        if (objectUrl) {
          URL.revokeObjectURL(objectUrl);
          objectUrl = null;
        }
        loadedFlag = false;
      },
    };
    players.set(index, player);
    return player;
  }

  function disarm() {
    if (armedBtn) {
      const action = armedBtn.dataset.action || 'copy';
      armedBtn.innerHTML = action === 'reprocess' ? '↻' : actionIcon(action);
      armedBtn.classList.remove('confirm-armed');
    }
    if (armedTimer) {
      clearTimeout(armedTimer);
      armedTimer = null;
    }
    armedBtn = null;
    armedAction = null;
  }

  function arm(btn: HTMLButtonElement, action: string, confirmLabel: string) {
    disarm();
    armedBtn = btn;
    armedAction = action;
    btn.classList.add('confirm-armed');
    btn.textContent = confirmLabel;
    armedTimer = window.setTimeout(disarm, 3000);
  }

  const playEntry = async (index: number, btn: HTMLButtonElement) => {
    const wrap = btn.closest('.history-audio-player') as HTMLDivElement | null;
    if (!wrap) return;
    const player = getPlayer(index, wrap);
    try {
      await player.ensureLoaded();
      if (activePlayer && activePlayer.audio !== player.audio) stopActive();
      if (player.audio.paused) {
        activePlayer = player;
        await player.audio.play();
        player.setPlaying(true);
      } else {
        player.audio.pause();
        player.setPlaying(false);
        if (activePlayer && activePlayer.audio === player.audio) activePlayer = null;
      }
    } catch (e) {
      console.error('Failed to play history audio:', e);
      player.reset();
    }
  };

  const stopActive = () => {
    if (!activePlayer) return;
    activePlayer.audio.pause();
    activePlayer.reset();
    activePlayer = null;
  };

  async function handleAction(action: string, index: number, btn: HTMLButtonElement, entry: HistoryItem) {
    if (armedBtn && armedBtn !== btn) disarm();

    switch (action) {
      case 'copy':
        try {
          await navigator.clipboard.writeText(entry.text);
          btn.innerHTML = COPIED_ICON;
          copiedIndex[index] = true;
          window.setTimeout(() => {
            btn.innerHTML = COPY_ICON;
            copiedIndex[index] = false;
          }, 1200);
        } catch (_) {}
        break;
      case 'play':
        playEntry(index, btn);
        break;
      case 'reprocess':
        if (reprocessing[index]) break;
        reprocessing[index] = true;
        try {
          await reprocessHistoryEntry(index);
          await load();
        } catch (e) {
          console.error('Failed to reprocess:', e);
        } finally {
          reprocessing[index] = false;
        }
        break;
      case 'delete-audio':
        if (armedBtn === btn) {
          disarm();
          stopActive();
          try {
            await deleteHistoryAudio(index);
            await load();
          } catch (e) {
            console.error(e);
          }
        } else {
          arm(btn, action, 'Sure?');
        }
        break;
      case 'delete':
        if (armedBtn === btn) {
          disarm();
          stopActive();
          try {
            await deleteHistoryEntry(index);
            await load();
          } catch (e) {
            console.error(e);
          }
        } else {
          arm(btn, action, 'Delete?');
        }
        break;
    }
  }

  async function onClearClick(btn: HTMLButtonElement) {
    disarm();
    if (!clearArmed) {
      clearArmed = true;
      clearEl = btn;
      btn.textContent = 'Clear all?';
      btn.classList.add('confirm-armed');
      clearTimer = window.setTimeout(() => resetClear(btn), 3000);
      return;
    }
    resetClear(btn);
    stopActive();
    try {
      await clearHistory();
      await load();
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
    btn.textContent = 'Clear All History';
    btn.classList.remove('confirm-armed');
  }

  function ensureObserver(): IntersectionObserver {
    if (observer) return observer;
    observer = new IntersectionObserver(
      (records) => {
        if (records.some((r) => r.isIntersecting)) renderPage();
      },
      { root: null, rootMargin: '200px' },
    );
    return observer;
  }

  function renderPage() {
    if (!sentinelEl) return;
    const prev = renderedCount;
    const end = Math.min(prev + PAGE_SIZE, entries.length);
    renderedCount = end;
    ensureObserver().unobserve(sentinelEl);
    if (end >= entries.length) {
      sentinelEl.hidden = true;
    } else {
      ensureObserver().observe(sentinelEl);
    }
  }

  async function load() {
    let items: HistoryItem[] = [];
    try {
      const history = (await getHistory()) as { entries?: HistoryItem[] } | HistoryItem[];
      items = Array.isArray(history) ? history : (history?.entries || []);
    } catch (e) {
      console.error('Failed to load history:', e);
      return;
    }
    releasePlayers();
    for (const k in reprocessing) reprocessing[k] = false;
    entries = items;
    empty = items.length === 0;
    renderedCount = 0;
    loaded = true;
    if (!empty) requestAnimationFrame(() => {
      renderedCount = Math.min(PAGE_SIZE, items.length);
      if (sentinelEl) {
        if (renderedCount < items.length) {
          sentinelEl.hidden = false;
          ensureObserver().observe(sentinelEl);
        } else {
          sentinelEl.hidden = true;
        }
      }
    });
  }

  function releasePlayers() {
    stopActive();
    players.forEach((p) => p.release());
    players.clear();
  }

  // Refresh when the tab becomes active.
  let prevActive = false;
  $effect(() => {
    if (active && !prevActive) load();
    prevActive = active;
  });
  $effect(() => {
    if (!active && loaded) releasePlayers();
  });

  export function refresh() {
    return load();
  }

  export async function clearAll() {
    disarm();
    stopActive();
    try {
      await clearHistory();
      await load();
    } catch (e) {
      console.error(e);
    }
  }
</script>

{#if !loaded}
  <div class="history-empty" id="history-empty"></div>
{:else if empty}
  <div class="history-empty" id="history-empty">No transcriptions yet</div>
{:else}
  <div class="history-list" id="history-list" bind:this={listRoot}>
    {#each entries.slice(0, renderedCount) as entry, index}
      <div class="history-entry" data-index={index}>
        <div class="history-header">
          <span class="history-time">{formatTimestamp(entry.timestamp)}</span>
          {#if entry.audio_file}
            <span class="history-audio-tag" title="Audio saved">
              {entry.duration_secs ? formatDuration(Number(entry.duration_secs)) : 'audio'}
            </span>
          {/if}
          <span class="history-entry-actions">
            <button
              type="button"
              class="history-action-btn"
              title="Copy"
              onclick={(e) => handleAction('copy', index, e.currentTarget as HTMLButtonElement, entry)}
            >{@html COPY_ICON}</button>
            {#if entry.audio_file}
              <button
                type="button"
                class="history-action-btn"
                title="Reprocess audio"
                onclick={(e) => handleAction('reprocess', index, e.currentTarget as HTMLButtonElement, entry)}
              >{reprocessing[index] ? '…' : '↻'}</button>
              <button
                type="button"
                class="history-action-btn delete-audio"
                title="Delete audio only (keep text)"
                onclick={(e) => handleAction('delete-audio', index, e.currentTarget as HTMLButtonElement, entry)}
              >♪⌫</button>
            {/if}
            <button
              type="button"
              class="history-action-btn delete"
              title="Delete entry and audio"
              onclick={(e) => handleAction('delete', index, e.currentTarget as HTMLButtonElement, entry)}
            >{@html DELETE_ICON}</button>
          </span>
        </div>
        <div class="history-text">{entry.text}</div>
        {#if entry.audio_file}
          <div class="history-audio-player" data-duration-hint={entry.duration_secs ? String(entry.duration_secs) : undefined}>
            <button
              type="button"
              class="history-play-btn"
              title="Play recording"
              onclick={(e) => handleAction('play', index, e.currentTarget as HTMLButtonElement, entry)}
            >▶</button>
            <div class="history-seekbar">
              <input type="range" class="history-seek" min="0" max="1000" value="0" disabled aria-label="Playback position">
            </div>
            <span class="history-audio-time">
              <span class="history-time-cur">0:00</span>
              <span class="history-time-total"> / {entry.duration_secs ? formatDuration(Number(entry.duration_secs)) : '0:00'}</span>
            </span>
            <audio preload="metadata"></audio>
          </div>
        {/if}
      </div>
    {/each}
    <div class="history-sentinel" aria-hidden="true" bind:this={sentinelEl} hidden></div>
  </div>
{/if}
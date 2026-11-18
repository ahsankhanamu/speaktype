<script lang="ts">
  import {
    getScribeSessions,
    startScribeSession,
    stopScribeSession,
    setScribeEnabled,
    deleteScribeSession,
    getScribeSessionEvents,
    captureScribeScreenshot,
    scribeSessionImagePath,
    scribePermissions,
    scribeRequestScreenAccess,
    openSystemPane,
    generateScribeDocument,
    exportScribeSessionAll,
    revealPath,
    searchScribeSessions,
    getSettings,
    saveSettings,
    copyToClipboard,
    validateScribeLlmSettings,
    startScribeLlmLocal,
    stopScribeLlmLocal,
    getScribeLlmLocalStatus,
    type ScribeSession,
    type ScribeEvent,
  } from '../lib/ipc';
  import { listenEvent } from '../lib/ipc';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { normalizeHotkey, isAcceptedHotkey, formatHotkey } from '../lib/format';
  import StSelect from '../lib/StSelect.svelte';
  import Button from '../lib/ui/Button.svelte';
  import { onMount } from 'svelte';

  let { active = false }: { active?: boolean } = $props();

  const TEMPLATE_OPTIONS = [
    { value: 'auto', label: 'Auto (detect from content)' },
    { value: 'general', label: 'General' },
    { value: 'thesis', label: 'Thesis' },
    { value: 'qa_test_plan', label: 'QA Test Plan' },
    { value: 'blog', label: 'Blog Post' },
    { value: 'meeting_notes', label: 'Meeting Notes' },
  ];

  let sessions: ScribeSession[] = $state([]);
  let loaded = $state(false);
  let loading = $state(false);
  let error = $state('');
  let statusMsg = $state('Scribe captures your dictation, copied text, and screenshots into a session — then assembles them into a document.');
  let busy = $state(false);
  let screenRecording = $state<boolean | null>(null);

  let activeSessionId: number | null = $state(null);
  let newSessionName = $state('');
  let newTemplate = $state('auto');

  let expandedId: number | null = $state(null);
  let events: ScribeEvent[] = $state([]);
  let imgSrc = $state<Record<number, string>>({});
  let docMarkdown = $state('');
  let docOpen = $state(false);
  let query = $state('');
  let searchHits: ScribeEvent[] = $state([]);
  let searching = $state(false);

  let clipboardMonitor = $state(true);
  let templateDefault = $state('auto');
  let screenshotHotkey = $state('Command+Option+S');
  let hotkeyError = $state('');
  let llmBaseUrl = $state('');
  let llmApiKey = $state('');
  let llmModel = $state('');
  let llmValid = $state<{ ok: boolean; error?: string } | null>(null);
  let llmUseLocal = $state(false);
  let llmLocalModel = $state('');
  let llmLocalCtx = $state(4096);
  let llmServerStatus = $state<{ running: boolean; port: number | null; base_url: string | null } | null>(null);
  let llmServerBusy = $state(false);
  let maxScreenshots = $state(50);
  let liveScreenshots = $state(0);
  let scribeEnabled = $state(false);

  let armedBtn: HTMLButtonElement | null = $state(null);
  let armedTimer: number | null = null;

  let unlisteners: Array<() => void> = [];

  function fmt(ts: string): string {
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
  }

  function eventLabel(t: string): string {
    switch (t) {
      case 'transcript': return 'Dictation';
      case 'screenshot': return 'Screenshot';
      case 'clipboard': return 'Copied';
      default: return t;
    }
  }

  function metadataDims(meta: string | null): string {
    if (!meta) return '';
    try {
      const v = JSON.parse(meta);
      return v.dimensions ? String(v.dimensions) : '';
    } catch {
      return '';
    }
  }

  function renderDoc(md: string): string {
    const lines = md.split('\n');
    const html: string[] = [];
    let bulletOpen = false;
    let orderedOpen = false;
    const escape = (s: string) =>
      s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    const isImageHref = (u: string) => /\.(png|jpe?g|gif|webp)(\?|$)/i.test(u.split(/[#?]/)[0]);
    const toSrc = (href: string) =>
      /^(\/|\/\/)/.test(href) || /^[A-Za-z]:[\\/]/.test(href) ? convertFileSrc(href) : href;
    const inline = (s: string) =>
      s
        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.+?)\*/g, '<em>$1</em>')
        .replace(/`([^`]+)`/g, '<code>$1</code>')
        .replace(/\[([^\]]+)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g, (_m, t, u) =>
          u.startsWith('#') || u.includes('://') ? `<a href="${u}" target="_blank" rel="noreferrer">${escape(t)}</a>` : `<strong>${escape(t)}</strong>`
        );
    const close = (depth: number) => {
      if (bulletOpen) { html.push('</ul>'); bulletOpen = false; }
      if (orderedOpen) { html.push('</ol>'); orderedOpen = false; }
    };
    let inCode = false;
    for (const raw of lines) {
      const trimmed = raw.trim();
      if (trimmed.startsWith('```')) {
        close(1);
        if (!inCode) { html.push('<pre><code>'); inCode = true; } else { html.push('</code></pre>'); inCode = false; }
        continue;
      }
      if (inCode) { html.push(escape(raw)); continue; }
      if (trimmed.startsWith('#')) {
        close(1);
        const m = trimmed.match(/^(#{1,6})\s+(.*)$/);
        if (m) html.push(`<h${m[1].length}>${inline(escape(m[2]))}</h${m[1].length}>`);
        continue;
      }
      if (trimmed === '---' || trimmed === '***') {
        close(1);
        html.push('<hr>');
        continue;
      }
      if (trimmed.startsWith('|')) {
        close(1);
        const cells = trimmed.split('|').filter((c) => c.trim());
        if (cells.some((c) => /^:?-{3,}:?$/.test(c.trim()))) continue;
        html.push(`<table><tr>${cells.map((c) => `<td>${inline(escape(c.trim()))}</td>`).join('')}</tr></table>`);
        continue;
      }
      if (trimmed.startsWith('> ')) {
        close(1);
        html.push(`<blockquote>${inline(escape(trimmed.slice(2)))}</blockquote>`);
        continue;
      }
      const bm = trimmed.match(/^[-*]\s+(.+)$/);
      if (bm) {
        if (!bulletOpen) { html.push('<ul>'); bulletOpen = true; }
        html.push(`<li>${inline(escape(bm[1]))}</li>`);
        continue;
      }
      const om = trimmed.match(/^\d+\.\s+(.+)$/);
      if (om) {
        if (!orderedOpen) { html.push('<ol>'); orderedOpen = true; }
        html.push(`<li>${inline(escape(om[1]))}</li>`);
        continue;
      }
      const im = trimmed.match(/^!\[([^\]]*)\]\(([^)\s]+)/);
      if (im && isImageHref(im[2])) {
        close(1);
        html.push(`<img class="doc-img" src="${toSrc(im[2])}" alt="${escape(im[1])}" />`);
        continue;
      }
      if (trimmed === '') {
        close(1);
        continue;
      }
      close(1);
      html.push(`<p>${inline(escape(trimmed))}</p>`);
    }
    if (inCode) html.push('</code></pre>');
    close(1);
    return html.join('\n');
  }

  async function resolveScreenshots() {
    if (expandedId === null) return;
    const shots = events.filter((e) => e.event_type === 'screenshot' && e.image_path);
    const next: Record<number, string> = {};
    await Promise.all(
      shots.map(async (e) => {
        try {
          const abs = await scribeSessionImagePath(expandedId!, e.image_path!);
          next[e.id] = convertFileSrc(abs);
        } catch {
          // leave missing; treat as no thumb.
        }
      })
    );
    imgSrc = next;
  }

  async function refreshPermissions() {
    try {
      const p = await scribePermissions();
      screenRecording = !!p.screen_recording;
    } catch {
      screenRecording = null;
    }
  }

  async function requestScreenAccess() {
    llmServerBusy = true;
    try {
      const r = await scribeRequestScreenAccess();
      screenRecording = r.granted;
      // macOS's own prompt only appears the first time; if it was granted we can
      // use screenshots right away, otherwise route to System Settings.
      if (!r.granted && r.denied) {
        openSystemPane('com.apple.preference.security?Privacy_ScreenCapture');
        statusMsg = 'Grant Screen Recording for SpeakType in System Settings, then relaunch to enable screenshots.';
      } else if (r.granted) {
        statusMsg = 'Screen Recording granted — screenshots enabled.';
      }
      await refreshPermissions();
    } catch {
      statusMsg = 'Could not request Screen Recording permission.';
    } finally {
      llmServerBusy = false;
    }
  }

  function requestScreenAccessManually() {
    openSystemPane('com.apple.preference.security?Privacy_ScreenCapture');
    statusMsg = 'Grant Screen Recording for SpeakType in System Settings, then relaunch to enable screenshots.';
  }

  async function loadSettings() {
    try {
      const s = await getSettings();
      clipboardMonitor = s.scribe_clipboard_monitor !== false;
      scribeEnabled = s.scribe_enabled === true;
      templateDefault = s.scribe_template || 'auto';
      newTemplate = templateDefault;
      screenshotHotkey = formatHotkey(s.scribe_screenshot_hotkey || 'Super+Option+S');
      llmBaseUrl = s.scribe_llm_base_url || '';
      llmApiKey = s.scribe_llm_api_key || '';
      llmModel = s.scribe_llm_model || '';
      llmUseLocal = s.scribe_llm_use_local || false;
      llmLocalModel = s.scribe_llm_local_model || '';
      llmLocalCtx = s.scribe_llm_local_ctx || 4096;
      refreshLlmStatus();
      maxScreenshots = s.scribe_max_screenshots || 50;
      llmValid = await validateScribeLlmSettings().catch(() => null);
    } catch (e) {
      console.error('[scribe] load settings:', e);
    }
  }

  async function saveScribeSettings() {
    try {
      const s = await getSettings();
      s.scribe_enabled = scribeEnabled;
      s.scribe_clipboard_monitor = clipboardMonitor;
      s.scribe_template = templateDefault;
      s.scribe_screenshot_hotkey = normalizeHotkey(screenshotHotkey.trim());
      s.scribe_max_screenshots = Math.max(1, Number(maxScreenshots) || 50);
      s.scribe_llm_base_url = llmBaseUrl.trim();
      s.scribe_llm_api_key = llmApiKey.trim();
      s.scribe_llm_model = llmModel.trim();
      s.scribe_llm_use_local = llmUseLocal;
      s.scribe_llm_local_model = llmLocalModel.trim();
      s.scribe_llm_local_ctx = Math.max(256, Number(llmLocalCtx) || 4096);
      await saveSettings(s);
      statusMsg = 'Scribe settings saved.';
      llmValid = await validateScribeLlmSettings().catch(() => null);
    } catch (e) {
      statusMsg = 'Failed to save settings: ' + e;
    }
  }

  export async function saveAll() {
    await saveScribeSettings();
  }

  async function refreshLlmStatus() {
    llmServerStatus = await getScribeLlmLocalStatus().catch(() => null);
  }

  async function startLocalServer() {
    llmServerBusy = true;
    try {
      await startScribeLlmLocal();
      llmServerStatus = await getScribeLlmLocalStatus().catch(() => null);
      statusMsg = llmServerStatus?.running
        ? 'Local llama.cpp server ready on ' + (llmServerStatus.base_url ?? 'unknown port')
        : 'Local LLM started but is not ready yet.';
    } catch (e) {
      statusMsg = 'Failed to start local LLM: ' + e;
      llmServerStatus = (await getScribeLlmLocalStatus().catch(() => null)) ?? null;
    } finally {
      llmServerBusy = false;
    }
  }

  async function stopLocalServer() {
    llmServerBusy = true;
    try {
      await stopScribeLlmLocal();
      llmServerStatus = { running: false, port: null, base_url: null };
      statusMsg = 'Local llama.cpp server stopped.';
    } catch (e) {
      statusMsg = 'Failed to stop local LLM: ' + e;
    } finally {
      llmServerBusy = false;
    }
  }

  export async function refresh() {
    await Promise.all([refreshSessions(), refreshPermissions(), loadSettings()]);
  }

  async function refreshSessions() {
    if (loading) return;
    loading = true;
    error = '';
    try {
      sessions = await getScribeSessions();
      loaded = true;
      if (sessions.some((s) => !s.ended_at)) {
        activeSessionId = sessions.find((s) => !s.ended_at)?.id ?? null;
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function recountScreenshots() {
    if (activeSessionId === null) {
      liveScreenshots = 0;
      return;
    }
    try {
      const evs = await getScribeSessionEvents(activeSessionId);
      liveScreenshots = evs.filter((e) => e.event_type === 'screenshot').length;
    } catch {
      liveScreenshots = 0;
    }
  }

  async function startSession() {
    let name = newSessionName.trim();
    if (!name) {
      const now = new Date();
      name =
        'Scribe session ' +
        now.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) +
        ' ' +
        now.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    }
    if (busy) return;
    busy = true;
    error = '';
    try {
      const res = await startScribeSession(name, newTemplate);
      activeSessionId = res.session.id;
      statusMsg = `Session "${res.session.name}" running — dictate, copy, and hit ${formatHotkey(screenshotHotkey) || 'Command+Option+S'} to snapshot a region or window.`;
      newSessionName = '';
      await refresh();
      await recountScreenshots();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function stopSession(id: number) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await stopScribeSession(id);
      if (activeSessionId === id) {
        activeSessionId = null;
        liveScreenshots = 0;
      }
      statusMsg = 'Session stopped — generate a document from it below.';
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function toggleMode() {
    const next = !scribeEnabled;
    try {
      await setScribeEnabled(next);
      scribeEnabled = next;
    } catch (e) {
      error = String(e);
      return;
    }
    await refresh();
    if (scribeEnabled) {
      statusMsg = 'Scribe mode on — dictate, copy, or snap; the first capture starts the session.';
    } else {
      activeSessionId = null;
      liveScreenshots = 0;
      statusMsg = 'Scribe mode off — any live session was stopped.';
    }
  }

  async function snapScreenshot() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const ev = await captureScribeScreenshot();
      statusMsg = 'Screenshot captured and OCR’d into the active session.';
      await recountScreenshots();
      if (expandedId !== null) await expand(expandedId);
      if (liveScreenshots >= maxScreenshots) {
        statusMsg = `Screenshot ${ev.id} captured — session is at the ${maxScreenshots}-screenshot cap.`;
      }
    } catch (e) {
      error = String(e);
      if (String(e).includes('Screen Recording') || String(e).includes('permission')) {
        screenRecording = false;
      }
    } finally {
      busy = false;
    }
  }

  async function expand(id: number) {
    expandedId = expandedId === id ? null : id;
    events = [];
    imgSrc = {};
    docOpen = false;
    if (expandedId === null) return;
    try {
      events = await getScribeSessionEvents(id);
      await resolveScreenshots();
    } catch (e) {
      error = String(e);
    }
  }

  function disarm() {
    if (armedBtn) {
      armedBtn.textContent = 'Delete';
      armedBtn.classList.remove('confirm-armed');
    }
    if (armedTimer) {
      clearTimeout(armedTimer);
      armedTimer = null;
    }
    armedBtn = null;
  }

  function arm(btn: HTMLButtonElement) {
    disarm();
    armedBtn = btn;
    btn.classList.add('confirm-armed');
    btn.textContent = 'Delete?';
    armedTimer = window.setTimeout(disarm, 3000);
  }

  async function removeSession(id: number, btn: HTMLButtonElement) {
    if (armedBtn && armedBtn !== btn) disarm();
    if (armedBtn === btn) {
      disarm();
      try {
        await deleteScribeSession(id);
        if (expandedId === id) expandedId = null;
        if (activeSessionId === id) activeSessionId = null;
        await refresh();
      } catch (e) {
        error = String(e);
      }
    } else {
      arm(btn);
    }
  }

  async function generateDoc(id: number) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      docMarkdown = await generateScribeDocument(id);
      docOpen = true;
      statusMsg = 'Document generated.';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function copyDoc() {
    try {
      await copyToClipboard(docMarkdown);
      statusMsg = 'Generated document copied to the clipboard.';
    } catch (e) {
      error = String(e);
    }
  }

  async function exportDocs(id: number) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const paths = await exportScribeSessionAll(id);
      statusMsg = `Exported .md and .docx to ${paths[0].split('/').slice(0, -1).join('/')}`;
      const first = paths[0];
      if (first) revealPath(first).catch((e: unknown) => { error = String(e); });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function doSearch() {
    if (!query.trim()) {
      searchHits = [];
      return;
    }
    searching = true;
    try {
      searchHits = await searchScribeSessions(query);
    } catch (e) {
      error = String(e);
    } finally {
      searching = false;
    }
  }

  function clearSearch() {
    query = '';
    searchHits = [];
    searching = false;
  }

  onMount(() => {
    refresh().then(() => {
      // Auto-ask once when the tab opens: if the grant is undetermined the OS
      // shows its own prompt; if it is already denied we just show the banner.
      if (screenRecording === false) requestScreenAccess();
    });

    const un = (ev: string, fn: (payload: unknown) => void) =>
      listenEvent(ev, fn).then((u) => unlisteners.push(u));
    un('scribe:event_added', (_payload) => {
      if (expandedId !== null) getScribeSessionEvents(expandedId).then((evs) => { events = evs; });
      if (activeSessionId !== null) recountScreenshots();
      refresh();
    });
    un('scribe:session_started', () => refresh());
    un('scribe:session_stopped', () => refresh());
    un('scribe:llm_server:crash', (payload) => {
      const p = (payload ?? {}) as { reason?: string; code?: number; crashes?: number };
      llmServerStatus = { running: false, port: null, base_url: null };
      statusMsg = `Local llama.cpp server stopped unexpectedly (${p.crashes ?? 1} crash${(p.crashes ?? 1) === 1 ? '' : 's'})${p.code != null ? `, code ${p.code}` : ''}.`;
    });
    un('scribe:llm_server:error', (_payload) => refreshLlmStatus());

    return () => {
      unlisteners.forEach((u) => u());
      unlisteners = [];
    };
  });
</script>

{#if loaded}
  <section class="card scribe-card">
    <div class="scribe-actions">
      <input
        type="text"
        class="sess-name"
        id="scribe-session-name"
        placeholder="Session name (e.g. Thesis research)"
        bind:value={newSessionName}
        onkeydown={(e) => { if (e.key === 'Enter') startSession(); }}
      />
      <StSelect
        bind:value={newTemplate}
        options={TEMPLATE_OPTIONS}
        ariaLabel="Document template"
        placeholder="Auto"
      />
      <div class="scribe-switch-wrap">
        <button
          class="settings-switch"
          id={scribeEnabled ? 'scribe-on-switch' : 'scribe-off-switch'}
          class:on={scribeEnabled}
          role="switch"
          aria-checked={scribeEnabled}
          title={scribeEnabled ? 'Turn scribe mode off' : 'Turn scribe mode on'}
          onclick={toggleMode}
          disabled={busy}
        >
          <span class="settings-switch-knob"></span>
        </button>
        <span class="scribe-switch-state">
          {scribeEnabled ? 'Scribe on' : 'Scribe off'}
        </span>
      </div>
    </div>
    {#if error}
      <p class="scribe-error" role="status">{error}</p>
    {/if}
    <div class="scribe-active-row">
      <span class="scribe-dot" class:live={activeSessionId !== null}></span>
      <span class="scribe-activestate">
        {activeSessionId !== null ? 'Session live — dictation and copies are being captured' : 'No active session'}
      </span>
      {#if scribeEnabled}
        {#if screenRecording === false}
          <span class="scribe-perm-warn" title="Grant Screen Recording in System Settings to enable screenshots">no screenshot permission</span>
        {:else}
          <Button size="small" variant="secondary" id="scribe-snap-btn" onclick={snapScreenshot} disabled={busy || liveScreenshots >= maxScreenshots}>Screenshot ({formatHotkey(screenshotHotkey, { compact: true })})</Button>
        {/if}
      {/if}
    </div>
    {#if liveScreenshots >= maxScreenshots}
      <p class="hint scribe-cap-warn" id="scribe-cap-warn">Screenshot limit reached ({liveScreenshots}/{maxScreenshots}) — adjust it in Scribe settings below.</p>
    {/if}
    <p class="hint" id="scribe-status">{statusMsg}</p>
  </section>

  {#if screenRecording === false}
    <section class="card">
      <p class="hint">
        Screenshot capture needs <strong>Screen Recording</strong> permission.
        SpeakType fully auto-detects content for dictation and clipboard; this
        permission is only for screenshots.
      </p>
      <div class="scribe-actions">
        <Button size="small" variant="secondary" id="scribe-req-screen-btn" onclick={requestScreenAccess} disabled={llmServerBusy}>{llmServerBusy ? 'Requesting…' : 'Request Screen Recording'}</Button>
        <Button size="small" variant="secondary" id="scribe-open-screen-btn" onclick={requestScreenAccessManually}>Open System Settings</Button>
      </div>
      <p class="hint">Grant it in System Settings, then <strong>Quit &amp; Relaunch</strong>.</p>
    </section>
  {/if}

  <section class="card">
    <h2 class="card-title">Sessions</h2>
    <div class="scribe-actions">
      <input
        type="text"
        class="sess-name"
        id="scribe-search"
        placeholder="Search transcripts, screenshots’ OCR, copies…"
        bind:value={query}
        onkeydown={(e) => { if (e.key === 'Enter') doSearch(); }}
      />
      <Button variant="secondary" id="scribe-search-btn" onclick={doSearch} disabled={searching || !query.trim()}>Search</Button>
      {#if query.trim() && !searching}
        <Button size="small" variant="secondary" id="scribe-search-clear" onclick={clearSearch}>Clear</Button>
      {/if}
    </div>
    {#if searching}
      <p class="hint">Searching…</p>
    {:else if query.trim()}
      {#if searchHits.length > 0}
        <ul class="event-list">
          {#each searchHits as hit (hit.id)}
            <li class="event-row">
              <span class="event-tag" class:tran={hit.event_type === 'transcript'} class:shot={hit.event_type === 'screenshot'} class:clip={hit.event_type === 'clipboard'}>{eventLabel(hit.event_type)}</span>
              <span class="event-time">#{hit.session_id} · {fmt(hit.timestamp)}</span>
              <p class="event-content">{hit.content}</p>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="hint">No matches.</p>
      {/if}
    {:else if sessions.length === 0}
      <p class="hint">No sessions yet. Start one above.</p>
    {:else}
      <ul class="session-list" id="scribe-session-list">
        {#each sessions as s (s.id)}
          <li class="session-row" class:live={!s.ended_at}>
            <div class="session-main" role="button" tabindex="0" onclick={() => expand(s.id)} onkeydown={(e) => { if (e.key === 'Enter') expand(s.id); }}>
              <span class="session-name">{s.name}</span>
              <span class="session-meta">
                {s.ended_at ? fmt(s.ended_at) : '● live'}
                · {TEMPLATE_OPTIONS.find((t) => t.value === s.template)?.label ?? s.template}
              </span>
            </div>
            <div class="session-actions">
              <Button size="small" variant="secondary" id="gen-{s.id}" title="Generate document" onclick={() => generateDoc(s.id)} disabled={busy}>Generate</Button>
              <Button size="small" variant="secondary" id="export-all-{s.id}" title="Export .md and .docx to Documents/SpeakType/Scribe, then reveal in Finder" onclick={() => exportDocs(s.id)} disabled={busy}>.md + .docx</Button>
              <Button size="small" variant="danger" title="Delete session" onclick={(e) => removeSession(s.id, e.currentTarget)}>Delete</Button>
            </div>
          </li>

          {#if expandedId === s.id}
            <li class="session-detail">
              {#if events.length === 0}
                <p class="hint">No captures yet.</p>
              {:else}
                <ul class="event-list">
                  {#each events as ev (ev.id)}
                    <li class="event-row">
                      <span class="event-tag" class:tran={ev.event_type === 'transcript'} class:shot={ev.event_type === 'screenshot'} class:clip={ev.event_type === 'clipboard'}>{eventLabel(ev.event_type)}</span>
                      <span class="event-time">{fmt(ev.timestamp)}</span>
                      {#if ev.source_app}<span class="event-app">in {ev.source_app}</span>{/if}
                      {#if ev.content}<p class="event-content">{ev.content}</p>{/if}
                      {#if ev.event_type === 'screenshot'}{#if imgSrc[ev.id]}<img class="event-thumb" src={imgSrc[ev.id]} alt="screenshot" loading="lazy" />{/if}{/if}
                      {#if ev.event_type === 'screenshot'}{#if metadataDims(ev.metadata)}<p class="event-dim">{metadataDims(ev.metadata)}</p>{/if}{/if}
                    </li>
                  {/each}
                </ul>
              {/if}
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
  </section>

  {#if docOpen}
    <section class="card">
      <h2 class="card-title">Generated document</h2>
      <div class="doc-preview doc-html">
        {@html renderDoc(docMarkdown)}
      </div>
      <div class="doc-open-closer">
        <Button size="small" variant="secondary" id="scribe-doc-copy" onclick={copyDoc}>Copy</Button>
        <Button size="small" variant="secondary" onclick={() => (docOpen = false)}>Close</Button>
      </div>
    </section>
  {/if}
{:else if loading}
  <p class="hint">Loading sessions…</p>
{:else}
  <p class="hint">Scribe database unavailable: {error || 'unknown error'}</p>
{/if}

<section class="card">
  <h2 class="card-title">Scribe settings</h2>
  <div class="field">
    <span class="field-label">Document template</span>
    <StSelect
      bind:value={templateDefault}
      options={TEMPLATE_OPTIONS}
      ariaLabel="Default template"
      placeholder="Auto"
    />
    <p class="hint">Content type for generated documents. "Auto" detects it from the session name and captures.</p>
  </div>

  <div class="field">
    <label class="checkbox-row" for="scribe-clipboard-toggle">
      <input type="checkbox" id="scribe-clipboard-toggle" bind:checked={clipboardMonitor} />
      <span>Capture copied text into the active session</span>
    </label>
    <p class="hint">Your own dictation paste is filtered out automatically.</p>
  </div>

  <div class="field">
    <span class="field-label">Screenshot hotkey</span>
    <input
      type="text"
      class="sess-name"
      id="scribe-hotkey"
      value={screenshotHotkey}
      oninput={(e) => {
        screenshotHotkey = e.currentTarget.value;
        const parts = normalizeHotkey(screenshotHotkey.trim()).split('+');
        hotkeyError = screenshotHotkey.trim() && !isAcceptedHotkey(parts) ? 'Use modifiers + a key, e.g. Command+Option+S' : '';
      }}
    />
    <p class="hint">Press {formatHotkey(screenshotHotkey) || 'Command+Option+S'} while a session is live to pick a region or window, then it’s snapshotted + OCR’d into the session.</p>
    {#if hotkeyError}<p class="scribe-error">{hotkeyError}</p>{/if}
  </div>

  <div class="field">
    <span class="field-label">Screenshot cap</span>
    <input type="number" class="sess-name" id="scribe-max-shots" min="1" bind:value={maxScreenshots} />
    <p class="hint">Max screenshots per session before captures are refused.</p>
  </div>

  <div class="field llm-fields">
    <span class="field-label">Document synth (optional LLM)</span>
    <input type="text" class="sess-name" id="scribe-llm-base" placeholder="Base URL, e.g. http://127.0.0.1:8000/v1" bind:value={llmBaseUrl} />
    <input type="password" class="sess-name" id="scribe-llm-key" placeholder="API key (leave empty to skip)" bind:value={llmApiKey} />
    <input type="text" class="sess-name" id="scribe-llm-model" placeholder="Model (default gpt-4o-mini)" bind:value={llmModel} />
    <p class="hint">
      Leave the base URL empty to export the plain chronological session render.
      Set an OpenAI-compatible endpoint to restructure captures into a polished document.
      {#if llmValid && llmValid.ok === false}<span class="scribe-error">{llmValid.error}</span>{/if}
    </p>
  </div>

  <div class="field llm-fields">
    <span class="field-label">Local llama.cpp (llama-server)</span>
    <label class="checkbox-row" for="scribe-llm-local-toggle">
      <input type="checkbox" id="scribe-llm-local-toggle" bind:checked={llmUseLocal} />
      Use the local llama.cpp server instead of the API above
    </label>
    <input type="text" class="sess-name" id="scribe-llm-local-model" placeholder="Path to GGUF model, e.g. ~/models/qwen2.5-7b-instruct-q4_k_m.gguf" bind:value={llmLocalModel} />
    <input type="number" class="sess-name" id="scribe-llm-local-ctx" min="256" step="512" placeholder="Context size (default 4096)" bind:value={llmLocalCtx} />
    <div class="scribe-actions">
      <Button variant="secondary" id="scribe-llm-local-start" onclick={startLocalServer} disabled={llmServerBusy || !llmLocalModel.trim()}>Start server</Button>
      <Button variant="secondary" id="scribe-llm-local-stop" onclick={stopLocalServer} disabled={llmServerBusy}>Stop server</Button>
      {#if llmServerStatus}
        <p class="hint" id="scribe-llm-local-status">
          {llmServerStatus.running
            ? `Running — ${llmServerStatus.base_url ?? 'unknown port'}`
            : 'Stopped'}
        </p>
      {/if}
    </div>
    <p class="hint">
      Requires the llama.cpp <code>llama-server</code> binary on your PATH (e.g. <code>brew install llama.cpp</code>).
      Generation will start it automatically when the model path is set.
    </p>
  </div>

  <div class="scribe-actions">
    <Button variant="primary" id="scribe-settings-save-card" onclick={saveScribeSettings}>Save Scribe Settings</Button>
  </div>
</section>

<style>
  .scribe-card {
    border-top: 3px solid var(--accent, #3b82f6);
  }
  .scribe-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .scribe-switch-wrap {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .settings-switch {
    position: relative;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: var(--muted, rgba(127, 127, 127, 0.25));
    cursor: pointer;
    transition: background 0.18s ease;
  }
  .settings-switch.on {
    background: var(--accent, #34c759);
  }
  .settings-switch-knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    transition: transform 0.18s ease;
  }
  .settings-switch.on .settings-switch-knob {
    transform: translateX(18px);
  }
  .scribe-switch-state {
    font-size: 13px;
    font-weight: 600;
  }
  .sess-name {
    flex: 1;
    min-width: 160px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .sess-name:focus {
    outline: none;
    border-color: var(--accent, #3b82f6);
  }
  .scribe-error {
    color: var(--danger, #ef4444);
    font-size: 12px;
    margin-top: 6px;
  }
  .scribe-active-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
  }
  .scribe-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-dim, #94a3b8);
  }
  .scribe-dot.live {
    background: #4ade80;
    box-shadow: 0 0 6px rgba(74, 222, 128, 0.8);
  }
  .scribe-activestate {
    flex: 1;
    font-size: 12px;
    color: var(--text-dim);
  }
  .scribe-perm-warn {
    font-size: 11px;
    color: var(--warning, #eab308);
  }
  .scribe-cap-warn {
    color: var(--warning, #eab308);
  }
  .session-list {
    list-style: none;
    padding: 0;
    margin: 8px 0 0;
  }
  .session-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
  }
  .session-row.live .session-name {
    color: var(--success, #4ade80);
  }
  .session-main {
    min-width: 0;
    cursor: pointer;
  }
  .session-name {
    font-weight: 600;
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .session-meta {
    font-size: 11px;
    color: var(--text-dim);
  }
  .session-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .session-detail {
    padding: 8px 4px 12px;
    border-bottom: 1px solid var(--border);
  }
  .event-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .event-row {
    padding: 6px 0;
    border-bottom: 1px dashed var(--border);
  }
  .event-row:last-child {
    border-bottom: none;
  }
  .event-tag {
    display: inline-block;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface-3);
    color: var(--text-dim);
    margin-right: 6px;
  }
  .event-tag.tran {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }
  .event-tag.shot {
    background: rgba(236, 72, 153, 0.15);
    color: #f472b6;
  }
  .event-tag.clip {
    background: rgba(250, 204, 21, 0.15);
    color: #facc15;
  }
  .event-time {
    font-size: 11px;
    color: var(--text-dim);
  }
  .event-app {
    font-size: 11px;
    color: var(--text-dim);
    margin-left: 4px;
  }
  .event-content {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .event-dim {
    font-size: 11px;
    color: var(--text-dim);
    margin: 2px 0 0;
  }
  .doc-preview {
    width: 100%;
    min-height: 220px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-1);
    color: var(--text);
    padding: 10px;
    resize: vertical;
  }
  .doc-html {
    resize: none;
    min-height: 220px;
    overflow: auto;
    font-family: var(--font, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
    white-space: normal;
  }
  .doc-html :global(h1) {
    font-size: 18px;
    margin: 4px 0 8px;
  }
  .doc-html :global(h2) {
    font-size: 15px;
    margin: 14px 0 6px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 3px;
  }
  .doc-html :global(h3),
  .doc-html :global(h4),
  .doc-html :global(h5),
  .doc-html :global(h6) {
    font-size: 13px;
    margin: 10px 0 4px;
  }
  .doc-html :global(p) {
    margin: 6px 0;
  }
  .doc-html :global(ul),
  .doc-html :global(ol) {
    margin: 6px 0 6px 18px;
    padding: 0;
  }
  .doc-html :global(li) {
    margin: 2px 0;
  }
  .doc-html :global(blockquote) {
    border-left: 3px solid var(--border);
    margin: 6px 0;
    padding: 2px 10px;
    color: var(--text-dim);
  }
  .doc-html :global(pre) {
    background: var(--surface-3);
    padding: 8px;
    border-radius: 6px;
    overflow-x: auto;
  }
  .doc-html :global(code) {
    background: var(--surface-3);
    padding: 1px 4px;
    border-radius: 4px;
    font-size: 12px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .doc-html :global(pre code) {
    background: none;
    padding: 0;
  }
  .doc-html :global(table) {
    border-collapse: collapse;
    margin: 8px 0;
  }
  .doc-html :global(th),
  .doc-html :global(td) {
    border: 1px solid var(--border);
    padding: 4px 8px;
    font-size: 12px;
  }
  .doc-html :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 10px 0;
  }
  .doc-html :global(img),
  .doc-html :global(.doc-img) {
    max-width: 100%;
    border-radius: 6px;
    border: 1px solid var(--border);
    margin: 6px 0;
  }
  .doc-open-closer {
    text-align: right;
    margin-top: 8px;
  }
  .event-thumb {
    display: block;
    max-width: 260px;
    max-height: 200px;
    margin-top: 6px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  .llm-fields input {
    margin-bottom: 6px;
  }
</style>
<script lang="ts">
  import { CircleCheck } from '@lucide/svelte';
  import { Button, formatShortcut, toast } from '@speaktype/ui';
  import ModelMenu from '../lib/app/ModelMenu.svelte';
  import { plainModel } from '../lib/app/models';
  import { runModelAction, useModels } from '../lib/app/models.svelte';
  import { openSettings, reprocessHistoryEntry } from '../lib/ipc';
  import Page from './Page.svelte';
  import Paper from './Paper.svelte';

  let { hotkey }: { hotkey: string } = $props();

  const KEY_NAMES: Record<string, string> = { '⌃': 'control', '⌥': 'option', '⇧': 'shift', '⌘': 'command' };
  const keys = $derived(formatShortcut(hotkey.split('+')));
  const GETTING_READY = 'Getting ready…';

  const models = useModels();
  const name = (id: string) => plainModel(id).name;

  let field: HTMLTextAreaElement;
  let written = $state('');
  $effect(() => field.focus());

  let original = $state<string>();
  let rerun = $state<{ id: string; text?: string }>();
  let tried = '';

  const loading = $derived(models.rows.find((row) => row.state.label === GETTING_READY)?.model.id);
  const target = $derived(rerun?.id ?? loading);

  $effect(() => {
    if (written && !original) original = models.activeId;
  });

  $effect(() => {
    const id = models.activeId;
    if (!original || id === original || id === tried) return;
    tried = id;
    compare(id);
  });

  async function compare(id: string) {
    rerun = { id };
    try {
      const { text } = (await reprocessHistoryEntry(0)) as { text: string };
      rerun = { id, text };
    } catch (error) {
      rerun = undefined;
      toast.show({ message: String(error) });
    }
  }

  function settle(keep: string) {
    if (keep !== models.activeId) runModelAction('use', keep);
    original = keep;
    rerun = undefined;
    tried = '';
  }
</script>

<Page label="Step 4 of 4" title="Try it once.">
  <p>Press these two keys, say a sentence, then press them again. Your words appear where you're typing.</p>
  <div class="keys">
    {#each keys as key (key)}
      <kbd class="key"><span class="glyph">{key}</span><span class="word">{KEY_NAMES[key] ?? ''}</span></kbd>
    {/each}
  </div>
  {#if written}
    <p class="success"><CircleCheck size={16} color="var(--st-success)" aria-hidden="true" />That worked. Press Done when you're ready.</p>
  {:else}
    <p class="caption">You can change this later in Settings.</p>
  {/if}
  <div class="model">
    <p class="caption">Speech model</p>
    <ModelMenu trigger="row" onopenproviders={() => openSettings()} />
  </div>
  {#snippet panel()}
    <Paper>
      <div class="bar"><span class="lights"><i></i><i></i><i></i></span><span class="title">Practice</span></div>
      {#if original && target}<p class="tag">{name(original)}</p>{/if}
      <textarea bind:this={field} bind:value={written} class:short={original && target} aria-label="Practice" spellcheck="false"></textarea>
      {#if original && target}
        <div class="rerun">
          <p class="tag">{name(target)}</p>
          {#if rerun?.text}
            <p class="text">{rerun.text}</p>
          {:else}
            <p class="text wait">{rerun ? 'Running it again…' : GETTING_READY}</p>
          {/if}
        </div>
      {/if}
    </Paper>
    {#if rerun?.text && original}
      <div class="verdict">
        <Button onclick={() => settle(original!)}>Back to {name(original)}</Button>
        <Button onclick={() => settle(rerun!.id)}>Keep {name(rerun.id)}</Button>
      </div>
    {/if}
  {/snippet}
</Page>

<style>
  .keys {
    display: flex;
    gap: 12px;
  }
  .key {
    position: relative;
    display: block;
    box-sizing: border-box;
    width: 75px;
    height: 72px;
    border-radius: 10px;
    background: var(--st-keycap);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.14), 0 1px 0 rgba(0, 0, 0, 0.65), 0 2px 6px rgba(0, 0, 0, 0.3);
    color: var(--st-text);
    font: inherit;
  }
  .glyph {
    position: absolute;
    top: 8px;
    right: 10px;
    font-size: 22px;
    line-height: 1;
  }
  .word {
    position: absolute;
    bottom: 8px;
    left: 10px;
    font-size: 11px;
    line-height: 1;
  }
  .success {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--st-text);
  }
  .model {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 12px;
  }
  .model p {
    margin: 0;
  }
  .bar {
    position: relative;
    display: flex;
    flex: none;
    align-items: center;
    height: 36px;
    padding: 0 14px;
    border-bottom: 0.5px solid rgba(0, 0, 0, 0.12);
    background: #f1f1ef;
  }
  .lights {
    display: flex;
    gap: 6px;
  }
  .lights i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #d4d4d2;
  }
  .title {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: #6e6e73;
    font-size: var(--st-font-info);
  }
  textarea {
    box-sizing: border-box;
    height: 440px;
    padding: 24px 28px;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 18px;
    line-height: 1.6;
    resize: none;
    outline: none;
    caret-color: var(--st-accent);
  }
  textarea.short {
    height: 190px;
    padding-top: 6px;
  }
  .tag {
    margin: 0;
    padding: 12px 28px 0;
    color: #6e6e73;
    font-size: var(--st-font-caption);
    font-weight: 500;
  }
  .rerun {
    border-top: 0.5px solid rgba(0, 0, 0, 0.12);
  }
  .text {
    margin: 0;
    padding: 6px 28px 24px;
    font-size: 18px;
    line-height: 1.6;
  }
  .wait {
    color: #6e6e73;
  }
  .verdict {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 16px;
  }
</style>

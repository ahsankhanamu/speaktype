<script lang="ts">
  import { StatusDot } from '@speaktype/ui';
  import type { ModelRow } from './models.svelte';
  import { plainModel } from './models';

  const LABELLED = new Set(['fast', 'balanced', 'accurate']);
  const COMMIT_DELAY_MS = 400;

  let {
    rows,
    index = $bindable(),
    activeId,
    oncommit,
  }: { rows: ModelRow[]; index: number; activeId: string; oncommit: (row: ModelRow) => void } = $props();

  const last = $derived(rows.length - 1);
  const selected = $derived(rows[index]);
  const active = $derived(rows.find((row) => row.model.id === activeId));
  const inUse = $derived(selected?.model.id === activeId && selected.state.action === null);

  let timer: ReturnType<typeof setTimeout> | undefined;
  function pick() {
    clearTimeout(timer);
    timer = setTimeout(() => oncommit(rows[index]), COMMIT_DELAY_MS);
  }
  $effect(() => () => clearTimeout(timer));
</script>

<div class="slider-card">
  <div class="line">
    <span class="end start">Faster</span>
    <div class="control">
      <input
        type="range"
        min="0"
        max={last}
        step="1"
        bind:value={index}
        oninput={pick}
        aria-label="Speed and accuracy"
        aria-valuetext={selected?.name}
        style:--fill={index / last}
      />
      <div class="marks" aria-hidden="true">
        {#each rows as row, i (row.model.id)}
          {#if i !== index}<span class="tick" style:left="{(i / last) * 100}%"></span>{/if}
          {#if LABELLED.has(plainModel(row.model.id).tier)}
            <span class="tag" class:current={i === index} style:left="{(i / last) * 100}%">{row.name}</span>
          {/if}
        {/each}
      </div>
    </div>
    <span class="end">More accurate</span>
  </div>

  {#if selected}
    <p class="live">
      {#if inUse}
        <StatusDot tone="success" label="In use" />
        <span>{selected.name} is in use · {plainModel(selected.model.id).size} · switching models takes a few seconds</span>
      {:else}
        <StatusDot tone={selected.state.tone === 'neutral' ? 'accent' : selected.state.tone} label={selected.state.label} />
        <span>
          {selected.name} · {plainModel(selected.model.id).size}
          {#if selected.state.action === 'pause' || selected.state.action === 'cancel'}
            · downloading now. Until it's ready, {active?.name ?? 'the current model'} is used.
          {:else}
            · {selected.state.label}
          {/if}
        </span>
      {/if}
    </p>
  {/if}
</div>

<style>
  .slider-card {
    --thumb: 26px;
    padding: 26px 30px 22px;
    border-radius: 20px;
    background: var(--st-field);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 18px;
  }
  .end {
    width: 96px;
    color: var(--st-text-secondary);
    font-size: 13px;
    text-align: right;
  }
  .end.start {
    width: 52px;
    text-align: left;
  }
  .control {
    position: relative;
    flex: 1;
    height: 22px;
  }
  input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    background: transparent;
    appearance: none;
  }
  input:focus-visible {
    outline: none;
  }
  input::-webkit-slider-runnable-track {
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(
      to right,
      var(--st-accent) calc(var(--thumb) / 2 + (100% - var(--thumb)) * var(--fill)),
      var(--st-track) 0
    );
  }
  input::-webkit-slider-thumb {
    width: var(--thumb);
    height: var(--thumb);
    margin-top: -10px;
    border-radius: 50%;
    background: #f5f5f7;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.4),
      0 0 0 0.5px rgba(0, 0, 0, 0.2);
    appearance: none;
  }
  input:focus-visible::-webkit-slider-thumb {
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.4),
      var(--st-focus-ring);
  }
  .marks {
    position: absolute;
    inset: 0 calc(var(--thumb) / 2);
    pointer-events: none;
  }
  .tick {
    position: absolute;
    top: 50%;
    width: 4px;
    height: 4px;
    border-radius: 2px;
    background: var(--st-text-tertiary);
    transform: translate(-50%, -50%);
  }
  .tag {
    position: absolute;
    top: 32px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    transform: translateX(-50%);
  }
  .tag.current {
    color: var(--st-text);
    font-weight: 500;
  }
  .live {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin: 42px 0 0;
    color: var(--st-text);
    font-size: var(--st-font-body);
    line-height: 1.4;
  }
</style>

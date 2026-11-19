<script lang="ts">
  import { Check, Cloud, Laptop, Server } from '@lucide/svelte';
  import { RadioGroup } from 'bits-ui';
  import type { ProviderConfig } from '../../lib/ipc';

  let { value = $bindable() }: { value: ProviderConfig['kind'] } = $props();

  const CARDS = [
    { kind: 'local', icon: Laptop, tile: 'linear-gradient(180deg, #8e8e93, #636366)', title: 'On this Mac', line: 'Private and offline. Uses your Dictation speech model.' },
    { kind: 'server', icon: Server, tile: 'linear-gradient(180deg, #3aa8ff, #0a84ff)', title: 'Your own server', line: 'A whisper.cpp server on your network.' },
    { kind: 'cloud', icon: Cloud, tile: 'linear-gradient(180deg, #d06cf5, #a24ae0)', title: 'Cloud service', line: 'OpenAI, Groq, Deepgram or any OpenAI-compatible API.' },
  ] as const;
</script>

<RadioGroup.Root bind:value orientation="horizontal" class="provider-cards" aria-label="Speech to text">
  {#each CARDS as card (card.kind)}
    <RadioGroup.Item value={card.kind} class="provider-card">
      <span class="tile" style:background={card.tile}><card.icon size={18} strokeWidth={2} aria-hidden="true" /></span>
      <span class="title">
        {card.title}
        {#if card.kind === 'local'}<span class="tag">Recommended</span>{/if}
      </span>
      <span class="line">{card.line}</span>
      {#if value === card.kind}<span class="check"><Check size={11} strokeWidth={3} aria-hidden="true" /></span>{/if}
    </RadioGroup.Item>
  {/each}
</RadioGroup.Root>

<style>
  :global(.provider-cards) {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }
  :global(.provider-card) {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    box-sizing: border-box;
    padding: 16px;
    border: 0;
    border-radius: var(--st-radius-card);
    background: var(--st-group);
    box-shadow: inset 0 0 0 0.5px var(--st-separator);
    color: var(--st-text);
    font: inherit;
    text-align: left;
    cursor: default;
  }
  :global(.provider-card[data-state='checked']) {
    background: color-mix(in srgb, var(--st-accent) 12%, var(--st-group));
    box-shadow: inset 0 0 0 1.5px var(--st-accent);
  }
  :global(.provider-card:focus-visible) {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .tile {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    margin-bottom: 8px;
    border-radius: 10px;
    color: #fff;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--st-font-body);
    font-weight: 600;
    white-space: nowrap;
  }
  .tag {
    padding: 1px 6px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 0.5px var(--st-edge);
    color: var(--st-text-secondary);
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .line {
    color: var(--st-text-secondary);
    font-size: var(--st-font-info);
    line-height: 1.4;
  }
  .check {
    position: absolute;
    top: 12px;
    right: 12px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--st-accent);
    color: #fff;
  }
</style>

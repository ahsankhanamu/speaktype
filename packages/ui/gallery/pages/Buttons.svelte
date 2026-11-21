<script lang="ts">
  import { Button } from '@speaktype/ui';

  const variants = [
    { variant: 'primary', label: 'Primary', text: 'Copy' },
    { variant: 'secondary', label: 'Secondary', text: 'Show in Finder' },
    { variant: 'plain', label: 'Plain', text: 'Open in History' },
    { variant: 'destructive', label: 'Destructive', text: 'Delete' },
  ] as const;
  const states = ['Idle', 'Hover', 'Pressed', 'Focus', 'Disabled'];
  const forced = [undefined, 'hover', 'pressed', 'focus'];
</script>

<div class="page">
  <section class="card span">
    <p class="sec">Buttons</p>
    <p class="cap">
      One primary per view. Secondary for everything else. Plain for in-text actions. Destructive = a normal button with a red label (as macOS does), never filled red and never the default; say Delete when something is destroyed, Remove when it only leaves a list.
    </p>
    <div class="row">
      {#each variants as v}
        <div class="cell"><Button variant={v.variant}>{v.text}</Button><p class="lbl">{v.label}</p></div>
      {/each}
      <div class="cell"><Button size={28}>Change…</Button><p class="lbl">Small · 28 pt</p></div>
      <div class="cell"><Button variant="primary" busy>Copy</Button><p class="lbl">Busy</p></div>
      <div class="cell"><Button variant="primary" shortcut={['Super', 'Enter']}>Copy</Button><p class="lbl">With shortcut</p></div>
    </div>
    <div class="states">
      <span></span>
      {#each states as state}<span class="lbl">{state}</span>{/each}
      {#each variants as v}
        <span class="name">{v.label}</span>
        {#each forced as force}
          <span class="cell"><Button variant={v.variant} size={28} data-force-state={force}>{v.text}</Button></span>
        {/each}
        <span class="cell"><Button variant={v.variant} size={28} disabled>{v.text}</Button></span>
      {/each}
    </div>
    <p class="cap behaviour">
      Behaviour: colour changes ease over 120 ms; pressed is one step darker with no movement (macOS buttons don't shift); the cursor stays an arrow, as on macOS; focus ring only for keyboard focus; disabled is 40% opacity, ignores hover and press, and keeps its label readable.
    </p>
  </section>
</div>

<style>
  .states {
    display: grid;
    grid-template-columns: 120px repeat(5, minmax(0, 1fr));
    gap: 12px 16px;
    align-items: center;
    max-width: 1100px;
    margin-top: 18px;
  }
  .states .lbl {
    margin: 0;
  }
  .name {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .behaviour {
    margin: 14px 0 0;
  }
</style>

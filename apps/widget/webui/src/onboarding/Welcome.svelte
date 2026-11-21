<script lang="ts">
  import { MicGlyph } from '@speaktype/ui';
  import Page from './Page.svelte';
  import Paper from './Paper.svelte';
</script>

<Page title="Welcome to SpeakType.">
  {#snippet eyebrow()}
    <span class="tile"><MicGlyph size={24} /></span>
  {/snippet}
  <p>Talk instead of typing, anywhere on your Mac. When words aren't enough, draw on your screen while you explain.</p>
  <p class="caption note">Takes about a minute. Everything stays on this Mac.</p>
  {#snippet panel()}
    <div class="demo" aria-hidden="true">
      <Paper>
        <div class="to">To: Sam</div>
        <p class="body">
          Quick update — the checkout fix is live. Can you retest on
          <span class="inked">your phone when you get a moment?<svg viewBox="0 0 240 8" preserveAspectRatio="none"><path d="M1 5C40 1 80 7 120 4s80-3 119 1" /></svg></span>
        </p>
      </Paper>
      <div class="pill"><span class="dot"></span><span class="bars"><i></i><i></i><i></i><i></i><i></i></span></div>
    </div>
  {/snippet}
</Page>

<style>
  .tile {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin-bottom: 8px;
    border-radius: 12px;
    background: var(--st-control);
    box-shadow: inset 0 0 0 0.5px var(--st-edge);
  }
  .demo {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 28px;
  }
  .demo :global(.paper) {
    width: 100%;
    min-height: 360px;
  }
  .note {
    margin-top: auto;
  }
  .to {
    padding: 12px 18px;
    border-bottom: 0.5px solid rgba(0, 0, 0, 0.12);
    background: #f1f1ef;
    color: #6e6e73;
    font-size: var(--st-font-caption);
  }
  .body {
    margin: 0;
    padding: 20px 18px;
    font-size: 15px;
    line-height: 1.55;
  }
  .inked {
    position: relative;
    white-space: nowrap;
  }
  .inked svg {
    position: absolute;
    left: 0;
    bottom: -3px;
    width: 100%;
    height: 7px;
    fill: none;
    stroke: var(--st-ink-orange);
    stroke-width: 2;
    stroke-linecap: round;
  }
  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: 20px;
    background: #2a2a2c;
    box-shadow: 0 0 0 0.5px rgba(255, 255, 255, 0.14);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--st-live);
  }
  .bars {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 14px;
  }
  .bars i {
    width: 2px;
    height: 10px;
    border-radius: 1px;
    background: #f5f5f7;
  }
  .bars i:nth-child(2n) {
    height: 14px;
  }
  .bars i:nth-child(3) {
    height: 6px;
  }
</style>

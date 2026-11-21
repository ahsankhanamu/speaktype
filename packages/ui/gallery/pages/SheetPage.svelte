<script lang="ts">
  import { Mic } from '@lucide/svelte';
  import { Button, Group, Progress, Row, Sheet, Skeleton, StatusDot } from '@speaktype/ui';

  let open = $state(true);
</script>

<div class="page">
  <h1>History</h1>
  <Skeleton lines={4} />
  <Button variant="primary" onclick={() => (open = true)}>Speech models</Button>
</div>

<Sheet bind:open title="Speech models">
  <p class="lead">Slide toward Accurate for better results with names, accents and noisy rooms. Faster models download quicker and use less memory.</p>
  <Group>
    <Row kind="icon" label="Balanced" caption="Small · best for most people · ~500 MB" icon={Mic} tileColor="#0a84ff">
      {#snippet trailing()}<StatusDot tone="success" label="In use" /><span class="meta">In use</span>{/snippet}
    </Row>
    <Row kind="icon" label="Accurate" caption="Large v3 Turbo · names, accents, noisy rooms · ~1.5 GB" icon={Mic} tileColor="#ff9f0a">
      {#snippet trailing()}<div class="bar"><Progress value={0.38} label="Download" /></div><span class="meta">580 MB of 1.5 GB · 2 min</span>{/snippet}
    </Row>
    <Row kind="icon" label="Most accurate" caption="Large v3 · every language · needs 16 GB of memory · ~3 GB" icon={Mic} tileColor="#ff375f">
      {#snippet trailing()}<StatusDot tone="warning" label="Connection lost" /><span class="meta">Connection lost · retrying in 8 s</span>{/snippet}
    </Row>
  </Group>
  {#snippet footer()}
    <span class="meta">Using 650 MB · English or multilingual follows your Language setting</span>
    <span class="spacer"></span>
    <Button variant="primary" onclick={() => (open = false)}>Done</Button>
  {/snippet}
</Sheet>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 20px;
    width: 640px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
  }
  .lead {
    margin: 0 0 16px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
    line-height: 1.45;
  }
  .meta {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .bar {
    width: 120px;
  }
</style>

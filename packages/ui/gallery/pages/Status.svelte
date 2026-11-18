<script lang="ts">
  import { Progress, ProgressRing, ProBadge, Spinner, StatusDot } from '@speaktype/ui';
</script>

<div class="page">
  <section class="card">
    <p class="sec">Status and progress</p>
    <p class="cap">Status is a dot plus plain words, never a filled banner. Progress is a thin bar with real numbers.</p>
    <div class="row center">
      <span class="status"><StatusDot tone="success" label="Green" />In use</span>
      <span class="status"><StatusDot tone="warning" label="Amber" />Off</span>
      <span class="status"><StatusDot tone="danger" label="Red" />Couldn't download</span>
      <span class="status"><StatusDot tone="live" label="Orange" />Session live</span>
      <span class="status"><StatusDot tone="accent" label="Blue" />Ready</span>
      <span class="status"><StatusDot tone="neutral" label="Grey" />Idle</span>
      <ProBadge />
    </div>
    <div class="progress">
      <div class="bar"><Progress value={0.38} label="Download" /></div>
      <span class="meta">580 MB of 1.5 GB · about 2 min left</span>
    </div>
    <div class="progress">
      <div class="bar"><Progress value={null} label="Preparing" /></div>
      <span class="meta">Preparing…</span>
    </div>
    <div class="row center small">
      <ProgressRing value={0.38} size={16} label="Download" />
      <ProgressRing value={0.7} size={20} label="Download" />
      <Spinner />
    </div>
  </section>
</div>

<style>
  .center {
    align-items: center;
    gap: 26px;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: var(--st-font-info);
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
  }
  .bar {
    flex: 1;
    max-width: 300px;
  }
  .meta {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    font-variant-numeric: tabular-nums;
  }
  .small {
    margin-top: 16px;
  }
</style>

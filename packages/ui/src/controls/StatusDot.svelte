<script lang="ts">
  const TONES = {
    neutral: 'var(--st-text-tertiary)',
    accent: 'var(--st-accent)',
    success: 'var(--st-success)',
    warning: 'var(--st-warning)',
    danger: 'var(--st-danger)',
    live: 'var(--st-live)',
  };

  let { tone, label }: { tone: keyof typeof TONES; label: string } = $props();
</script>

<span class="st-status-dot" style:background={TONES[tone]} role="img" aria-label={label}></span>

<style>
  .st-status-dot {
    display: inline-block;
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
</style>

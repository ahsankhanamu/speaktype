<script lang="ts">
  let { value, label }: { value: number | null; label?: string } = $props();
</script>

<span
  class="st-progress"
  class:indeterminate={value === null}
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={value === null ? undefined : Math.round(value * 100)}
>
  <span class="bar" style:width={value === null ? undefined : `${value * 100}%`}></span>
</span>

<style>
  .st-progress {
    display: block;
    height: 4px;
    border-radius: 2px;
    background: var(--st-fill-pressed);
    overflow: hidden;
  }
  .bar {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--st-accent);
    transition: width var(--st-dur-base) var(--st-ease-out);
  }
  .indeterminate .bar {
    width: 35%;
    animation: slide 1.2s var(--st-ease-out) infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(290%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .indeterminate .bar {
      width: 100%;
      animation: pulse 1.6s ease-in-out infinite;
    }
    @keyframes pulse {
      50% {
        opacity: 0.35;
      }
    }
  }
</style>

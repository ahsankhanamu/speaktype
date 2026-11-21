<script lang="ts">
  let { value, size = 16, label }: { value: number; size?: 16 | 20; label?: string } = $props();

  const radius = $derived(size / 2 - 1.5);
  const circumference = $derived(2 * Math.PI * radius);
</script>

<svg
  class="st-progress-ring"
  width={size}
  height={size}
  viewBox="0 0 {size} {size}"
  fill="none"
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(value * 100)}
>
  <circle cx={size / 2} cy={size / 2} r={radius} stroke="var(--st-fill-pressed)" stroke-width="2" />
  <circle
    cx={size / 2}
    cy={size / 2}
    r={radius}
    stroke="var(--st-accent)"
    stroke-width="2"
    stroke-linecap="round"
    stroke-dasharray={circumference}
    stroke-dashoffset={circumference * (1 - value)}
    transform="rotate(-90 {size / 2} {size / 2})"
  />
</svg>

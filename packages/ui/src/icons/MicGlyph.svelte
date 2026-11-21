<script lang="ts">
  /** SpeakType's microphone: the same filled mic on its stand as the menu bar icon. */
  let { size = 18, label }: { size?: number; label?: string } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="currentColor"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
>
  <rect x="7.6" y="0.6" width="8.8" height="16" rx="4.4" />
  <path d="M4.9 10.4v1.3a7.1 7.1 0 0 0 14.2 0v-1.3" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" />
  <rect x="11" y="18.6" width="2" height="3.6" />
  <rect x="7.4" y="21.6" width="9.2" height="2.3" rx="1.15" />
</svg>

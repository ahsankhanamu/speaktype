<script lang="ts">
  export let value: 'auto' | 'light' | 'dark' = 'auto';
  export let labelId: string | undefined = undefined;

  const themes: Array<{ value: 'auto' | 'light' | 'dark'; title: string }> = [
    { value: 'auto', title: 'Auto' },
    { value: 'light', title: 'Light' },
    { value: 'dark', title: 'Dark' },
  ];
</script>

<div class="segmented segmented-icons" role="radiogroup" aria-labelledby={labelId}>
  {#each themes as theme}
    <input
      type="radio"
      name="theme-pref"
      id="theme-{theme.value}"
      value={theme.value}
      bind:group={value}
    />
    <label class="segment" for="theme-{theme.value}" title={theme.title}>
      {#if theme.value === 'auto'}
        <svg
          class="theme-icon"
          viewBox="0 0 24 24"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        ><circle cx="12" cy="12" r="4" /><path
            d="M12 2a10 10 0 0 1 0 20V12z"
            fill="currentColor"
            opacity="0.35"
          /></svg
        >
      {:else if theme.value === 'light'}
        <svg
          class="theme-icon"
          viewBox="0 0 24 24"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          aria-hidden="true"
        ><circle cx="12" cy="12" r="4" /><path
            d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"
          /></svg
        >
      {:else}
        <svg
          class="theme-icon"
          viewBox="0 0 24 24"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        ><path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z" /></svg
        >
      {/if}
    </label>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    background: var(--surface, #232326);
    border: 1px solid var(--border, #333);
    border-radius: var(--radius-md, 8px);
  }
  .segmented input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .segment {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 7px 10px;
    border-radius: 6px;
    gap: 6px;
    color: var(--text-muted, #94949c);
    cursor: pointer;
    user-select: none;
    transition: background-color 0.15s, color 0.15s;
    line-height: 1;
  }
  .segmented-icons .segment {
    padding: 6px 9px;
  }
  .segmented-icons .theme-icon {
    display: block;
  }
  .segment:hover {
    color: var(--text-strong, #fff);
  }
  .segmented input:checked + .segment {
    background: var(--surface-4, #333);
    color: var(--text-strong, #fff);
  }
  .segmented input:focus-visible + .segment {
    outline: 2px solid var(--accent, #3b82f6);
    outline-offset: 1px;
  }
</style>
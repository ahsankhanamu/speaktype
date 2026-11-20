<script lang="ts">
  import { truncate } from '@speaktype/ui';
  import { RadioGroup } from 'bits-ui';
  import { liveQuery, useEvent } from '../data';
  import { getInputDevices, getSettings, saveSettings } from '../ipc';
  import { litBars } from './meter';
  import { micLevelsStart, micLevelsStop } from './micLevels';

  const METER_BARS = 5;
  const KEEPALIVE_MS = 20_000;

  const devices = liveQuery('input-devices', getInputDevices, { invalidateOn: ['settings-changed', 'mic-devices-changed'] });
  const settings = liveQuery('settings', getSettings, { invalidateOn: ['settings-changed'] });

  let levels = $state<Record<string, number>>({});
  useEvent<{ levels: { device: string; level: number }[] }>('mic-levels', (event) => {
    levels = Object.fromEntries(event.levels.map(({ device, level }) => [device, level]));
  });

  liveQuery('mic-levels-keepalive', micLevelsStart, { invalidateOn: [], pollMs: KEEPALIVE_MS, refetchOnFocus: false });
  $effect(() => () => {
    micLevelsStop();
  });

  const selected = $derived(settings.data?.input_device ?? '');
  const choices = $derived.by(() => {
    const present = devices.data?.devices ?? [];
    const fallback = devices.data?.default ?? '';
    const list = [{ value: '', name: 'System default', caption: fallback, level: levels[fallback] ?? 0 }];
    for (const { name, label } of present) {
      const caption = label.includes('Bluetooth') ? 'Bluetooth · lower quality while in use' : '';
      list.push({ value: name, name, caption, level: levels[name] ?? 0 });
    }
    if (selected && !present.some(({ name }) => name === selected)) {
      list.push({ value: selected, name: selected, caption: 'Not connected', level: 0 });
    }
    return list;
  });

  async function choose(value: string) {
    await saveSettings({ ...(await getSettings()), input_device: value || null });
  }

</script>

<RadioGroup.Root value={selected} onValueChange={choose} class="mic-list" aria-label="Microphone">
  {#each choices as choice (choice.value)}
    <RadioGroup.Item value={choice.value} class="mic-choice">
      <span class="dot"></span>
      <span class="text">
        <span class="name" use:truncate={{ mode: 'tail', full: choice.name }}></span>
        {#if choice.caption}<span class="caption" use:truncate={{ mode: 'tail', full: choice.caption }}></span>{/if}
      </span>
      <span class="meter" role="meter" aria-label="Input level" aria-valuemin="0" aria-valuemax="1" aria-valuenow={choice.level}>
        {#each { length: METER_BARS } as _, bar}
          <span class="bar" class:on={bar < litBars(choice.level, METER_BARS)} style:height="{6 + bar * 2.5}px"></span>
        {/each}
      </span>
    </RadioGroup.Item>
  {/each}
</RadioGroup.Root>

<style>
  :global(.mic-list) {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  :global(.mic-choice) {
    display: flex;
    align-items: center;
    gap: 14px;
    box-sizing: border-box;
    width: 100%;
    min-height: 52px;
    padding: 8px 18px 8px 16px;
    border: 0;
    border-radius: 12px;
    background: var(--st-field);
    box-shadow: inset 0 0 0 0.5px var(--st-separator);
    color: var(--st-text);
    font: inherit;
    text-align: left;
  }
  :global(.mic-choice[data-state='checked']) {
    background: color-mix(in srgb, var(--st-accent) 14%, transparent);
    box-shadow: inset 0 0 0 1px var(--st-accent);
  }
  :global(.mic-choice:focus-visible) {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  :global(.mic-choice[data-state='checked']:focus-visible) {
    box-shadow:
      inset 0 0 0 1px var(--st-accent),
      var(--st-focus-ring);
  }
  .dot {
    flex: none;
    box-sizing: border-box;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--st-text) 34%, transparent);
  }
  :global(.mic-choice[data-state='checked']) .dot {
    background: radial-gradient(circle, #fff 0 3.5px, var(--st-accent) 4px);
    box-shadow: none;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .name {
    font-size: var(--st-font-body);
  }
  .caption {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .meter {
    display: flex;
    flex: none;
    align-items: flex-end;
    gap: 2px;
    height: 16px;
  }
  .bar {
    width: 3px;
    border-radius: 1.5px;
    background: var(--st-track);
    transition: background-color var(--st-dur-fast) var(--st-ease-out);
  }
  .bar.on {
    background: var(--st-success);
  }
</style>

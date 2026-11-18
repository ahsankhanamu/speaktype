<script lang="ts">
  import { Combobox, Keycap, Select } from '@speaktype/ui';

  let template = $state('bug');
  let open = $state(true);
  let key = $state('right-option');
  let notAvailable = $state('');
  let language = $state('auto');
  let languageNoMatch = $state('auto');
  let languagePort = $state('pt-br');
  let openNoMatch = $state(true);
  let openPort = $state(true);

  const templates = [
    { value: 'auto', label: 'Auto' },
    { value: 'bug', label: 'Bug report' },
    { value: 'feature', label: 'Feature request' },
  ];
  const languages = [
    { value: 'auto', label: 'Automatic' },
    { value: 'pt', label: 'Portuguese', hint: 'pt' },
    { value: 'pt-br', label: 'Portuguese (Brazil)' },
    { value: 'es', label: 'Spanish', hint: 'es' },
    { value: 'fr', label: 'French', hint: 'fr' },
  ];
</script>

<div class="page">
  <section class="card">
    <p class="sec">Dropdown (Select)</p>
    <p class="cap">Trigger · Value · Content · Item · Separator, composable. Content renders in a portal at the window root, with the shared popover shadow.</p>
    <div class="row" style="gap: 28px">
      <div class="cell left">
        <Select.Root bind:value={key} placement="below">
          <Select.Trigger>
            {#snippet leading()}<span class="keycap"><Keycap key="⌥" /></span>{/snippet}
            Right Option
          </Select.Trigger>
          <Select.Content><Select.Item value="right-option" label="Right Option" /></Select.Content>
        </Select.Root>
        <p class="lbl">Closed</p>
        <div class="gap"></div>
        <Select.Root bind:value={notAvailable} disabled>
          <Select.Trigger>Not available</Select.Trigger>
        </Select.Root>
        <p class="lbl">Disabled</p>
      </div>
      <div class="cell left w240 fill">
        <Select.Root bind:value={template} bind:open placement="below">
          <Select.Trigger>{templates.find((t) => t.value === template)?.label}</Select.Trigger>
          <Select.Content>
            {#each templates as item}<Select.Item value={item.value} label={item.label} />{/each}
            <Select.Item value="review" label="Code review · Pro" disabled />
            <Select.Separator />
            <Select.Item value="more" label="More templates…" />
          </Select.Content>
        </Select.Root>
        <div class="menu-space"></div>
      </div>
    </div>

    <p class="sec combo">Combobox</p>
    <p class="cap">A dropdown you can type into, for long lists like Language or Microphone. Typing filters; matches are bold; Return picks the highlighted one.</p>
    <div class="row" style="gap: 28px">
      <div class="cell left w220">
        <Combobox items={languages} bind:value={language} label="Language" />
        <p class="lbl">Closed</p>
        <div class="gap"></div>
        <Combobox
          items={languages}
          bind:value={languageNoMatch}
          bind:open={openNoMatch}
          query="xyz"
          label="Language, no results"
          emptyMessage={(q) => `No languages match “${q}”`}
        />
        <div class="menu-space-empty"></div>
        <p class="lbl">No results</p>
      </div>
      <div class="cell left w240">
        <Combobox items={languages} bind:value={languagePort} bind:open={openPort} query="port" label="Language, filtered" />
        <div class="menu-space-small"></div>
        <p class="lbl">Open, filtered</p>
      </div>
    </div>
  </section>
</div>

<style>
  .left {
    align-items: flex-start;
  }
  .keycap {
    --keycap-size: 20px;
    display: inline-flex;
  }
  .gap {
    height: 10px;
  }
  .menu-space {
    height: 245px;
  }
  .menu-space-empty {
    height: 56px;
  }
  .menu-space-small {
    height: 100px;
  }
  .combo {
    margin-top: 22px;
  }
</style>

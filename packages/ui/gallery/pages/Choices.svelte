<script lang="ts">
  import { Checkbox, RadioList, Segmented, ShortcutField, Switch } from '@speaktype/ui';

  let view = $state('timeline');
  let on = $state(true);
  let off = $state(false);
  let radio = $state('selected');
  let shortcut = $state(['Option', 'Super', 'K']);
  let recording = $state<string[]>([]);
  let conflict = $state(['Option', 'Super', 'S']);
</script>

<div class="page">
  <section class="card stack">
    <div>
      <p class="sec">Choices and toggles</p>
      <p class="cap tight">Segmented for 2–4 views; switch for on/off settings; radio list for picking one of several described options.</p>
      <div class="row center">
        <Segmented
          label="View"
          bind:value={view}
          items={[
            { value: 'timeline', label: 'Timeline' },
            { value: 'result', label: 'Result' },
            { value: 'details', label: 'Details' },
          ]}
        />
        <Switch bind:checked={on} label="On" />
        <Switch bind:checked={off} label="Off" />
        <div class="inline-radios">
          <RadioList
            label="Example"
            bind:value={radio}
            items={[
              { value: 'selected', label: 'Selected' },
              { value: 'other', label: 'Not selected' },
            ]}
          />
        </div>
      </div>
    </div>
    <div>
      <p class="sec">Checkbox</p>
      <p class="cap tight">
        For picking several things at once, like what to include in a report or which items to delete. A switch is for a single setting that takes effect immediately.
      </p>
      <div class="row center">
        <Checkbox checked label="Checked" />
        <Checkbox checked={false} label="Unchecked" />
        <Checkbox checked="mixed" label="Some selected" />
        <Checkbox checked disabled label="Disabled" />
        <Checkbox checked={false} label="Focus" data-force-state="focus" />
      </div>
    </div>
    <div>
      <p class="sec">Shortcut field</p>
      <p class="cap tight">Click to record. Keys always in Apple's order: ⌃ ⌥ ⇧ ⌘.</p>
      <div class="row center">
        <ShortcutField bind:value={shortcut} />
        <div data-screenshot-click><ShortcutField bind:value={recording} /></div>
        <ShortcutField bind:value={conflict} conflict="Already used for screenshots" />
      </div>
    </div>
  </section>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .tight {
    margin-bottom: 10px;
  }
  .center {
    align-items: center;
    gap: 22px;
  }
</style>

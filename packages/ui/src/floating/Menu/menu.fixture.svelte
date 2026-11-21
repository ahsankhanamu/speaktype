<script lang="ts">
  import { DropdownMenu, MenuItem } from './index';

  let side = $state('right');
</script>

<DropdownMenu.Root open>
  <DropdownMenu.Trigger>Menu</DropdownMenu.Trigger>
  <DropdownMenu.Content>
    <DropdownMenu.RadioGroup bind:value={side}>
      <MenuItem value="right" label="Right" />
      <MenuItem value="left" label="Left" />
    </DropdownMenu.RadioGroup>
    <MenuItem label="Reset" />
  </DropdownMenu.Content>
</DropdownMenu.Root>

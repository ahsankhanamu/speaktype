import { ContextMenu as ContextMenuPrimitive, DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';
import ContextContent from './ContextContent.svelte';
import DropdownContent from './DropdownContent.svelte';

export { default as MenuItem } from './MenuItem.svelte';
export { default as MenuSeparator } from '../Separator.svelte';

export const DropdownMenu = {
  Root: DropdownMenuPrimitive.Root,
  Trigger: DropdownMenuPrimitive.Trigger,
  RadioGroup: DropdownMenuPrimitive.RadioGroup,
  Content: DropdownContent,
};

export const ContextMenu = {
  Root: ContextMenuPrimitive.Root,
  Trigger: ContextMenuPrimitive.Trigger,
  RadioGroup: ContextMenuPrimitive.RadioGroup,
  Content: ContextContent,
};

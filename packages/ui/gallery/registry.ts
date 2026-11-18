import type { Component } from 'svelte';
import Alert from './pages/Alert.svelte';
import Buttons from './pages/Buttons.svelte';
import Choices from './pages/Choices.svelte';
import Components from './pages/Components.svelte';
import Dropdown from './pages/Dropdown.svelte';
import Feedback from './pages/Feedback.svelte';
import Fixtures from './pages/Fixtures.svelte';
import Patterns from './pages/Patterns.svelte';
import PopupMenuPage from './pages/PopupMenuPage.svelte';
import SheetPage from './pages/SheetPage.svelte';
import Status from './pages/Status.svelte';
import TextFields from './pages/TextFields.svelte';

export const pages: { id: string; title: string; component: Component }[] = [
  { id: 'components', title: 'Components board', component: Components },
  { id: 'buttons', title: 'Buttons', component: Buttons },
  { id: 'text-fields', title: 'Text fields', component: TextFields },
  { id: 'dropdown', title: 'Dropdown and combobox', component: Dropdown },
  { id: 'choices', title: 'Choices, checkbox, shortcut', component: Choices },
  { id: 'status', title: 'Status and progress', component: Status },
  { id: 'feedback', title: 'Floating feedback', component: Feedback },
  { id: 'patterns', title: 'Rows, sidebar, pickers', component: Patterns },
  { id: 'popup-menu', title: 'Pop-up menu', component: PopupMenuPage },
  { id: 'sheet', title: 'Sheet', component: SheetPage },
  { id: 'alert', title: 'Alert', component: Alert },
  { id: 'fixtures', title: 'Behaviour fixtures', component: Fixtures },
];

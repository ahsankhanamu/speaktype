import '@speaktype/ui/tokens.css';
import '../harness.css';
import { mount } from 'svelte';
import Widget from '../../../src/widget/Widget.svelte';
import { installMockIpc } from '../mock-ipc';

declare global {
  interface Window {
    __fixture: Record<string, unknown>;
  }
}

type Step = { event: string; payload: unknown };
type WidgetFixture = Record<string, unknown> & { $states: Record<string, Step[]> };

const params = new URLSearchParams(location.search);
const fixture: WidgetFixture = await (await fetch(`/tests/visual/fixtures/widget${params.get('fixture') ? `-${params.get('fixture')}` : ''}.json`)).json();

Object.assign(fixture.get_settings as object, params.get('size') && { widget_size: params.get('size') });
Object.assign(fixture.get_onboarding_status as object, params.get('setup') === 'unfinished' && { accessibility: false });

// The widget is dark in both themes; the theme only changes what sits behind it.
document.documentElement.dataset.theme = 'dark';
document.body.style.background = params.get('theme') === 'light' ? '#e8e8ea' : '#2b2b2d';
window.__fixture = fixture;
installMockIpc(fixture);
(window.__TAURI_INTERNALS__ as Record<string, unknown>).metadata = {
  currentWindow: { label: 'main' },
  currentWebview: { windowLabel: 'main', label: 'main' },
};
// The real window resizes around the pill's centre; centring the page content does the same here.
const app = document.getElementById('app')!;
Object.assign(app.style, { position: 'fixed', left: '50%', top: '50%', transform: 'translate(-50%, -50%)' });
mount(Widget, { target: app });

for (const { event, payload } of fixture.$states[params.get('state') ?? 'idle']) {
  window.__mock.emit(event, payload);
}

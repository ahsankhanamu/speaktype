import '@speaktype/ui/tokens.css';
import '../harness.css';
import { mount } from 'svelte';
import Onboarding from '../../../src/onboarding/Onboarding.svelte';
import { installMockIpc } from '../mock-ipc';

declare global {
  interface Window {
    __fixture: Record<string, unknown>;
  }
}

const params = new URLSearchParams(location.search);
const fixture = await (await fetch(`/tests/visual/fixtures/onboarding-${params.get('fixture')}.json`)).json();

document.documentElement.dataset.theme = params.get('theme') ?? 'dark';
window.__fixture = fixture;
installMockIpc(fixture);
mount(Onboarding, { target: document.getElementById('app')! });

for (const { event, payload, delayMs = 0 } of fixture.$events ?? []) {
  setTimeout(() => window.__mock.emit(event, payload), delayMs);
}

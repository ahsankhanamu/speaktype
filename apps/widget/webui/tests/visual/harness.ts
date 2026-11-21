import '@speaktype/ui/tokens.css';
import './harness.css';
import { mount } from 'svelte';
import { installMockIpc } from './mock-ipc';
import Scene from './scenes/Scene.svelte';

const params = new URLSearchParams(location.search);
const fixture = await (await fetch(`/tests/visual/fixtures/${params.get('fixture')}.json`)).json();

document.documentElement.dataset.theme = params.get('theme') ?? 'dark';
installMockIpc(fixture);
mount(Scene, { target: document.getElementById('app')!, props: { scene: params.get('scene') ?? '' } });

for (const { event, payload, delayMs = 0 } of fixture.$events ?? []) {
  setTimeout(() => window.__mock.emit(event, payload), delayMs);
}

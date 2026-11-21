import '@speaktype/ui/tokens.css';
import { mount } from 'svelte';
import { startTheme } from '../lib/theme';
import App from './App.svelte';

startTheme();
mount(App, { target: document.getElementById('app')! });

import { mount } from 'svelte';
import '../lib/theme.css';
import '../lib/settings.css';
import { startTheme } from '../lib/theme';
import Settings from './Settings.svelte';

startTheme();
mount(Settings, { target: document.getElementById('app')! });
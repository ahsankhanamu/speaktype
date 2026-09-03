import '../lib/theme.css';
import { mount } from 'svelte';
import About from './About.svelte';
import { startTheme } from '../lib/theme';

startTheme();

const target = document.getElementById('app');
if (target) {
  mount(About, { target });
} else {
  mount(About, { target: document.body });
}
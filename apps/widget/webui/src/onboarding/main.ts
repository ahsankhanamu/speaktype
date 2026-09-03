import '../lib/theme.css';
import { mount } from 'svelte';
import Onboarding from './Onboarding.svelte';
import { startTheme } from '../lib/theme';

startTheme();

const target = document.getElementById('app');
if (target) {
  mount(Onboarding, { target });
} else {
  mount(Onboarding, { target: document.body });
}
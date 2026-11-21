import '@speaktype/ui/tokens.css';
import './gallery.css';
import { mount } from 'svelte';
import App from './App.svelte';

mount(App, { target: document.getElementById('app')! });

import type { Component } from 'svelte';
import { Clock, Cloud, Cog, Contrast, Database, Keyboard, Lock, Mic, Pencil, SlidersHorizontal } from '@lucide/svelte';

export type PageId =
  | 'history' | 'general' | 'keyboard' | 'appearance' | 'dictation' | 'sessions'
  | 'providers' | 'permissions' | 'storage' | 'advanced' | 'about';

type Icon = Component<{ size?: number; strokeWidth?: number }>;

/** Sidebar items in order; `gap` starts a new group. */
export const PAGES: { id: PageId; label: string; icon: Icon; tile: string; gap?: true }[] = [
  { id: 'history', label: 'History', icon: Clock, tile: '#30b0c7' },
  { id: 'general', label: 'General', icon: Cog, tile: '#8e8e93', gap: true },
  { id: 'keyboard', label: 'Keyboard', icon: Keyboard, tile: '#8e8e93' },
  { id: 'appearance', label: 'Appearance', icon: Contrast, tile: '#1c1c1e' },
  { id: 'dictation', label: 'Dictation', icon: Mic, tile: '#0a84ff', gap: true },
  { id: 'sessions', label: 'Sessions', icon: Pencil, tile: '#ef643c' },
  { id: 'providers', label: 'Providers', icon: Cloud, tile: '#bf5af2' },
  { id: 'permissions', label: 'Permissions', icon: Lock, tile: '#5e5ce6', gap: true },
  { id: 'storage', label: 'Storage', icon: Database, tile: '#8e8e93' },
  { id: 'advanced', label: 'Advanced', icon: SlidersHorizontal, tile: '#8e8e93' },
];

export const GROUPS = PAGES.reduce<(typeof PAGES)[]>((groups, page) => {
  if (page.gap || !groups.length) groups.push([]);
  groups[groups.length - 1].push(page);
  return groups;
}, []);

const ROUTES = new Set<string>([...PAGES.map((page) => page.id), 'about']);
const STORAGE_KEY = 'speaktype:last-page';

export const currentPage = $state<{ id: PageId }>({ id: 'history' });

function remembered(): PageId | null {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved && ROUTES.has(saved) ? (saved as PageId) : null;
  } catch {
    return null;
  }
}

function fromHash(): PageId | null {
  const id = location.hash.replace(/^#\//, '');
  return ROUTES.has(id) ? (id as PageId) : null;
}

function show(id: PageId) {
  currentPage.id = id;
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    // Storage can be blocked (private window); the page just isn't remembered.
  }
}

export function navigate(id: PageId) {
  location.hash = `/${id}`;
  show(id);
}

/** Reads the hash (or the remembered page) and follows later hash changes. */
export function startRouter() {
  const follow = () => {
    const id = fromHash();
    if (id) show(id);
  };
  show(fromHash() ?? remembered() ?? 'history');
  window.addEventListener('hashchange', follow);
  return () => window.removeEventListener('hashchange', follow);
}

import { getContext, setContext } from 'svelte';

export type SelectLayout = { trigger: HTMLElement | null; readonly placement: 'aligned' | 'below' };

const KEY = Symbol('st-select');

export const setLayout = (layout: SelectLayout) => setContext(KEY, layout);
export const getLayout = () => getContext<SelectLayout>(KEY);

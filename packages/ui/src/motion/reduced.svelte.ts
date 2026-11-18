import { createSubscriber } from 'svelte/reactivity';

const QUERY = '(prefers-reduced-motion: reduce)';

const subscribe = createSubscriber((update) => {
  const media = matchMedia(QUERY);
  media.addEventListener('change', update);
  return () => media.removeEventListener('change', update);
});

export const reducedMotion = {
  get current() {
    subscribe();
    return matchMedia(QUERY).matches;
  },
};

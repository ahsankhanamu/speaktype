import { hint } from './hint.svelte';

const HINT_DELAY_MS = 500;

type Params = { mode: 'tail' | 'middle'; full: string };

function middleEllipsis(node: HTMLElement, full: string) {
  node.textContent = full;
  if (node.scrollWidth <= node.clientWidth) return;
  let [lo, hi] = [0, full.length];
  while (lo < hi) {
    const kept = Math.ceil((lo + hi) / 2);
    node.textContent = `${full.slice(0, Math.floor(kept / 2))}…${full.slice(full.length - Math.ceil(kept / 2))}`;
    if (node.scrollWidth <= node.clientWidth) lo = kept;
    else hi = kept - 1;
  }
  node.textContent = `${full.slice(0, Math.floor(lo / 2))}…${full.slice(full.length - Math.ceil(lo / 2))}`;
}

export function truncate(node: HTMLElement, params: Params) {
  let { mode, full } = params;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const isTruncated = () => (mode === 'middle' ? node.textContent !== full : node.scrollWidth > node.clientWidth);

  function render() {
    if (mode === 'middle') middleEllipsis(node, full);
    else node.textContent = full;
  }

  function hide() {
    clearTimeout(timer);
    if (hint.anchor === node) hint.anchor = null;
  }

  function show() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      if (!isTruncated()) return;
      hint.text = full;
      hint.anchor = node;
    }, HINT_DELAY_MS);
  }

  node.style.overflow = 'hidden';
  node.style.whiteSpace = 'nowrap';
  node.style.textOverflow = mode === 'tail' ? 'ellipsis' : 'clip';
  render();

  const observer = new ResizeObserver(render);
  observer.observe(node);
  node.addEventListener('mouseenter', show);
  node.addEventListener('mouseleave', hide);

  return {
    update(next: Params) {
      ({ mode, full } = next);
      node.style.textOverflow = mode === 'tail' ? 'ellipsis' : 'clip';
      render();
    },
    destroy() {
      hide();
      observer.disconnect();
      node.removeEventListener('mouseenter', show);
      node.removeEventListener('mouseleave', hide);
    },
  };
}

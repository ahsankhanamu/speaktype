window.matchMedia ??= (() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;

Element.prototype.animate ??= function () {
  return {
    cancel() {},
    set onfinish(done: () => void) {
      queueMicrotask(done);
    },
  } as unknown as Animation;
};

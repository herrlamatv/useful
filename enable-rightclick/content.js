(() => {
  'use strict';

  // devtools-check laeuft ueber outer- vs innergroesse
  for (const [o, i] of [['outerWidth', 'innerWidth'], ['outerHeight', 'innerHeight']]) {
    try {
      Object.defineProperty(window, o, { configurable: true, get: () => window[i], set() {} });
    } catch (_) {}
  }

  // capture auf window kommt vor den handlern auf document dran
  const stop = (e) => e.stopImmediatePropagation();
  for (const t of ['contextmenu', 'selectstart', 'copy', 'cut', 'paste', 'dragstart']) {
    window.addEventListener(t, stop, true);
  }

  try {
    Object.defineProperty(document, 'onkeydown', { configurable: true, get: () => null, set() {} });
  } catch (_) {}

  const keys = new Set([114, 123]); // f3, f12
  const ctrlKeys = new Set([67, 68, 70, 71, 73, 74, 80, 83, 85]); // c d f g i j p s u
  window.addEventListener('keydown', (e) => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (keys.has(e.keyCode) || (ctrl && ctrlKeys.has(e.keyCode)) || (e.altKey && e.keyCode === 68)) {
      e.stopImmediatePropagation();
    }
  }, true);
})();

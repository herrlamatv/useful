/* ai chat exporter - by herrlamatv
 * 1:1-Screenshot-PDF, Seite der Webseite: Chat ganz nachladen, schwebende
 * Leisten (Kopfzeile, Eingabefeld) ausblenden, dann Stück für Stück scrollen.
 * Die Aufnahmen selbst macht der Service Worker (captureVisibleTab), das
 * Zusammensetzen die PDF-Seite. Danach wird alles wiederhergestellt.
 */

(() => {
  const AIX = globalThis.AIX;
  if (!AIX || AIX.shot) return;
  AIX.shot = true;

  let state = null;

  const isDocScroller = (el) => el === document.scrollingElement || el === document.documentElement || el === document.body;

  /** Sichtbarer Innenbereich des Scroll-Containers (ohne Scrollbalken), auf den Viewport begrenzt. */
  function clientRect(el) {
    if (isDocScroller(el)) return { left: 0, top: 0, width: document.documentElement.clientWidth, height: innerHeight };
    const r = el.getBoundingClientRect();
    const left = Math.max(0, r.left + el.clientLeft);
    const top = Math.max(0, r.top + el.clientTop);
    return {
      left,
      top,
      width: Math.min(el.clientWidth, document.documentElement.clientWidth - left),
      height: Math.min(el.clientHeight, innerHeight - top)
    };
  }

  const intersects = (a, b) => a.left < b.left + b.width && a.left + a.width > b.left && a.top < b.top + b.height && a.top + a.height > b.top;

  async function imagesSettled(root, timeout = 1500) {
    const imgs = Array.from(root.querySelectorAll('img')).filter((img) => !img.complete);
    if (!imgs.length) return;
    await Promise.race([
      Promise.all(
        imgs.map(
          (img) =>
            new Promise((r) => {
              img.addEventListener('load', r, { once: true });
              img.addEventListener('error', r, { once: true });
            })
        )
      ),
      AIX.sleep(timeout)
    ]);
  }

  AIX.handlers['aix:shot:prep'] = async ({ loadAll = true }) => {
    const adapter = AIX.adapter();
    if (loadAll && adapter) {
      try {
        await AIX.loadAll(adapter);
      } catch (e) {}
    }
    const scroller = AIX.scroller(adapter);
    const rect = clientRect(scroller);
    AIX.capturing = true;

    /* Feste und klebende Elemente über dem Chat ausblenden - sonst stehen sie auf jeder Seite. */
    const hidden = [];
    for (const el of document.querySelectorAll('body *')) {
      if (el.contains(scroller)) continue;
      const pos = getComputedStyle(el).position;
      if (pos !== 'fixed' && pos !== 'sticky') continue;
      if (!intersects(el.getBoundingClientRect(), rect)) continue;
      hidden.push([el, el.style.getPropertyValue('visibility'), el.style.getPropertyPriority('visibility')]);
      el.style.setProperty('visibility', 'hidden', 'important');
    }
    for (const el of document.querySelectorAll('[data-aix-host]')) {
      hidden.push([el, el.style.getPropertyValue('visibility'), el.style.getPropertyPriority('visibility')]);
      el.style.setProperty('visibility', 'hidden', 'important');
    }
    const behavior = scroller.style.scrollBehavior;
    scroller.style.scrollBehavior = 'auto';
    state = { scroller, hidden, top: scroller.scrollTop, behavior };
    return {
      rect,
      viewport: { width: document.documentElement.clientWidth, height: innerHeight },
      scrollHeight: scroller.scrollHeight,
      clientHeight: rect.height
    };
  };

  AIX.handlers['aix:shot:scroll'] = async ({ y }) => {
    if (!state) throw new Error('not-prepared');
    const s = state.scroller;
    s.scrollTop = y;
    await AIX.sleep(380);
    await imagesSettled(isDocScroller(s) ? document : s);
    return { top: s.scrollTop, scrollHeight: s.scrollHeight, clientHeight: clientRect(s).height };
  };

  AIX.handlers['aix:shot:done'] = async () => {
    if (state) {
      for (const [el, value, prio] of state.hidden) {
        if (value) el.style.setProperty('visibility', value, prio);
        else el.style.removeProperty('visibility');
      }
      state.scroller.scrollTop = state.top;
      state.scroller.style.scrollBehavior = state.behavior;
      state = null;
    }
    AIX.capturing = false;
    return {};
  };
})();

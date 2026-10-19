/* ai chat exporter - by herrlamatv
 * Universal-Modus: „Chat-Bereich auswählen“. Maus über die Seite, der
 * Bereich unter dem Zeiger wird umrahmt, Klick wählt ihn. Pfeil hoch/runter
 * = eine Ebene größer/kleiner, Esc = abbrechen. Danach exportiert das Popup
 * genau diesen Bereich (AIX.picked).
 */

(() => {
  const AIX = globalThis.AIX;
  if (!AIX || AIX.picker) return;
  AIX.picker = true;

  const TEXT = {
    de: {
      banner: 'Klick auf den Bereich mit dem Chat · ↑ größer · ↓ kleiner · Esc abbrechen',
      done: 'Bereich gewählt: {n} Beiträge – öffne jetzt wieder die Erweiterung',
      none: 'In diesem Bereich habe ich keine Beiträge gefunden – versuch einen größeren (↑)'
    },
    en: {
      banner: 'Click the area that holds the chat · ↑ bigger · ↓ smaller · Esc cancel',
      done: 'Area picked: {n} messages – now open the extension again',
      none: 'No messages found in this area – try a bigger one (↑)'
    }
  };

  let active = null;

  function stop() {
    if (!active) return;
    document.removeEventListener('mousemove', active.move, true);
    document.removeEventListener('click', active.click, true);
    document.removeEventListener('keydown', active.key, true);
    active.box.remove();
    active.banner.remove();
    active = null;
  }

  function start() {
    stop();
    const L = AIX.L(TEXT);
    const box = document.createElement('div');
    box.setAttribute('data-aix-host', '');
    box.style.cssText =
      'position:fixed;z-index:2147483646;pointer-events:none;border:2px dashed #b98b52;background:rgba(185,139,82,.08);border-radius:10px;transition:all .08s;display:none';
    const banner = document.createElement('div');
    banner.setAttribute('data-aix-host', '');
    banner.textContent = L.banner;
    banner.style.cssText =
      'position:fixed;z-index:2147483647;top:14px;left:50%;transform:translateX(-50%);background:#fffdf8;color:#1f1d18;border:1px solid #e7dfd0;' +
      'border-radius:999px;padding:8px 16px;font:500 13px/1.3 -apple-system,"Segoe UI",system-ui,sans-serif;box-shadow:0 10px 26px rgba(74,62,40,.16);pointer-events:none';
    document.documentElement.append(box, banner);

    const state = { box, banner, target: null, stack: [] };
    const draw = () => {
      if (!state.target) return;
      const r = state.target.getBoundingClientRect();
      Object.assign(box.style, { display: 'block', left: r.left - 3 + 'px', top: r.top - 3 + 'px', width: r.width + 6 + 'px', height: r.height + 6 + 'px' });
    };
    state.move = (ev) => {
      const el = document.elementFromPoint(ev.clientX, ev.clientY);
      if (!el || el === box || el === banner || el === document.documentElement) return;
      state.target = el;
      state.stack = [];
      draw();
    };
    state.key = (ev) => {
      if (ev.key === 'Escape') {
        ev.preventDefault();
        stop();
      } else if (ev.key === 'ArrowUp' && state.target && state.target.parentElement && state.target.parentElement !== document.documentElement) {
        ev.preventDefault();
        state.stack.push(state.target);
        state.target = state.target.parentElement;
        draw();
      } else if (ev.key === 'ArrowDown' && state.stack.length) {
        ev.preventDefault();
        state.target = state.stack.pop();
        draw();
      }
    };
    state.click = (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      if (!state.target) return;
      const n = AIX.guessTurns(state.target).length;
      if (!n) {
        AIX.toast(L.none, 'error');
        return;
      }
      AIX.picked = state.target;
      stop();
      AIX.toast(L.done.replace('{n}', n));
    };
    document.addEventListener('mousemove', state.move, true);
    document.addEventListener('click', state.click, true);
    document.addEventListener('keydown', state.key, true);
    active = state;
  }

  AIX.handlers['aix:pick'] = async () => {
    start();
    return { started: true };
  };
})();

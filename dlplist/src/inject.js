/* dlplist - by herrlamatv
 * Diese Funktionen laufen IN der Webseite (chrome.scripting.executeScript,
 * freigegeben durch den Klick im Kontextmenü = activeTab). Sie müssen für
 * sich stehen: keine Importe, keine Variablen von außen, nur Argumente.
 *
 * Kein innerHTML: YouTube erzwingt Trusted Types. Styles nur über el.style,
 * das ist CSSOM und wird von keiner Seiten-CSP geblockt.
 */

/** Alle sichtbaren Links der Seite (YouTube behält alte Seiten versteckt im DOM). */
export function collectLinks() {
  const out = [];
  const seen = new Set();
  for (const a of document.querySelectorAll('a[href]')) {
    if (!a.getClientRects().length) continue;
    const href = a.href;
    if (!href || seen.has(href)) continue;
    seen.add(href);
    out.push(href);
    if (out.length >= 3000) break;
  }
  return out;
}

/** Links in der Markierung + der markierte Text selbst. */
export function collectSelection() {
  const sel = window.getSelection();
  const links = [];
  if (sel) {
    for (let i = 0; i < sel.rangeCount; i++) {
      const range = sel.getRangeAt(i);
      for (const a of range.cloneContents().querySelectorAll('a[href]')) {
        try {
          links.push(new URL(a.getAttribute('href'), document.baseURI).href);
        } catch (e) {}
      }
      let node = range.commonAncestorContainer;
      if (node && node.nodeType !== 1) node = node.parentElement;
      const outer = node && node.closest ? node.closest('a[href]') : null;
      if (outer) links.push(outer.href);
    }
  }
  return { links, text: sel ? String(sel) : '' };
}

/**
 * Kleine Creme-Einblendung unten rechts.
 * tone: 'ok' | 'same' | 'bad' - undoLabel: Text für "Rückgängig" oder ''
 */
export function showToast(title, detail, tone, undoLabel) {
  const ID = 'dlplist-toast-host';
  const old = document.getElementById(ID);
  if (old) old.remove();

  const host = document.createElement('div');
  host.id = ID;
  const hs = host.style;
  hs.setProperty('all', 'initial');
  hs.setProperty('position', 'fixed');
  hs.setProperty('right', '20px');
  hs.setProperty('bottom', '20px');
  hs.setProperty('z-index', '2147483647');

  const root = host.attachShadow({ mode: 'closed' });
  const box = document.createElement('div');
  Object.assign(box.style, {
    display: 'flex',
    alignItems: 'center',
    gap: '11px',
    padding: '11px 14px 11px 12px',
    background: '#fffdf8',
    color: '#1f1d18',
    border: '1px solid #e7dfd0',
    borderRadius: '14px',
    boxShadow: '0 1px 2px rgba(74,62,40,.06), 0 12px 30px rgba(74,62,40,.16)',
    font: '13px/1.4 -apple-system, "SF Pro Text", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif',
    maxWidth: '380px',
    opacity: '0',
    transform: 'translateY(8px)',
    transition: 'opacity .2s ease, transform .2s ease',
    WebkitFontSmoothing: 'antialiased'
  });

  const colors = { ok: '#34c759', same: '#b5ada0', bad: '#e5484d' };
  const marks = { ok: '✓', same: '=', bad: '!' };
  const dot = document.createElement('div');
  Object.assign(dot.style, {
    width: '22px',
    height: '22px',
    flex: 'none',
    borderRadius: '50%',
    background: colors[tone] || colors.ok,
    color: '#fff',
    display: 'grid',
    placeItems: 'center',
    fontSize: '12px',
    fontWeight: '700'
  });
  dot.textContent = marks[tone] || marks.ok;

  const txt = document.createElement('div');
  Object.assign(txt.style, { minWidth: '0', flex: '1 1 auto' });
  const t1 = document.createElement('div');
  Object.assign(t1.style, { fontWeight: '600', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' });
  t1.textContent = title;
  const t2 = document.createElement('div');
  Object.assign(t2.style, { fontSize: '12px', color: '#7c7466', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' });
  t2.textContent = detail || '';
  txt.append(t1);
  if (detail) txt.append(t2);
  box.append(dot, txt);

  let timer = null;
  const close = () => {
    clearTimeout(timer);
    box.style.opacity = '0';
    box.style.transform = 'translateY(8px)';
    setTimeout(() => host.remove(), 220);
  };

  if (undoLabel) {
    const btn = document.createElement('button');
    Object.assign(btn.style, {
      border: 'none',
      background: '#e8f1fd',
      color: '#0071e3',
      borderRadius: '999px',
      padding: '5px 11px',
      font: 'inherit',
      fontSize: '12px',
      fontWeight: '500',
      cursor: 'pointer',
      flex: 'none'
    });
    btn.textContent = undoLabel;
    btn.addEventListener('click', () => {
      try {
        chrome.runtime.sendMessage({ type: 'undoLast' });
      } catch (e) {}
      close();
    });
    box.append(btn);
  }

  root.append(box);
  (document.body || document.documentElement).append(host);
  requestAnimationFrame(() => {
    box.style.opacity = '1';
    box.style.transform = 'none';
  });
  timer = setTimeout(close, 2800);
  box.addEventListener('mouseenter', () => clearTimeout(timer));
  box.addEventListener('mouseleave', () => {
    timer = setTimeout(close, 1400);
  });
}

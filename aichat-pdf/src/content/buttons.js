/* ai chat exporter - by herrlamatv
 * Kleiner Export-Knopf an jeder Antwort: Maus über eine Antwort -> oben rechts
 * erscheint ⤓ -> Menü „Markdown / PDF / Kopieren“ für genau diese Antwort.
 *
 * Der Knopf hängt NICHT im DOM der Seite (React & Co. mögen fremde Kinder
 * nicht), sondern schwebt in einem eigenen Shadow-Root über der Antwort.
 * Abschaltbar in den Einstellungen („Knöpfe im Chat“).
 */

(() => {
  const AIX = globalThis.AIX;
  if (!AIX || AIX.buttons) return;
  AIX.buttons = true;

  const TEXT = {
    de: { title: 'Diese Antwort exportieren', md: 'Als Markdown', pdf: 'Als PDF', copy: 'Kopieren', copied: 'Antwort kopiert', saving: 'Wird gespeichert …', saved: 'Gespeichert', failed: 'Hat nicht geklappt' },
    en: { title: 'Export this answer', md: 'As Markdown', pdf: 'As PDF', copy: 'Copy', copied: 'Answer copied', saving: 'Saving …', saved: 'Saved', failed: 'Did not work' }
  };

  const ARROW = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v11"/><path d="M7 10.5l5 5 5-5"/><path d="M5 19.5h14"/></svg>';

  let host = null;
  let ui = null;
  let current = null; // { node, role, … } unter der Maus
  let hideTimer = 0;
  let turns = [];
  let byNode = new Map();
  let dirty = true;
  let lastRefresh = 0;

  const enabled = () => AIX.settings.buttons !== false && !AIX.capturing;

  function build() {
    host = document.createElement('div');
    host.setAttribute('data-aix-host', '');
    host.style.cssText = 'position:fixed;top:0;left:0;z-index:2147483646;display:none';
    const root = host.attachShadow({ mode: 'open' });
    root.innerHTML =
      '<style>' +
      ':host{all:initial}' +
      '.btn{width:30px;height:30px;border-radius:999px;border:1px solid #e7dfd0;background:#fffdf8;color:#b98b52;display:grid;place-items:center;cursor:pointer;' +
      'box-shadow:0 1px 2px rgba(74,62,40,.08),0 6px 16px rgba(74,62,40,.12);transition:transform .08s,background .15s;padding:0}' +
      '.btn:hover{background:#faf6ec}.btn:active{transform:scale(.94)}.btn svg{width:17px;height:17px}' +
      '.menu{position:absolute;top:36px;right:0;min-width:160px;background:#fffdf8;border:1px solid #e7dfd0;border-radius:12px;padding:5px;' +
      'box-shadow:0 2px 4px rgba(74,62,40,.05),0 18px 40px rgba(74,62,40,.16);font:500 13px/1.3 -apple-system,"Segoe UI",system-ui,sans-serif;color:#1f1d18}' +
      '.menu[hidden]{display:none}' +
      '.menu button{display:block;width:100%;text-align:left;border:0;background:transparent;font:inherit;color:inherit;padding:8px 10px;border-radius:8px;cursor:pointer}' +
      '.menu button:hover{background:#f3efe5}' +
      '.head{font-size:10.5px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:#a29a8a;padding:6px 10px 4px}' +
      '</style>' +
      '<div style="position:relative">' +
      '<button class="btn" id="btn" type="button">' + ARROW + '</button>' +
      '<div class="menu" id="menu" hidden><div class="head" id="head"></div>' +
      '<button type="button" data-kind="md"></button><button type="button" data-kind="pdf"></button><button type="button" data-kind="copy"></button>' +
      '</div></div>';
    ui = {
      btn: root.getElementById('btn'),
      menu: root.getElementById('menu'),
      head: root.getElementById('head'),
      items: Array.from(root.querySelectorAll('[data-kind]'))
    };
    ui.btn.addEventListener('click', (ev) => {
      ev.stopPropagation();
      labels();
      ui.menu.hidden = !ui.menu.hidden;
    });
    for (const item of ui.items) {
      item.addEventListener('click', (ev) => {
        ev.stopPropagation();
        ui.menu.hidden = true;
        if (current) run(item.dataset.kind, current);
      });
    }
    host.addEventListener('mouseenter', () => clearTimeout(hideTimer));
    document.documentElement.append(host);
  }

  function labels() {
    const L = AIX.L(TEXT);
    ui.btn.title = L.title;
    ui.head.textContent = L.title;
    for (const item of ui.items) item.textContent = L[item.dataset.kind];
  }

  function refresh() {
    const now = Date.now();
    if (!dirty || now - lastRefresh < 600) return;
    lastRefresh = now;
    dirty = false;
    const adapter = AIX.adapter();
    try {
      turns = adapter && adapter.id !== 'universal' ? adapter.turns().filter((t) => t.role === 'assistant') : [];
    } catch (e) {
      turns = [];
    }
    byNode = new Map(turns.map((t) => [t.node, t]));
  }

  function place() {
    if (!current || !current.node.isConnected) return hide(true);
    /* Am Inhalt ausrichten, nicht an der (oft fensterbreiten) Zeile. */
    const adapter = AIX.adapter();
    const body = (adapter && adapter.content && adapter.content(current.node, current.role)) || current.node;
    const r = body.getBoundingClientRect().width ? body.getBoundingClientRect() : current.node.getBoundingClientRect();
    if (r.bottom < 40 || r.top > innerHeight - 40) return hide(true);
    const top = Math.min(Math.max(r.top + 6, 8), innerHeight - 44);
    const left = Math.min(r.right - 38, innerWidth - 46);
    host.style.transform = 'translate(' + Math.max(8, left) + 'px,' + top + 'px)';
  }

  function show(turn) {
    if (!host) build();
    clearTimeout(hideTimer);
    if (current !== turn) {
      current = turn;
      ui.menu.hidden = true;
    }
    labels();
    host.style.display = 'block';
    place();
  }

  function hide(now = false) {
    clearTimeout(hideTimer);
    const go = () => {
      if (!host || !ui.menu.hidden) return;
      host.style.display = 'none';
      current = null;
    };
    if (now) go();
    else hideTimer = setTimeout(go, 900);
  }

  async function run(kind, turn) {
    const L = AIX.L(TEXT);
    const adapter = AIX.adapter();
    if (!adapter) return;
    const msg = AIX.readTurn(adapter, turn);
    if (kind === 'copy') {
      const ok = await AIX.copy(msg.md);
      AIX.toast(ok ? L.copied : L.failed, ok ? 'ok' : 'error');
      return;
    }
    AIX.toast(L.saving, 'busy');
    const raw = {
      platform: adapter.id,
      url: location.href,
      title: (adapter.title && adapter.title()) || document.title,
      id: adapter.chatId ? adapter.chatId() : '',
      dom: [msg],
      api: null
    };
    try {
      const res = await chrome.runtime.sendMessage({ type: 'exportRaw', raw, format: kind === 'pdf' ? 'pdf-styled' : 'md' });
      if (!res || !res.ok) throw new Error((res && res.error) || '');
      AIX.toast(L.saved + (res.data && res.data.filename ? ' · ' + res.data.filename.split('/').pop() : ''));
    } catch (err) {
      AIX.toast(L.failed + (err.message ? ': ' + err.message : ''), 'error');
    }
  }

  new MutationObserver(() => {
    dirty = true;
  }).observe(document.documentElement, { childList: true, subtree: true });

  document.addEventListener(
    'mouseover',
    (ev) => {
      if (!enabled()) return host && hide(true);
      if (host && ev.target === host) return;
      refresh();
      let el = ev.target;
      for (let i = 0; el && i < 60; i++, el = el.parentElement) {
        const turn = byNode.get(el);
        if (turn) return show(turn);
      }
      if (host && host.style.display !== 'none') hide();
    },
    { passive: true }
  );

  document.addEventListener('scroll', () => current && place(), { passive: true, capture: true });
  window.addEventListener('resize', () => current && place(), { passive: true });
  document.addEventListener('click', () => ui && (ui.menu.hidden = true), true);
  AIX.onSettings.push(() => {
    if (!enabled() && host) hide(true);
  });
})();

/* dlplist - by herrlamatv
 * Service Worker: Kontextmenüs, Liste, Badge, Titel-Abruf, Tastenkürzel.
 *
 * Alle Änderungen an der Liste laufen über mutate() - eine Warteschlange,
 * damit schnelle Rechtsklicks sich nicht gegenseitig überschreiben.
 */

import { parseLink, extractUrls, oembedUrl } from './links.js';
import {
  KEYS,
  mergeSettings,
  addParsed,
  removeKeys,
  moveKeys,
  markExported,
  clearItems,
  applyTitles,
  needsTitle,
  counts,
  badgeText
} from './store.js';
import { mergeExport } from './command.js';
import { collectLinks, collectSelection, showToast } from './inject.js';
import de from '../ui/i18n/de.js';
import en from '../ui/i18n/en.js';

/* ------------------------------------------------------------------ */
/* Übersetzung (ohne localStorage - den gibt es im Service Worker nicht) */
/* ------------------------------------------------------------------ */

function translator(pref) {
  let lang = pref;
  if (lang !== 'de' && lang !== 'en') {
    lang = (chrome.i18n.getUILanguage() || '').toLowerCase().startsWith('de') ? 'de' : 'en';
  }
  const bundle = lang === 'de' ? de : en;
  return (key, vars) => {
    let text = bundle[key] ?? en[key] ?? key;
    if (vars) for (const [k, v] of Object.entries(vars)) text = text.split('{' + k + '}').join(String(v));
    return text;
  };
}

/* ------------------------------------------------------------------ */
/* Speicher                                                             */
/* ------------------------------------------------------------------ */

const arr = (v) => (Array.isArray(v) ? v : []);

async function getSettings() {
  const r = await chrome.storage.local.get(KEYS.settings);
  return mergeSettings(r[KEYS.settings]);
}

async function patchSettings(patch) {
  const next = mergeSettings(Object.assign({}, await getSettings(), patch));
  await chrome.storage.local.set({ [KEYS.settings]: next });
  return next;
}

async function getItems() {
  const r = await chrome.storage.local.get(KEYS.items);
  return arr(r[KEYS.items]);
}

let queue = Promise.resolve();

/** fn(items) → { items, … } - schreibt nur, wenn sich die Liste geändert hat. */
function mutate(fn) {
  const run = queue.then(async () => {
    const items = await getItems();
    const res = (await fn(items)) || {};
    if (res.items && res.items !== items) await chrome.storage.local.set({ [KEYS.items]: res.items });
    return res;
  });
  queue = run.catch(() => {});
  return run;
}

/* ------------------------------------------------------------------ */
/* Hinzufügen                                                           */
/* ------------------------------------------------------------------ */

async function addUrls(urls, source) {
  const parsed = [];
  let invalid = 0;
  for (const u of urls) {
    if (!u) continue;
    const p = parseLink(u);
    if (p.ok) parsed.push(p);
    else invalid++;
  }
  const res = await mutate((items) => addParsed(items, parsed, Date.now(), source));
  if (res.added.length) chrome.storage.session.set({ lastBatch: res.added }).catch(() => {});
  if (res.added.length) fillTitlesSoon();
  const touched = res.added.concat(res.moved, res.dupes);
  const first = res.items.find((it) => it.key === touched[0]);
  return {
    added: res.added.length,
    moved: res.moved.length,
    dupes: res.dupes.length,
    invalid,
    cat: first ? first.cat : null,
    counts: counts(res.items)
  };
}

function toastMessage(res, tr) {
  const total = res.added + res.moved + res.dupes;
  const inCat = res.cat ? tr('toast.inCat', { cat: tr('cat.' + res.cat), n: res.counts[res.cat] }) : '';
  if (!total) return { tone: 'bad', title: tr('toast.nothing'), detail: tr('toast.nothingDetail') };
  if (total === 1) {
    if (res.added) return { tone: 'ok', title: tr('toast.added'), detail: inCat };
    if (res.moved) return { tone: 'ok', title: tr('toast.moved'), detail: inCat };
    return { tone: 'same', title: tr('toast.dupe'), detail: inCat };
  }
  if (!res.added && !res.moved) return { tone: 'same', title: tr('toast.dupesOnly', { n: res.dupes }), detail: '' };
  const parts = [];
  if (res.moved) parts.push(tr('toast.movedMany', { n: res.moved }));
  if (res.dupes) parts.push(tr('toast.dupesMany', { n: res.dupes }));
  return { tone: 'ok', title: tr('toast.addedMany', { n: res.added }), detail: parts.join(' · ') || tr('toast.total', { n: res.counts.total }) };
}

async function notify(tab, res, tr, settings) {
  const msg = toastMessage(res, tr);
  if (settings.toast && tab && tab.id >= 0) {
    try {
      await chrome.scripting.executeScript({
        target: { tabId: tab.id },
        func: showToast,
        args: [msg.title, msg.detail, msg.tone, res.added ? tr('toast.undo') : '']
      });
      return;
    } catch (e) {
      // chrome://, Web Store, PDF … - dann eben über das Badge
    }
  }
  flashBadge(msg.tone === 'ok' ? '+' + (res.added + res.moved) : msg.tone === 'same' ? '=' : '!');
}

async function inTab(tab, frameId, func) {
  if (!tab || tab.id < 0) return null;
  try {
    const target = { tabId: tab.id };
    if (frameId) target.frameIds = [frameId];
    const [first] = await chrome.scripting.executeScript({ target, func });
    return first ? first.result : null;
  } catch (e) {
    return null;
  }
}

async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
  return tab || null;
}

async function addTab(tab, { toast = true } = {}) {
  const settings = await getSettings();
  const tr = translator(settings.language);
  const target = tab || (await activeTab());
  if (!settings.enabled) {
    if (toast && target) await notifyOff(target, tr);
    return { off: true };
  }
  const res = await addUrls([target && target.url], 'tab');
  if (toast) await notify(target, res, tr, settings);
  return res;
}

async function notifyOff(tab, tr) {
  try {
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: showToast,
      args: [tr('toast.off'), tr('toast.offDetail'), 'bad', '']
    });
  } catch (e) {}
}

/* ------------------------------------------------------------------ */
/* Kontextmenüs                                                         */
/* ------------------------------------------------------------------ */

let menuChain = Promise.resolve();

/** Menüs neu aufbauen - hintereinander, nie parallel (sonst doppelte IDs). */
function setupMenus() {
  menuChain = menuChain.then(buildMenus).catch((e) => console.warn('[dlplist] menus', e));
  return menuChain;
}

async function buildMenus() {
  await new Promise((resolve) =>
    chrome.contextMenus.removeAll(() => {
      void chrome.runtime.lastError;
      resolve();
    })
  );
  const settings = await getSettings();
  const tr = translator(settings.language);
  const make = (props) =>
    new Promise((resolve) =>
      chrome.contextMenus.create(props, () => {
        void chrome.runtime.lastError;
        resolve();
      })
    );

  await make({ id: 'open-export', title: tr('menu.openExport'), contexts: ['action'] });
  if (!settings.enabled) return;
  await make({ id: 'add-tab', title: tr('menu.addTab'), contexts: ['action'] });
  await make({ id: 'add-link', title: tr('menu.addLink'), contexts: ['link'], targetUrlPatterns: ['http://*/*', 'https://*/*'] });
  await make({ id: 'add-page', title: tr('menu.addPage'), contexts: ['page'], documentUrlPatterns: ['http://*/*', 'https://*/*'] });
  await make({ id: 'add-all', title: tr('menu.addAll'), contexts: ['page'], documentUrlPatterns: ['*://*.youtube.com/*'] });
  await make({ id: 'add-selection', title: tr('menu.addSelection'), contexts: ['selection'] });
}

chrome.contextMenus.onClicked.addListener((info, tab) => {
  onMenu(info, tab).catch((e) => console.warn('[dlplist]', e));
});

async function onMenu(info, tab) {
  if (info.menuItemId === 'open-export') return chrome.runtime.openOptionsPage();
  const settings = await getSettings();
  if (!settings.enabled) return;
  const tr = translator(settings.language);
  let urls = [];
  let source = 'link';

  switch (info.menuItemId) {
    case 'add-link':
      urls = [info.linkUrl];
      break;
    case 'add-page':
    case 'add-tab':
      urls = [info.pageUrl || (tab && tab.url)];
      source = 'page';
      break;
    case 'add-all': {
      const links = arr(await inTab(tab, 0, collectLinks));
      urls = links.filter((u) => {
        const p = parseLink(u);
        return p.ok && (p.cat === 'videos' || p.cat === 'shorts');
      });
      source = 'page';
      break;
    }
    case 'add-selection': {
      const sel = await inTab(tab, info.frameId, collectSelection);
      urls = sel ? arr(sel.links).concat(extractUrls(sel.text)) : [];
      if (!urls.length) urls = extractUrls(info.selectionText);
      source = 'selection';
      break;
    }
    default:
      return;
  }
  const res = await addUrls(urls, source);
  await notify(tab, res, tr, settings);
}

chrome.commands.onCommand.addListener((command, tab) => {
  if (command === 'add-current-tab') addTab(tab).catch((e) => console.warn('[dlplist]', e));
});

/* ------------------------------------------------------------------ */
/* Badge                                                                */
/* ------------------------------------------------------------------ */

let flashTimer = null;

async function updateBadge() {
  const [settings, items] = await Promise.all([getSettings(), getItems()]);
  const tr = translator(settings.language);
  const c = counts(items);
  await chrome.action.setBadgeText({ text: settings.badge ? badgeText(c.pending) : '' });
  await chrome.action.setBadgeBackgroundColor({ color: settings.enabled ? '#0071e3' : '#b5ada0' });
  await chrome.action.setBadgeTextColor({ color: '#ffffff' }).catch(() => {});
  await chrome.action.setTitle({ title: tr(settings.enabled ? 'action.title' : 'action.titleOff', { n: c.pending }) });
}

function flashBadge(text) {
  clearTimeout(flashTimer);
  chrome.action.setBadgeText({ text }).catch(() => {});
  flashTimer = setTimeout(() => updateBadge().catch(() => {}), 1600);
}

/* ------------------------------------------------------------------ */
/* Titel über oEmbed (und nebenbei: tote Links erkennen)                */
/* ------------------------------------------------------------------ */

let filling = null;

function fillTitlesSoon() {
  if (!filling) filling = fillTitles().catch((e) => console.warn('[dlplist] titles', e)).finally(() => (filling = null));
  return filling;
}

async function fillTitles() {
  if (!(await getSettings()).titles) return;
  // kleine Runden: jede schreibt in den Speicher und hält den Worker wach
  for (let round = 0; round < 200; round++) {
    const todo = needsTitle(await getItems(), (it) => !!oembedUrl(it)).slice(0, 9);
    if (!todo.length) return;
    const results = await pool(todo, 3, fetchTitle);
    await mutate((items) => ({ items: applyTitles(items, results) }));
  }
}

async function pool(list, size, fn) {
  const out = [];
  let i = 0;
  const worker = async () => {
    while (i < list.length) out.push(await fn(list[i++]));
  };
  await Promise.all(Array.from({ length: Math.min(size, list.length) }, worker));
  return out;
}

async function fetchTitle(item) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), 5000);
  try {
    const r = await fetch(oembedUrl(item), { signal: ctrl.signal, credentials: 'omit' });
    if (r.ok) {
      const j = await r.json();
      return { key: item.key, title: j.title || null, author: j.author_name || null };
    }
    // 401 = Einbetten verboten, das Video gibt es aber - nur 400/404 heißt "weg?"
    if (r.status === 400 || r.status === 404) return { key: item.key, status: 'unavailable' };
    return { key: item.key };
  } catch (e) {
    return { key: item.key };
  } finally {
    clearTimeout(timer);
  }
}

/* ------------------------------------------------------------------ */
/* Nachrichten von Popup, Export-Seite und Toast                        */
/* ------------------------------------------------------------------ */

const handlers = {
  async getState() {
    const r = await chrome.storage.local.get(Object.values(KEYS));
    const items = arr(r[KEYS.items]);
    return {
      settings: mergeSettings(r[KEYS.settings]),
      items,
      counts: counts(items),
      export: mergeExport(r[KEYS.exp]),
      presets: arr(r[KEYS.presets]),
      history: arr(r[KEYS.history])
    };
  },
  async setEnabled({ value }) {
    return patchSettings({ enabled: !!value });
  },
  async saveSettings({ patch }) {
    return patchSettings(patch || {});
  },
  async addUrls({ urls, source }) {
    return addUrls(arr(urls).slice(0, 5000), source || 'paste');
  },
  async addTab() {
    return addTab(null, { toast: false });
  },
  async removeItems({ keys }) {
    await mutate((items) => ({ items: removeKeys(items, arr(keys)) }));
    return true;
  },
  async moveItems({ keys, cat }) {
    await mutate((items) => ({ items: moveKeys(items, arr(keys), cat) }));
    return true;
  },
  async clearItems({ filter }) {
    await mutate((items) => ({ items: clearItems(items, filter) }));
    return true;
  },
  async markExported({ keys, at }) {
    await mutate((items) => ({ items: markExported(items, arr(keys), at) }));
    return true;
  },
  async undoLast() {
    const s = await chrome.storage.session.get('lastBatch');
    const keys = arr(s.lastBatch);
    if (!keys.length) return { removed: 0 };
    await mutate((items) => ({ items: removeKeys(items, keys) }));
    await chrome.storage.session.remove('lastBatch');
    return { removed: keys.length };
  },
  async fillTitles({ retry }) {
    if (retry) {
      await mutate((items) => ({
        items: items.map((it) => (it.title == null && it.titleTried ? Object.assign({}, it, { titleTried: false, status: null }) : it))
      }));
    }
    fillTitlesSoon();
    return true;
  }
};

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  const handler = msg && handlers[msg.type];
  if (!handler) return false;
  Promise.resolve(handler(msg))
    .then((data) => sendResponse({ ok: true, data }))
    .catch((err) => sendResponse({ ok: false, error: String((err && err.message) || err) }));
  return true; // asynchrone Antwort
});

/* ------------------------------------------------------------------ */
/* Lebenszyklus                                                         */
/* ------------------------------------------------------------------ */

chrome.runtime.onInstalled.addListener((details) => {
  setupMenus();
  updateBadge().catch(() => {});
  if (details.reason === 'install') chrome.runtime.openOptionsPage();
});

chrome.runtime.onStartup.addListener(() => {
  setupMenus();
  updateBadge().catch(() => {});
  fillTitlesSoon();
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== 'local') return;
  const s = changes[KEYS.settings];
  if (s) {
    const before = mergeSettings(s.oldValue);
    const after = mergeSettings(s.newValue);
    if (before.enabled !== after.enabled || before.language !== after.language) setupMenus();
    if (after.titles && !before.titles) fillTitlesSoon();
  }
  if (s || changes[KEYS.items]) updateBadge().catch(() => {});
});

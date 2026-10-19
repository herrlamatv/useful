/* ai chat exporter - by herrlamatv
 * Service Worker: holt den Chat aus dem Tab (Content-Script), baut daraus die
 * gewünschte Datei und lädt sie herunter. PDFs entstehen auf ui/render.html.
 * Dazu Kontextmenü, Tastenkürzel und Screenshot-Aufnahmen.
 */

import { loadSettings, saveSettings, setPath, DEFAULT_SETTINGS } from './settings.js';
import { platformFor, scriptFiles, ALL_MATCHES } from './platforms.js';
import { buildConversation, filterConversation, imageUrls, replaceImageUrls, summarize } from './model.js';
import { toMarkdown } from './export/markdown.js';
import { toText } from './export/text.js';
import { toJson } from './export/json.js';
import { toHtml } from './export/html.js';
import { buildPath } from './export/filename.js';
import { t, setLanguage, language, outputLabels } from '../ui/i18n.js';

const EXT = { md: 'md', txt: 'txt', json: 'json', html: 'html', 'pdf-styled': 'pdf', 'pdf-raw': 'pdf', 'pdf-shot': 'pdf' };
const MIME = { md: 'text/markdown', txt: 'text/plain', json: 'application/json', html: 'text/html' };
const MENU_FORMATS = ['md', 'pdf-styled', 'pdf-raw', 'pdf-shot', 'html', 'txt', 'json', 'copy'];

const jobs = new Map(); // PDF-Aufträge für ui/render.html
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/* ----------------------------- Hilfen -------------------------------- */

async function settingsNow() {
  const settings = await loadSettings();
  setLanguage(settings.language);
  return settings;
}

function errorText(err) {
  const raw = String((err && err.message) || err);
  const known = t('error.' + raw);
  return known === 'error.' + raw ? t('error.generic', { message: raw }) : known;
}

async function tabMessage(tabId, msg) {
  const res = await chrome.tabs.sendMessage(tabId, msg);
  if (!res) throw new Error('no-response');
  if (!res.ok) throw new Error(res.error || 'no-response');
  return res.data;
}

/**
 * Content-Scripts sicherstellen. Tabs, die schon vor der Installation offen
 * waren, haben noch keine - dann jetzt nachladen. Passt kein Adapter
 * (z. B. x.com außerhalb von Grok), kommt der Universal-Modus dazu.
 */
async function ensureInjected(tab) {
  const platform = platformFor(tab.url || '');
  if (!platform) throw new Error('unsupported-page');
  const ping = async () => {
    try {
      return (await tabMessage(tab.id, { type: 'aix:ping' })).platform;
    } catch (e) {
      return null;
    }
  };
  let id = await ping();
  if (id) return id;
  try {
    await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: scriptFiles(platform.id) });
    id = await ping();
    if (!id && platform.id !== 'universal') {
      await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: scriptFiles('universal') });
      id = await ping();
    }
  } catch (e) {
    throw new Error('inject-failed');
  }
  if (!id) throw new Error('no-adapter');
  return id;
}

async function extract(tab, { loadAll = true, api = true } = {}) {
  await ensureInjected(tab);
  const raw = await tabMessage(tab.id, { type: 'aix:extract', loadAll, api });
  const conv = buildConversation(raw);
  if (raw.apiError) conv.apiError = raw.apiError;
  return conv;
}

function badge(tabId, text) {
  chrome.action.setBadgeBackgroundColor({ color: '#b98b52' }).catch(() => {});
  chrome.action.setBadgeText({ tabId, text }).catch(() => {});
}

function notify(tab, text, kind = 'ok') {
  if (tab && tab.id) tabMessage(tab.id, { type: 'aix:toast', text, kind }).catch(() => {});
}

/** Text -> data:-URL (UTF-8, base64). Im Service Worker gibt es kein URL.createObjectURL. */
function dataUrl(text, mime) {
  const bytes = new TextEncoder().encode(text);
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return 'data:' + mime + ';charset=utf-8;base64,' + btoa(bin);
}

async function saveText({ text, filename, mime }, settings) {
  await chrome.downloads.download({ url: dataUrl(text, mime), filename, saveAs: settings.saveAs, conflictAction: 'uniquify' });
  return { filename };
}

/** Bilder als data:-URL einbetten (PDF/HTML sollen ohne Internet und ohne Login funktionieren). */
async function embedImages(tab, conv, settings) {
  if (!tab || settings.include.images === false) return conv;
  const urls = imageUrls(conv);
  if (!urls.length) return conv;
  try {
    const { map } = await tabMessage(tab.id, { type: 'aix:images', urls });
    return replaceImageUrls(conv, map);
  } catch (e) {
    return conv;
  }
}

function outputOptions(settings, now) {
  return {
    labels: outputLabels(),
    include: settings.include,
    exportedAt: now,
    lang: language(),
    fontSize: settings.fontSize,
    paper: settings.paper,
    frontmatter: settings.frontmatter,
    tags: settings.tags
  };
}

function renderText(conv, format, settings, now = new Date()) {
  const opts = outputOptions(settings, now);
  if (format === 'md') return toMarkdown(conv, opts);
  if (format === 'txt') return toText(conv, opts);
  if (format === 'json') return toJson(conv, opts);
  if (format === 'html') return toHtml(conv, Object.assign({ mode: 'styled' }, opts));
  throw new Error('unknown-format');
}

/* ------------------------------- PDF --------------------------------- */

async function startPdf(job) {
  const id = crypto.randomUUID();
  jobs.set(id, job);
  setTimeout(() => jobs.delete(id), 20 * 60 * 1000);
  await chrome.tabs.create({ url: chrome.runtime.getURL('ui/render.html?job=' + id), active: job.method === 'print' });
  return { filename: job.filename, pdf: job.method };
}

/** 1:1-Screenshot: Chat Stück für Stück aufnehmen (höchstens 2 Aufnahmen pro Sekunde erlaubt). */
async function screenshotPdf(tab, settings) {
  await ensureInjected(tab);
  if (!tab.active) throw new Error('tab-not-visible');
  await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: ['src/content/screenshot.js'] });
  const prep = await tabMessage(tab.id, { type: 'aix:shot:prep', loadAll: true });
  const shots = [];
  let last = 0;
  let y = 0;
  try {
    for (let i = 0; i < 400; i++) {
      const pos = await tabMessage(tab.id, { type: 'aix:shot:scroll', y });
      const wait = 560 - (Date.now() - last);
      if (wait > 0) await sleep(wait);
      const shot = await chrome.tabs.captureVisibleTab(tab.windowId, { format: 'jpeg', quality: 90 });
      last = Date.now();
      shots.push({ top: pos.top, dataUrl: shot });
      badge(tab.id, Math.min(99, Math.round(((pos.top + pos.clientHeight) / Math.max(1, pos.scrollHeight)) * 100)) + '%');
      if (pos.top + pos.clientHeight >= pos.scrollHeight - 2) break;
      const next = pos.top + pos.clientHeight;
      if (next <= pos.top) break;
      y = next;
    }
  } finally {
    await tabMessage(tab.id, { type: 'aix:shot:done' }).catch(() => {});
    badge(tab.id, '');
  }
  const platform = platformFor(tab.url);
  const conv = { platform: platform.id, platformName: platform.name, title: (tab.title || 'Chat').replace(/\s*[-|]\s*[^-|]{2,24}$/, '') };
  const filename = buildPath(settings, conv, 'pdf');
  return startPdf({
    mode: 'shot',
    shots,
    rect: prep.rect,
    viewport: prep.viewport,
    method: settings.pdfMethod,
    paper: settings.paper,
    saveAs: settings.saveAs,
    filename,
    title: conv.title,
    lang: language()
  });
}

/* ------------------------------ Export ------------------------------- */

/**
 * Eine Unterhaltung exportieren.
 * opts: format, keys (Auswahl), roles ('all'|'user'|'assistant'), include (überschreibt Einstellungen),
 *       conv (fertige Unterhaltung, z. B. eine einzelne Antwort)
 */
async function exportConversation(tab, { format, keys = null, roles = 'all', include = null, conv: given = null }) {
  const settings = await settingsNow();
  if (include) settings.include = Object.assign({}, settings.include, include);
  if (format === 'pdf-shot') return screenshotPdf(tab, settings);

  let conv = given || (await extract(tab, { loadAll: true, api: true }));
  conv = filterConversation(conv, { keys, roles, include: settings.include });
  if (!conv.messages.length) throw new Error('empty');
  const now = new Date();
  const filename = buildPath(settings, conv, EXT[format], now);

  if (format === 'pdf-styled' || format === 'pdf-raw') {
    if (format === 'pdf-styled') conv = await embedImages(tab, conv, settings);
    return startPdf({
      mode: format === 'pdf-raw' ? 'raw' : 'styled',
      conv,
      labels: outputLabels(),
      include: settings.include,
      exportedAt: now.toISOString(),
      lang: language(),
      fontSize: settings.fontSize,
      paper: settings.paper,
      method: settings.pdfMethod,
      saveAs: settings.saveAs,
      filename,
      title: conv.title
    });
  }
  if (format === 'html') conv = await embedImages(tab, conv, settings);
  return saveText({ text: renderText(conv, format, settings, now), filename, mime: MIME[format] }, settings);
}

/** Markdown für die Zwischenablage (ohne Frontmatter). */
async function markdownFor(tab, { keys = null, roles = 'all', include = null } = {}) {
  const settings = await settingsNow();
  if (include) settings.include = Object.assign({}, settings.include, include);
  const conv = filterConversation(await extract(tab, { loadAll: true, api: true }), { keys, roles, include: settings.include });
  if (!conv.messages.length) throw new Error('empty');
  return renderText(conv, 'md', Object.assign({}, settings, { frontmatter: false }));
}

/** Schnell-Export (Tastenkürzel, Kontextmenü) - Rückmeldung als Hinweis in der Seite. */
async function quickExport(tab, format) {
  if (!tab) return;
  const settings = await settingsNow();
  const fmt = format || settings.format;
  try {
    if (fmt === 'copy') {
      const text = await markdownFor(tab);
      const { copied } = await tabMessage(tab.id, { type: 'aix:copy', text });
      notify(tab, copied ? t('status.copied') : t('error.copy-failed'), copied ? 'ok' : 'error');
      return;
    }
    notify(tab, fmt === 'pdf-shot' ? t('status.shot') : t('status.working'), 'busy');
    const res = await exportConversation(tab, { format: fmt });
    if (!res.pdf) notify(tab, t('status.saved', { file: res.filename.split('/').pop() }));
  } catch (err) {
    notify(tab, errorText(err), 'error');
  }
}

/* --------------------------- Kontextmenü ----------------------------- */

async function buildMenus() {
  await chrome.contextMenus.removeAll();
  await settingsNow();
  const contexts = ['page', 'selection', 'link', 'image'];
  chrome.contextMenus.create({ id: 'aix', title: t('menu.root'), contexts, documentUrlPatterns: ALL_MATCHES });
  for (const f of MENU_FORMATS) {
    chrome.contextMenus.create({ id: 'aix:' + f, parentId: 'aix', title: t('format.' + f), contexts, documentUrlPatterns: ALL_MATCHES });
  }
}

chrome.runtime.onInstalled.addListener(() => buildMenus().catch(() => {}));
chrome.runtime.onStartup.addListener(() => buildMenus().catch(() => {}));

chrome.contextMenus.onClicked.addListener((info, tab) => {
  const id = String(info.menuItemId);
  if (id.startsWith('aix:')) quickExport(tab, id.slice(4));
});

chrome.commands.onCommand.addListener(async (command, tab) => {
  if (command !== 'quick-export') return;
  const target = tab && tab.id ? tab : (await chrome.tabs.query({ active: true, currentWindow: true }))[0];
  quickExport(target, null);
});

/* ----------------------------- Nachrichten --------------------------- */

const getTab = async (tabId) => chrome.tabs.get(Number(tabId));
/* Popup/Bulk geben die Tab-ID mit, Content-Scripts meinen ihren eigenen Tab. */
const tabFor = (msg, sender) => (msg.tabId ? getTab(msg.tabId) : sender.tab);

const HANDLERS = {
  async getSettings() {
    return loadSettings();
  },

  async setOption({ key, value }) {
    const next = await saveSettings(setPath(await loadSettings(), key, value));
    if (key === 'language') await buildMenus().catch(() => {});
    return next;
  },

  async resetSettings() {
    const next = await saveSettings(DEFAULT_SETTINGS);
    await buildMenus().catch(() => {});
    return next;
  },

  async describeTab({ tabId }) {
    const tab = await getTab(tabId);
    const platform = platformFor(tab.url || '');
    return { tab: { id: tab.id, url: tab.url, title: tab.title }, platform };
  },

  /** Popup: Chat lesen (ohne Nachladen), Kurzfassung zurück. */
  async scan(msg, sender) {
    const conv = await extract(await tabFor(msg, sender), { loadAll: false, api: true });
    return Object.assign(summarize(conv), { apiError: conv.apiError || null });
  },

  async export(msg, sender) {
    const { format, keys, roles, include } = msg;
    return exportConversation(await tabFor(msg, sender), { format, keys, roles, include });
  },

  async markdown(msg, sender) {
    const { keys, roles, include } = msg;
    return markdownFor(await tabFor(msg, sender), { keys, roles, include });
  },

  async pick({ tabId }) {
    const tab = await getTab(tabId);
    await ensureInjected(tab);
    await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: scriptFiles('universal') });
    return tabMessage(tab.id, { type: 'aix:pick' });
  },

  async ensure({ tabId }) {
    return ensureInjected(await getTab(tabId));
  },

  /** Knopf an einer Antwort: die Nachricht kommt schon fertig gelesen mit. */
  async exportRaw({ raw, format }, sender) {
    const conv = buildConversation(raw);
    return exportConversation(sender.tab, { format, conv });
  },

  async openBulk({ tabId }) {
    await chrome.tabs.create({ url: chrome.runtime.getURL('ui/bulk.html?tab=' + tabId) });
    return {};
  },

  /* PDF-Seite */
  async 'pdf:job'({ id }) {
    const job = jobs.get(id);
    if (!job) throw new Error('job-missing');
    if (job.mode !== 'shot') return job;
    return Object.assign({}, job, { shots: job.shots.map((s) => ({ top: s.top })) });
  },

  async 'pdf:shot'({ id, index }) {
    const job = jobs.get(id);
    if (!job || !job.shots || !job.shots[index]) throw new Error('job-missing');
    return job.shots[index].dataUrl;
  },

  async 'pdf:done'({ id }) {
    jobs.delete(id);
    return {};
  }
};

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  const handler = msg && HANDLERS[msg.type];
  if (!handler) return false;
  Promise.resolve()
    .then(() => handler(msg, sender))
    .then(
      (data) => sendResponse({ ok: true, data }),
      (err) => sendResponse({ ok: false, error: String((err && err.message) || err) })
    );
  return true;
});

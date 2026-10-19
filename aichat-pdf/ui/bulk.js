/* ai chat exporter - by herrlamatv
 *
 * Läuft in einem eigenen Tab (das Popup schließt sich, sobald man woanders
 * hinklickt). Die Daten holt das Content-Script im offenen Chat-Tab - mit
 * deiner Anmeldung - über die interne API der Seite, eine Unterhaltung nach
 * der anderen. Ziel: ZIP in Downloads oder direkt ein Ordner (z. B. Obsidian).
 */

import { send, sendTab, el, errorText, check, segmented, toggleRow, languageSwitch, downloadBlob } from './common.js';
import { t, setLanguage, applyStatic, languagePreference, outputLabels, language } from './i18n.js';
import { platformFor } from '../src/platforms.js';
import { buildConversation, filterConversation } from '../src/model.js';
import { toMarkdown } from '../src/export/markdown.js';
import { toText } from '../src/export/text.js';
import { toJson } from '../src/export/json.js';
import { toHtml } from '../src/export/html.js';
import { buildPath, isoDate, sanitizeSegment } from '../src/export/filename.js';
import { ZipWriter } from '../src/zip.js';

const $ = (id) => document.getElementById(id);
const tabId = Number(new URLSearchParams(location.search).get('tab'));
const DONE_KEY = 'aichat.bulk'; // { [plattform]: { [chatId]: updated } }
const FORMATS = ['md', 'json', 'html', 'txt'];
const CHAT_URL = { chatgpt: (id) => 'https://chatgpt.com/c/' + id, claude: (id) => 'https://claude.ai/chat/' + id };
const DELAY = { chatgpt: 900, claude: 450 };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const state = {
  settings: null,
  platform: null,
  items: [],
  selected: new Set(),
  done: {},
  search: '',
  since: '',
  onlyNew: false,
  formats: new Set(['md']),
  dest: 'zip',
  run: null
};

applyStatic();

/* ------------------------------ Liste --------------------------------- */

const changed = (item) => !state.done[item.id] || (item.updated && item.updated > state.done[item.id]);

function visibleItems() {
  const q = state.search.trim().toLowerCase();
  return state.items.filter(
    (item) =>
      (!q || (item.title || '').toLowerCase().includes(q)) &&
      (!state.since || (item.updated || item.created || '') >= state.since) &&
      (!state.onlyNew || changed(item))
  );
}

const chosen = () => visibleItems().filter((item) => state.selected.has(item.id));

function renderList() {
  const items = visibleItems();
  $('selCount').textContent = t('bulk.selected', { n: chosen().length }) + ' · ' + t('bulk.count', { n: items.length });
  $('chatList').replaceChildren(
    ...items.slice(0, 3000).map((item) => {
      const on = state.selected.has(item.id);
      const flip = (value) => {
        if (value) state.selected.add(item.id);
        else state.selected.delete(item.id);
        renderList();
      };
      return el('div', { class: 'chat-row', onclick: () => flip(!on) }, [
        check(on, flip),
        el('div', { class: 't', title: item.title }, item.title || '—'),
        changed(item) && state.done[item.id] ? el('span', { class: 'chip on' }, '↻') : null,
        el('div', { class: 'd' }, isoDate(item.updated || item.created || ''))
      ]);
    })
  );
  $('onlyNewRow').replaceChildren(
    toggleRow(t('bulk.onlyNew'), null, state.onlyNew, (v) => {
      state.onlyNew = v;
      renderList();
    })
  );
}

async function loadList() {
  $('load').disabled = true;
  state.items = [];
  try {
    await send('ensure', { tabId });
    let offset = 0;
    for (let guard = 0; guard < 500; guard++) {
      $('loadStatus').textContent = t('bulk.loading', { n: state.items.length });
      const page = await sendTab(tabId, 'aix:bulk:list', { offset });
      state.items.push(...page.items);
      if (page.next === null || page.next === undefined || !page.items.length) break;
      offset = page.next;
      await sleep(250);
    }
    state.selected = new Set(state.items.map((i) => i.id));
    $('loadStatus').textContent = t('bulk.count', { n: state.items.length });
    $('filterBody').hidden = false;
    renderList();
  } catch (err) {
    $('loadStatus').textContent = err.message === 'no-response' ? t('bulk.noTab') : errorText(err);
  } finally {
    $('load').disabled = false;
  }
}

/* ------------------------------ Export -------------------------------- */

function renderExportPanel() {
  $('formatChips').replaceChildren(
    ...FORMATS.map((f) =>
      el(
        'button',
        {
          class: 'tchip',
          type: 'button',
          'aria-pressed': String(state.formats.has(f)),
          onclick: () => {
            if (state.formats.has(f) && state.formats.size > 1) state.formats.delete(f);
            else state.formats.add(f);
            renderExportPanel();
          }
        },
        t('format.' + f)
      )
    )
  );
  $('destSeg').replaceChildren(
    segmented(
      [
        ['zip', t('dest.zip')],
        ['folder', t('dest.folder')]
      ],
      state.dest,
      (v) => {
        state.dest = v;
        renderExportPanel();
      }
    )
  );
  $('destDesc').textContent = state.dest === 'folder' ? t('dest.folderDesc') : 'Downloads/AI-Chats/…zip';
}

/** Ordner-Ziel (File System Access) - Unterordner werden bei Bedarf angelegt. */
function folderWriter(base) {
  const dirs = new Map();
  const dirFor = (segs) => {
    const key = segs.join('/');
    if (!dirs.has(key)) {
      dirs.set(key, segs.length ? dirFor(segs.slice(0, -1)).then((d) => d.getDirectoryHandle(segs[segs.length - 1], { create: true })) : Promise.resolve(base));
    }
    return dirs.get(key);
  };
  return async (path, text) => {
    const segs = path.split('/');
    const dir = await dirFor(segs.slice(0, -1));
    const writable = await (await dir.getFileHandle(segs[segs.length - 1], { create: true })).createWritable();
    try {
      await writable.write(new Blob([text], { type: 'text/plain' }));
      await writable.close();
    } catch (err) {
      await writable.abort().catch(() => {});
      throw err;
    }
  };
}

function renderFile(conv, format, now) {
  const opts = {
    labels: outputLabels(),
    include: state.settings.include,
    exportedAt: now,
    lang: language(),
    fontSize: state.settings.fontSize,
    paper: state.settings.paper,
    frontmatter: state.settings.frontmatter,
    tags: state.settings.tags
  };
  if (format === 'md') return toMarkdown(conv, opts);
  if (format === 'txt') return toText(conv, opts);
  if (format === 'json') return toJson(conv, opts);
  return toHtml(conv, Object.assign({ mode: 'styled' }, opts));
}

/** Eine Unterhaltung holen - bei 429 (zu viele Anfragen) warten und neu versuchen. */
async function fetchOne(id, run) {
  for (let attempt = 1; attempt <= 4; attempt++) {
    try {
      return await sendTab(tabId, 'aix:bulk:get', { id });
    } catch (err) {
      if (err.message !== 'http-429' || attempt === 4) throw err;
      for (let s = 20 * attempt; s > 0 && !run.stop; s--) {
        $('progress').textContent = t('bulk.rate', { s });
        await sleep(1000);
      }
    }
  }
  return null;
}

function setProgress(run) {
  const pct = run.total ? Math.round((run.done / run.total) * 100) : 0;
  $('bar').querySelector('span').style.width = pct + '%';
  $('progress').textContent = t('bulk.progress', { done: run.done, total: run.total });
}

async function start() {
  if (state.run) return;
  const items = chosen();
  $('result').hidden = true;
  $('log').replaceChildren();
  if (!items.length) {
    $('result').hidden = false;
    $('result').textContent = t('bulk.nothing');
    return;
  }

  /* Ordner zuerst - der Dialog muss direkt aus dem Klick kommen. */
  let write;
  let zip = null;
  let target;
  const platformName = state.platform.name;
  if (state.dest === 'folder') {
    if (typeof window.showDirectoryPicker !== 'function') {
      $('result').hidden = false;
      $('result').textContent = t('bulk.noFolderApi');
      return;
    }
    let dir;
    try {
      dir = await window.showDirectoryPicker({ id: 'aichat-export', mode: 'readwrite', startIn: 'documents' });
    } catch (err) {
      return; // abgebrochen
    }
    write = folderWriter(dir);
    target = dir.name + '/' + sanitizeSegment(platformName);
  } else {
    zip = new ZipWriter();
    write = (path, text) => zip.add(path, text);
  }

  const run = { total: items.length, done: 0, failed: [], stop: false, paused: false, resume: null };
  state.run = run;
  $('start').disabled = true;
  $('runBox').hidden = false;
  $('pause').textContent = t('bulk.pause');
  $('bar').className = 'bar';
  setProgress(run);

  const now = new Date();
  const used = new Set();
  for (const item of items) {
    if (run.paused) await new Promise((r) => (run.resume = r));
    if (run.stop) break;
    try {
      const api = await fetchOne(item.id, run);
      if (run.stop) break;
      let conv = buildConversation({
        platform: state.platform.id,
        url: (CHAT_URL[state.platform.id] || String)(item.id),
        title: item.title,
        id: item.id,
        dom: [],
        api
      });
      conv = filterConversation(conv, { include: state.settings.include });
      for (const format of FORMATS.filter((f) => state.formats.has(f))) {
        let path = sanitizeSegment(platformName) + '/' + buildPath({ folder: '', file: state.settings.file }, conv, format, now);
        if (used.has(path.toLowerCase())) path = path.replace(/(\.\w+)$/, ' (' + item.id.slice(0, 8) + ')$1');
        used.add(path.toLowerCase());
        await write(path, renderFile(conv, format, now));
      }
      state.done[item.id] = item.updated || now.toISOString();
    } catch (err) {
      if (err.message === 'no-response') {
        run.stop = true;
        $('log').append(el('div', {}, t('bulk.noTab')));
        break;
      }
      run.failed.push(item);
      $('log').append(el('div', {}, (item.title || item.id) + ' – ' + errorText(err)));
    }
    run.done++;
    setProgress(run);
    await sleep(DELAY[state.platform.id] || 600);
  }

  /* Gemerkt wird nur, was wirklich geschrieben wurde. */
  const all = (await chrome.storage.local.get(DONE_KEY))[DONE_KEY] || {};
  all[state.platform.id] = state.done;
  await chrome.storage.local.set({ [DONE_KEY]: all });

  const written = run.done - run.failed.length;
  try {
    if (zip && zip.count) {
      const name = 'AI-Chats/' + sanitizeSegment(platformName + ' export ' + isoDate(now) + ' ' + String(now.getHours()).padStart(2, '0') + '-' + String(now.getMinutes()).padStart(2, '0')) + '.zip';
      target = 'Downloads/' + name;
      await downloadBlob(zip.blob(), name, { saveAs: state.settings.saveAs });
    }
    $('result').hidden = false;
    $('result').textContent = run.stop ? t('bulk.stopped', { n: written }) : t('bulk.done', { n: written, target: target || '—' });
    if (run.failed.length) $('result').textContent += ' · ' + t('bulk.failed', { n: run.failed.length });
  } catch (err) {
    $('result').hidden = false;
    $('result').textContent = errorText(err);
  }
  $('bar').className = 'bar done';
  $('start').disabled = false;
  state.run = null;
  renderList();
}

/* ----------------------------- Verdrahten ----------------------------- */

$('load').addEventListener('click', loadList);
$('start').addEventListener('click', start);
$('search').addEventListener('input', () => {
  state.search = $('search').value;
  renderList();
});
$('since').addEventListener('change', () => {
  state.since = $('since').value;
  renderList();
});
$('selAll').addEventListener('click', () => {
  for (const item of visibleItems()) state.selected.add(item.id);
  renderList();
});
$('selNone').addEventListener('click', () => {
  for (const item of visibleItems()) state.selected.delete(item.id);
  renderList();
});
$('pause').addEventListener('click', () => {
  const run = state.run;
  if (!run) return;
  run.paused = !run.paused;
  $('pause').textContent = run.paused ? t('bulk.resume') : t('bulk.pause');
  if (!run.paused && run.resume) {
    run.resume();
    run.resume = null;
  }
});
$('stop').addEventListener('click', () => {
  const run = state.run;
  if (!run) return;
  run.stop = true;
  if (run.resume) run.resume();
});
window.addEventListener('beforeunload', (ev) => {
  if (state.run) ev.preventDefault();
});

async function init() {
  state.settings = await send('getSettings');
  if (setLanguage(state.settings.language)) applyStatic();
  $('langSwitch').replaceChildren(
    languageSwitch(languagePreference(), async (v) => {
      state.settings = await send('setOption', { key: 'language', value: v });
      if (setLanguage(v)) applyStatic();
      init();
    })
  );
  const tab = await chrome.tabs.get(tabId).catch(() => null);
  state.platform = tab && platformFor(tab.url || '');
  if (!state.platform || !state.platform.bulk) {
    $('platformChip').textContent = state.platform ? state.platform.name : '—';
    $('sourceDesc').textContent = tab ? t('error.bulk-unsupported') : t('bulk.noTab');
    $('load').disabled = true;
    $('start').disabled = true;
    return;
  }
  $('platformChip').textContent = state.platform.name;
  $('sourceDesc').textContent = t('bulk.sourceDesc', { name: state.platform.name });
  state.done = ((await chrome.storage.local.get(DONE_KEY))[DONE_KEY] || {})[state.platform.id] || {};
  renderExportPanel();
  if (state.items.length) renderList();
}

init().catch((err) => {
  $('sourceDesc').textContent = errorText(err);
});

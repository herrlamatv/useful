/* dlplist - by herrlamatv
 * Export-Seite: Liste verwalten, Einstellungen wählen, Befehl ansehen,
 * kopieren oder als Datei herunterladen.
 */

import { t, setLanguage, applyStatic, languagePreference } from './i18n.js';
import { send, el, segmented, thumb, itemTitle, itemSubline, copyToClipboard, relTime, confirmButton } from './common.js';
import { presetsPanel, formatPanel, generalPanel, envPanel, outputPanel, extensionPanel } from './export-settings.js';
import { CATS, extractUrls } from '../src/links.js';
import { KEYS, mergeSettings, counts as countItems, addHistory } from '../src/store.js';
import { mergeExport, applyFormat, formatOf, buildExport, linksOnly } from '../src/command.js';

const $ = (id) => document.getElementById(id);
const MAX_ROWS = 400;
const MAX_HISTORY_TEXT = 400000;

let settings = mergeSettings(null);
let items = [];
let cfg = mergeExport(null);
let presets = [];
let history = [];
let tab = 'all';
let lastSaved = '';
let saveTimer = null;
const countEls = {};

/* ------------------------------------------------------------------ */
/* Einstellungen ändern und speichern                                   */
/* ------------------------------------------------------------------ */

function setPath(obj, path, value) {
  const parts = path.split('.');
  const root = JSON.parse(JSON.stringify(obj));
  let cur = root;
  for (let i = 0; i < parts.length - 1; i++) cur = cur[parts[i]];
  cur[parts[parts.length - 1]] = value;
  return root;
}

function update(path, value, rerender = true) {
  cfg = mergeExport(setPath(cfg, path, value));
  scheduleSave();
  if (rerender) renderSettings();
  renderPreview();
}

function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    lastSaved = JSON.stringify(cfg);
    chrome.storage.local.set({ [KEYS.exp]: cfg });
  }, 300);
}

const api = {
  cfg: () => cfg,
  set: update,
  counts: () => countItems(items),
  countEls,
  presets: () => presets,
  settings: () => settings,
  applyFormat(format) {
    cfg = applyFormat(cfg, format);
    scheduleSave();
    renderSettings();
    renderPreview();
  },
  savePreset(name) {
    const entry = { id: 'p' + Date.now().toString(36), name: name.slice(0, 40), format: formatOf(cfg) };
    presets = presets.filter((p) => p.name !== entry.name).concat(entry);
    chrome.storage.local.set({ [KEYS.presets]: presets });
    renderSettings();
  },
  deletePreset(id) {
    presets = presets.filter((p) => p.id !== id);
    chrome.storage.local.set({ [KEYS.presets]: presets });
    renderSettings();
  },
  async patchSettings(patch) {
    settings = await send('saveSettings', { patch });
    renderSettings();
  },
  async refetchTitles() {
    await send('fillTitles', { retry: true });
    flash('listFlash', t('list.titlesRequested'));
  },
  openShortcuts() {
    chrome.tabs.create({ url: 'chrome://extensions/shortcuts' });
  }
};

/* ------------------------------------------------------------------ */
/* Kopfzeile                                                            */
/* ------------------------------------------------------------------ */

function renderHeader() {
  $('master').checked = settings.enabled;
  $('masterLabel').textContent = settings.enabled ? t('app.on') : t('app.off');
  $('langSwitch').replaceChildren(
    segmented(
      [
        ['auto', t('lang.auto')],
        ['de', 'DE'],
        ['en', 'EN']
      ],
      languagePreference(),
      async (value) => {
        settings = await send('saveSettings', { patch: { language: value } });
        setLanguage(value);
        renderAll();
      }
    )
  );
}

$('master').addEventListener('change', async (e) => {
  settings = await send('setEnabled', { value: e.target.checked });
  renderHeader();
});

/* ------------------------------------------------------------------ */
/* Liste                                                                */
/* ------------------------------------------------------------------ */

function renderList() {
  const c = countItems(items);
  $('listSummary').textContent =
    c.total === 0 ? t('list.summaryEmpty') : t('list.summary', { total: c.total, pending: c.pending, exported: c.exported });

  const tabs = [['all', t('list.all') + ' ' + c.total]].concat(CATS.map((cat) => [cat, t('cat.' + cat) + ' ' + (c[cat] || 0)]));
  $('catTabs').replaceChildren(
    segmented(tabs, tab, (value) => {
      tab = value;
      renderList();
    })
  );

  const shown = items.filter((it) => tab === 'all' || it.cat === tab).slice().reverse();
  if (!shown.length) {
    $('listBox').replaceChildren(
      el('div', { class: 'empty' }, [el('div', {}, t(c.total ? 'list.emptyCat' : 'list.empty')), el('div', { class: 'hint', style: 'margin-top: 6px' }, t('list.emptyHint'))])
    );
  } else {
    const catEntries = CATS.map((cat) => [cat, t('cat.' + cat)]);
    const rows = shown.slice(0, MAX_ROWS).map((it) => {
      const move = el('select', { title: t('list.move') });
      for (const [value, label] of catEntries) {
        const opt = el('option', { value }, label);
        if (value === it.cat) opt.selected = true;
        move.append(opt);
      }
      move.addEventListener('change', () => send('moveItems', { keys: [it.key], cat: move.value }));
      return el('div', { class: 'entry' + (it.exported ? ' exported' : '') }, [
        thumb(it),
        el('div', { class: 'txt' }, [
          el('div', { class: 't' }, el('a', { href: it.url, target: '_blank', rel: 'noopener noreferrer', title: it.url, style: 'color: inherit' }, itemTitle(it))),
          el('div', { class: 'd' }, itemSubline(it))
        ]),
        it.status === 'unavailable' ? el('span', { class: 'chip warn', title: t('list.unavailableDesc') }, t('list.unavailable')) : null,
        it.exported ? el('span', { class: 'chip done', title: new Date(it.exported).toLocaleString() }, t('list.exported')) : null,
        move,
        el('button', { class: 'icon', type: 'button', title: t('common.remove'), onclick: () => send('removeItems', { keys: [it.key] }) }, '✕')
      ]);
    });
    if (shown.length > MAX_ROWS) rows.push(el('div', { class: 'entry hint' }, t('list.more', { n: shown.length - MAX_ROWS })));
    $('listBox').replaceChildren(el('div', { class: 'list list-scroll' }, rows));
  }

  const tools = [];
  if (c.exported) tools.push(el('button', { class: 'btn small', type: 'button', onclick: () => send('clearItems', { filter: 'exported' }) }, t('list.clearExported')));
  if (c.unavailable) tools.push(el('button', { class: 'btn small', type: 'button', onclick: () => send('clearItems', { filter: 'unavailable' }) }, t('list.clearUnavailable')));
  if (c.total) tools.push(confirmButton(t('list.clearAll'), t('common.confirm'), () => send('clearItems', { filter: 'all' })));
  $('listTools').replaceChildren(...tools);

  for (const cat of CATS) if (countEls[cat]) countEls[cat].textContent = String(c[cat] || 0);
}

async function addUrls(urls, source) {
  if (!urls.length) return flash('listFlash', t('list.noLinks'), true);
  const res = await send('addUrls', { urls, source });
  const parts = [t('list.added', { n: res.added })];
  if (res.moved) parts.push(t('toast.movedMany', { n: res.moved }));
  if (res.dupes) parts.push(t('toast.dupesMany', { n: res.dupes }));
  if (res.invalid) parts.push(t('list.invalid', { n: res.invalid }));
  flash('listFlash', parts.join(' · '), !res.added && !res.moved);
  return res;
}

$('pasteAdd').addEventListener('click', async () => {
  const res = await addUrls(extractUrls($('paste').value), 'paste');
  if (res && (res.added || res.moved)) $('paste').value = '';
});

/* Drag & Drop auf die ganze Seite */
let dragDepth = 0;
const hasText = (e) => [...(e.dataTransfer?.types || [])].some((x) => x === 'text/uri-list' || x === 'text/plain');
const inField = (e) => e.target && (e.target.tagName === 'TEXTAREA' || e.target.tagName === 'INPUT');
document.addEventListener('dragenter', (e) => {
  if (!hasText(e) || inField(e)) return;
  dragDepth++;
  $('drop').hidden = false;
});
document.addEventListener('dragleave', () => {
  if (--dragDepth <= 0) {
    dragDepth = 0;
    $('drop').hidden = true;
  }
});
document.addEventListener('dragover', (e) => {
  if (hasText(e) && !inField(e)) e.preventDefault();
});
document.addEventListener('drop', (e) => {
  dragDepth = 0;
  $('drop').hidden = true;
  if (inField(e)) return;
  e.preventDefault();
  const text = e.dataTransfer.getData('text/uri-list') + '\n' + e.dataTransfer.getData('text/plain');
  addUrls(extractUrls(text), 'drop');
});

/* ------------------------------------------------------------------ */
/* Einstellungen                                                        */
/* ------------------------------------------------------------------ */

function renderSettings() {
  $('slotPresets').replaceChildren(presetsPanel(api));
  $('slotFormat').replaceChildren(formatPanel(api));
  $('slotGeneral').replaceChildren(generalPanel(api));
  $('slotEnv').replaceChildren(envPanel(api));
  $('slotOutput').replaceChildren(outputPanel(api));
  $('slotExt').replaceChildren(extensionPanel(api));
}

/* ------------------------------------------------------------------ */
/* Vorschau                                                             */
/* ------------------------------------------------------------------ */

const KEYWORDS = /^(\s*)(@echo|echo|set|if|goto|call|pushd|popd|title|exit|pause|where|start|shutdown|chcp|setlocal|cd|rem|foreach|for|do|done|then|else|fi|Write-Host|Set-Location|New-Item|Read-Host|Invoke-Item|mkdir|command|read)\b/i;
const TOKENS = /("[^"]*"|'(?:[^']|'')*')|((?:^|\s)--?[A-Za-z][\w-]*)|(%[A-Za-z_~][\w]*%?|\$\{?[A-Za-z_][\w:]*\}?)/g;

function isComment(line, target) {
  if (/^\s*#/.test(line)) return target !== 'bat' && target !== 'cmd';
  return /^\s*(::|rem\b)/i.test(line) && (target === 'bat' || target === 'cmd');
}

/** Einfache Syntax-Färbung - nur Knoten, kein innerHTML. */
function highlight(text, target) {
  const frag = document.createDocumentFragment();
  const lines = text.replace(/\r\n/g, '\n').replace(/\n$/, '').split('\n');
  lines.forEach((line, i) => {
    if (i) frag.append('\n');
    if (!target || isComment(line, target)) {
      frag.append(target ? el('span', { class: 'tok-cmt' }, line) : line);
      return;
    }
    let rest = line;
    const kw = KEYWORDS.exec(rest);
    if (kw) {
      frag.append(kw[1], el('span', { class: 'tok-kw' }, kw[2]));
      rest = rest.slice(kw[0].length);
    }
    let last = 0;
    let m;
    TOKENS.lastIndex = 0;
    while ((m = TOKENS.exec(rest)) !== null) {
      if (m.index > last) frag.append(rest.slice(last, m.index));
      const cls = m[1] ? 'tok-str' : m[2] ? 'tok-flag' : 'tok-var';
      frag.append(el('span', { class: cls }, m[0]));
      last = m.index + m[0].length;
    }
    if (last < rest.length) frag.append(rest.slice(last));
  });
  return frag;
}

function warningText(w) {
  const vars = Object.assign({}, w.vars);
  if (vars.cat) vars.cat = t('cat.' + vars.cat);
  if (vars.field) vars.field = t('field.' + vars.field);
  return t('warn.' + w.code, vars);
}

let current = null;

function renderPreview() {
  current = buildExport(items, cfg, { t, now: new Date() });
  $('fileChip').textContent = current.filename;
  $('fileChip').title = current.filename;
  const parts = [t('preview.summary', { n: current.count })];
  if (current.skipped) parts.push(t('preview.skipped', { n: current.skipped }));
  $('previewSummary').textContent = parts.join(' · ');
  $('code').replaceChildren(highlight(current.text, cfg.output.type === 'links' ? null : current.target));
  $('warns').replaceChildren(
    ...current.warnings.map((w) => el('div', { class: 'note' + (w.level === 'info' ? ' info' : '') }, warningText(w)))
  );
  $('download').disabled = !current.count;
  $('copy').disabled = !current.count;
  $('copyLinks').disabled = !current.count;
}

/* ------------------------------------------------------------------ */
/* Kopieren, Herunterladen, Verlauf                                     */
/* ------------------------------------------------------------------ */

const flashTimers = {};
function flash(id, text, bad = false) {
  const node = $(id);
  node.textContent = text;
  node.className = 'flash' + (bad ? ' bad' : '');
  clearTimeout(flashTimers[id]);
  flashTimers[id] = setTimeout(() => (node.textContent = ''), 4000);
}

$('copy').addEventListener('click', () => {
  if (current && current.count) copyToClipboard(current.text, $('copy'));
});

$('copyLinks').addEventListener('click', () => {
  copyToClipboard(linksOnly(items, cfg), $('copyLinks'));
});

/* Laufende Downloads: id → { url, res } - fertig erst bei state=complete. */
const pending = new Map();

async function startDownload(res, { remember = true } = {}) {
  const blob = new Blob([(res.bom ? String.fromCharCode(0xfeff) : '') + res.text], { type: res.mime });
  const url = URL.createObjectURL(blob);
  let id;
  try {
    id = await chrome.downloads.download({ url, filename: res.filename, saveAs: cfg.output.saveAs, conflictAction: 'uniquify' });
  } catch (e) {
    id = undefined;
  }
  if (id === undefined) {
    URL.revokeObjectURL(url);
    flash('actionFlash', t('preview.cancelled'), true);
    return;
  }
  pending.set(id, { url, res, remember });
  const [item] = await chrome.downloads.search({ id });
  if (item) onDownloadChange({ id, state: { current: item.state }, danger: { current: item.danger } });
}

async function finished(entry) {
  const { res, remember } = entry;
  flash('actionFlash', t('preview.saved', { file: res.filename }));
  $('danger').replaceChildren();
  if (!remember) return;
  if (cfg.output.after === 'mark') await send('markExported', { keys: res.keys, at: Date.now() });
  else if (cfg.output.after === 'clear') await send('removeItems', { keys: res.keys });
  history = addHistory(history, {
    at: Date.now(),
    name: res.filename,
    ext: res.ext,
    count: res.count,
    mime: res.mime,
    bom: res.bom,
    text: res.text.length > MAX_HISTORY_TEXT ? '' : res.text
  });
  chrome.storage.local.set({ [KEYS.history]: history });
  renderHistory();
}

function onDownloadChange(delta) {
  const entry = pending.get(delta.id);
  if (!entry) return;
  const danger = delta.danger && delta.danger.current;
  if (danger && danger !== 'safe' && danger !== 'accepted') {
    $('danger').replaceChildren(
      el('div', { class: 'note', style: 'margin-top: 8px' }, [
        el('div', {}, t('preview.danger', { ext: entry.res.ext })),
        el('div', { style: 'margin-top: 8px' }, [
          el('button', { class: 'btn small primary', type: 'button', onclick: () => chrome.downloads.acceptDanger(delta.id) }, t('preview.keep'))
        ])
      ])
    );
  }
  const state = delta.state && delta.state.current;
  if (state === 'complete') {
    pending.delete(delta.id);
    URL.revokeObjectURL(entry.url);
    finished(entry);
  } else if (state === 'interrupted') {
    pending.delete(delta.id);
    URL.revokeObjectURL(entry.url);
    $('danger').replaceChildren();
    flash('actionFlash', t('preview.cancelled'), true);
  }
}

chrome.downloads.onChanged.addListener(onDownloadChange);

$('download').addEventListener('click', () => {
  if (current && current.count) startDownload(current);
});

function renderHistory() {
  if (!history.length) {
    $('history').replaceChildren(el('div', { class: 'empty' }, t('history.empty')));
    return;
  }
  $('history').replaceChildren(
    el(
      'div',
      { class: 'list' },
      history.map((h) =>
        el('div', { class: 'entry' }, [
          el('div', { class: 'thumb glyph', style: 'font-size: 11px' }, '.' + h.ext),
          el('div', { class: 'txt' }, [
            el('div', { class: 't' }, h.name),
            el('div', { class: 'd' }, t('history.meta', { n: h.count, when: relTime(h.at) }))
          ]),
          h.text
            ? el('button', { class: 'btn small', type: 'button', onclick: (e) => copyToClipboard(h.text, e.currentTarget) }, t('preview.copy'))
            : null,
          h.text
            ? el(
                'button',
                {
                  class: 'btn small',
                  type: 'button',
                  onclick: () => startDownload({ text: h.text, filename: h.name, mime: h.mime, bom: h.bom, ext: h.ext, keys: [], count: h.count }, { remember: false })
                },
                t('history.again')
              )
            : null
        ])
      )
    ),
    el('div', { class: 'row', style: 'margin-top: 10px' }, [
      el('span', { class: 'grow' }),
      confirmButton(t('history.clear'), t('common.confirm'), () => {
        history = [];
        chrome.storage.local.set({ [KEYS.history]: history });
        renderHistory();
      }, 'btn ghost small')
    ])
  );
}

/* ------------------------------------------------------------------ */
/* Laden und live halten                                                */
/* ------------------------------------------------------------------ */

function renderAll() {
  applyStatic();
  renderHeader();
  renderList();
  renderSettings();
  renderPreview();
  renderHistory();
}

async function load() {
  const state = await send('getState');
  settings = state.settings;
  items = state.items;
  cfg = state.export;
  presets = state.presets;
  history = state.history;
  lastSaved = JSON.stringify(cfg);
  setLanguage(settings.language);
  renderAll();
}

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== 'local') return;
  if (changes[KEYS.items]) {
    items = Array.isArray(changes[KEYS.items].newValue) ? changes[KEYS.items].newValue : [];
    renderList();
    renderPreview();
  }
  if (changes[KEYS.settings]) {
    const next = mergeSettings(changes[KEYS.settings].newValue);
    const langChanged = next.language !== settings.language;
    settings = next;
    if (langChanged) {
      setLanguage(settings.language);
      renderAll();
    } else {
      renderHeader();
    }
  }
  if (changes[KEYS.exp]) {
    // eigene Schreibvorgänge ignorieren, nur fremde Änderungen (anderer Tab) übernehmen
    const next = mergeExport(changes[KEYS.exp].newValue);
    const json = JSON.stringify(next);
    if (json !== lastSaved && json !== JSON.stringify(cfg)) {
      cfg = next;
      lastSaved = json;
      renderSettings();
      renderPreview();
    }
  }
  if (changes[KEYS.presets]) {
    const next = Array.isArray(changes[KEYS.presets].newValue) ? changes[KEYS.presets].newValue : [];
    if (JSON.stringify(next) !== JSON.stringify(presets)) {
      presets = next;
      renderSettings();
    }
  }
  if (changes[KEYS.history]) {
    history = Array.isArray(changes[KEYS.history].newValue) ? changes[KEYS.history].newValue : [];
    renderHistory();
  }
});

load().catch((e) => flash('listFlash', e.message, true));

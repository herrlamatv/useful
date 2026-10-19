/* ai chat exporter - by herrlamatv : Optionen */

import { send, el, errorText, segmented, toggleRow, row, languageSwitch } from './common.js';
import { t, setLanguage, applyStatic, languagePreference } from './i18n.js';
import { PLATFORMS } from '../src/platforms.js';
import { FORMATS } from '../src/settings.js';
import { buildPath } from '../src/export/filename.js';

const $ = (id) => document.getElementById(id);
const INCLUDE = ['thinking', 'sources', 'images', 'attachments', 'tools', 'times'];

let settings = null;
let savedTimer = 0;

applyStatic();

function saved() {
  $('savedHint').textContent = t('opt.saved') + ' ✓';
  clearTimeout(savedTimer);
  savedTimer = setTimeout(() => ($('savedHint').textContent = ''), 1600);
}

async function set(key, value, { quiet = false } = {}) {
  try {
    settings = await send('setOption', { key, value });
    if (setLanguage(settings.language)) applyStatic();
    if (!quiet) render();
    else renderPreview();
    saved();
  } catch (err) {
    $('savedHint').textContent = errorText(err);
  }
}

/* ------------------------------- Teile -------------------------------- */

function formatSelect() {
  const select = el('select', { class: 'select', onchange: (ev) => set('format', ev.target.value) });
  for (const f of FORMATS) select.append(el('option', { value: f, selected: f === settings.format }, t('format.' + f)));
  return select;
}

function renderDefaults() {
  $('defaults').replaceChildren(
    row(t('opt.format'), null, formatSelect()),
    row(
      t('opt.paper'),
      null,
      segmented(
        [
          ['a4', 'A4'],
          ['letter', 'Letter']
        ],
        settings.paper,
        (v) => set('paper', v)
      )
    ),
    row(
      t('opt.fontSize'),
      null,
      segmented(
        [
          ['s', 'S'],
          ['m', 'M'],
          ['l', 'L']
        ],
        settings.fontSize,
        (v) => set('fontSize', v)
      )
    )
  );
  $('methodSeg').replaceChildren(
    segmented(
      [
        ['direct', t('method.direct')],
        ['print', t('method.print')]
      ],
      settings.pdfMethod,
      (v) => set('pdfMethod', v)
    )
  );
  $('methodNote').textContent = t(settings.pdfMethod === 'print' ? 'method.printDesc' : 'method.directDesc');
}

function renderInclude() {
  $('include').replaceChildren(
    ...INCLUDE.map((key) => toggleRow(t('inc.' + key), t('inc.' + key + 'Desc'), settings.include[key], (v) => set('include.' + key, v)))
  );
}

function renderPreview() {
  const sample = { platform: 'claude', platformName: 'Claude', title: 'Python – Listen sortieren', model: 'claude-opus-5-5', id: 'a1b2c3', createdAt: new Date().toISOString() };
  $('pathPreview').textContent = t('opt.preview', { path: 'Downloads/' + buildPath(settings, sample, 'md') });
}

function renderFiles() {
  if (document.activeElement !== $('folder')) $('folder').value = settings.folder;
  if (document.activeElement !== $('file')) $('file').value = settings.file;
  renderPreview();
  $('files').replaceChildren(toggleRow(t('opt.saveAs'), t('opt.saveAsDesc'), settings.saveAs, (v) => set('saveAs', v)));
}

function renderObsidian() {
  $('obsidian').replaceChildren(toggleRow(t('opt.frontmatter'), t('opt.frontmatterDesc'), settings.frontmatter, (v) => set('frontmatter', v)));
  if (document.activeElement !== $('tags')) $('tags').value = settings.tags;
}

function renderInChat() {
  $('inChat').replaceChildren(
    toggleRow(t('opt.buttons'), t('opt.buttonsDesc'), settings.buttons, (v) => set('buttons', v)),
    row(
      t('opt.shortcuts'),
      t('opt.shortcutsDesc'),
      el('button', { class: 'btn small', type: 'button', onclick: () => chrome.tabs.create({ url: 'chrome://extensions/shortcuts' }) }, t('opt.shortcutsBtn'))
    )
  );
}

function renderPlatforms() {
  $('platforms').replaceChildren(
    ...PLATFORMS.map((p) =>
      el('div', { class: 'p' }, [
        el('b', {}, p.name),
        el('span', {}, (p.bulk ? t('opt.api') : t('opt.dom')) + ' · ' + p.matches.map((m) => m.replace(/^https:\/\//, '').replace(/\/\*$/, '')).join(', '))
      ])
    )
  );
}

function render() {
  $('langSwitch').replaceChildren(languageSwitch(languagePreference(), (v) => set('language', v)));
  renderDefaults();
  renderInclude();
  renderFiles();
  renderObsidian();
  renderInChat();
  renderPlatforms();
}

/* ----------------------------- Verdrahten ----------------------------- */

/* Textfelder: beim Tippen kurz warten, dann speichern. */
for (const [id, key] of [
  ['folder', 'folder'],
  ['file', 'file'],
  ['tags', 'tags']
]) {
  let timer = 0;
  $(id).addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(() => set(key, $(id).value, { quiet: true }), 400);
  });
}

$('reset').addEventListener('click', async () => {
  settings = await send('resetSettings');
  if (setLanguage(settings.language)) applyStatic();
  render();
  $('resetStatus').textContent = t('reset.done');
});

async function init() {
  settings = await send('getSettings');
  if (setLanguage(settings.language)) applyStatic();
  render();
}

init().catch((err) => {
  $('savedHint').textContent = errorText(err);
});

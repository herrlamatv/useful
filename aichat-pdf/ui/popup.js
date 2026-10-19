/* ai chat exporter - by herrlamatv : Popup
 *
 * Zum Testen in einem normalen Tab: ui/popup.html?tab=<tabId>
 */

import { send, el, errorText, check, segmented } from './common.js';
import { t, setLanguage, applyStatic } from './i18n.js';

const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);
const MODE_KEY = 'aichat.pdfMode';

const FORMATS = [
  ['md', 'btn.md', '.md'],
  ['pdf', 'btn.pdf', '.pdf'],
  ['html', 'btn.html', '.html'],
  ['txt', 'btn.txt', '.txt'],
  ['json', 'btn.json', '.json'],
  ['copy', 'btn.copy', 'MD']
];
const INCLUDE = ['thinking', 'sources', 'images', 'attachments', 'tools', 'times'];

const state = {
  tab: null,
  platform: null,
  settings: null,
  scan: null,
  selected: null, // Set der Schlüssel - null = alle
  roles: 'all',
  listOpen: false,
  pdfMode: 'styled',
  busy: false
};

applyStatic();

/* ------------------------------ Anzeige ------------------------------- */

function status(text, kind = '') {
  const box = $('status');
  box.hidden = !text;
  box.className = 'panel-body status' + (kind ? ' ' + kind : '');
  box.textContent = text || '';
}

function setBusy(on) {
  state.busy = on;
  for (const b of $('formats').querySelectorAll('button')) b.disabled = on || !hasMessages();
}

const hasMessages = () => !!(state.scan && state.scan.messages.length);

function chosenKeys() {
  if (!state.scan) return [];
  return state.scan.messages
    .filter((m) => state.roles === 'all' || m.role === state.roles)
    .filter((m) => !state.selected || state.selected.has(m.key))
    .map((m) => m.key);
}

function renderUnsupported() {
  $('chatTitle').textContent = t('popup.title');
  $('chatMeta').textContent = t('popup.unsupported');
  $('formatPanel').hidden = true;
}

function renderHeader() {
  const chip = $('platformChip');
  chip.hidden = false;
  chip.className = 'chip site';
  chip.textContent = state.platform.id === 'universal' ? t('popup.universal') : state.platform.name;
  $('openBulk').hidden = !state.platform.bulk;
  $('pickBody').hidden = state.platform.id !== 'universal';
}

function renderFormats() {
  const primary = String(state.settings.format).startsWith('pdf') ? 'pdf' : state.settings.format;
  $('formats').replaceChildren(
    ...FORMATS.map(([id, label, ext]) =>
      el('button', { class: 'fmt' + (id === primary ? ' primary' : ''), type: 'button', 'data-format': id, onclick: () => run(id) }, [
        el('span', { class: 'ext' }, ext),
        t(label)
      ])
    )
  );
  $('pdfSeg').replaceChildren(
    segmented(
      [
        ['styled', t('pdf.styled')],
        ['raw', t('pdf.raw')],
        ['shot', t('pdf.shot')]
      ],
      state.pdfMode,
      (value) => {
        state.pdfMode = value;
        try {
          localStorage.setItem(MODE_KEY, value);
        } catch (e) {}
        renderFormats();
      }
    )
  );
  setBusy(state.busy);
}

function renderInclude() {
  const inc = state.settings.include;
  $('includeChips').replaceChildren(
    ...INCLUDE.map((key) =>
      el(
        'button',
        {
          class: 'tchip',
          type: 'button',
          'aria-pressed': String(!!inc[key]),
          title: t('inc.' + key + 'Desc'),
          onclick: async () => {
            state.settings = await send('setOption', { key: 'include.' + key, value: !inc[key] });
            renderInclude();
          }
        },
        t('inc.' + key)
      )
    )
  );
}

function renderChat() {
  const scan = state.scan;
  $('chatTitle').textContent = scan.title || state.tab.title || '';
  const n = scan.messages.length;
  if (!n) {
    $('chatMeta').textContent = t('popup.empty');
    $('selectBody').hidden = true;
    setBusy(false);
    return;
  }
  const how = scan.source === 'api' ? t('popup.viaApi', { name: scan.platformName }) : t('popup.viaDom');
  $('chatMeta').textContent = t('popup.messages', { n }) + ' · ' + how + (scan.source === 'api' ? '' : ' · ' + t('popup.olderHint'));
  $('selectBody').hidden = false;
  renderList();
  setBusy(false);
}

function renderList() {
  const scan = state.scan;
  const total = scan.messages.length;
  $('toggleList').textContent = (state.listOpen ? '▾ ' : '▸ ') + t('popup.select');
  $('selCount').textContent = t('popup.selected', { n: chosenKeys().length, total });
  $('listWrap').hidden = !state.listOpen;
  if (!state.listOpen) return;

  $('roleSeg').replaceChildren(
    segmented(
      [
        ['all', t('popup.all')],
        ['user', t('popup.prompts')],
        ['assistant', t('popup.answers')]
      ],
      state.roles,
      (value) => {
        state.roles = value;
        renderList();
      }
    )
  );

  $('msgList').replaceChildren(
    ...scan.messages.map((m) => {
      const on = !state.selected || state.selected.has(m.key);
      const roleOff = state.roles !== 'all' && m.role !== state.roles;
      const toggleOne = (value) => {
        if (!state.selected) state.selected = new Set(scan.messages.map((x) => x.key));
        if (value) state.selected.add(m.key);
        else state.selected.delete(m.key);
        if (state.selected.size === scan.messages.length) state.selected = null;
        renderList();
      };
      return el('div', { class: 'msg-row ' + m.role + (roleOff ? ' off' : ''), onclick: () => toggleOne(!on) }, [
        check(on, toggleOne),
        el('div', { class: 'grow' }, [
          el('div', { class: 'who' }, m.role === 'user' ? t('popup.you') : scan.platformName),
          el('div', { class: 'pv' }, m.preview || '…')
        ])
      ]);
    })
  );
}

/* ------------------------------ Aktionen ------------------------------ */

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (e) {
    const ta = el('textarea', { style: 'position:fixed;top:-1000px' });
    ta.value = text;
    document.body.append(ta);
    ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    if (!ok) throw new Error('copy-failed');
  }
}

async function run(id) {
  if (state.busy || !hasMessages()) return;
  const keys = chosenKeys();
  if (!keys.length) {
    status(t('error.empty'), 'error');
    return;
  }
  const format = id === 'pdf' ? 'pdf-' + state.pdfMode : id;
  const payload = {
    tabId: state.tab.id,
    keys: state.selected || state.roles !== 'all' ? keys : null,
    roles: 'all',
    include: state.settings.include
  };
  setBusy(true);
  status(format === 'pdf-shot' ? t('status.shot') : t('status.working'));
  try {
    if (id === 'copy') {
      await copyText(await send('markdown', payload));
      status(t('status.copied'), 'ok');
    } else {
      const res = await send('export', Object.assign({ format }, payload));
      if (res.pdf === 'print') status(t('status.print'), 'ok');
      else if (res.pdf) status(t('status.pdf'), 'ok');
      else status(t('status.saved', { file: res.filename.split('/').pop() }), 'ok');
    }
  } catch (err) {
    status(errorText(err), 'error');
  } finally {
    setBusy(false);
  }
}

/* ----------------------------- Verdrahten ----------------------------- */

$('toggleList').addEventListener('click', () => {
  state.listOpen = !state.listOpen;
  renderList();
});

$('selAll').addEventListener('click', () => {
  state.selected = null;
  state.roles = 'all';
  renderList();
});

$('selNone').addEventListener('click', () => {
  state.selected = new Set();
  renderList();
});

$('pick').addEventListener('click', async () => {
  try {
    await send('pick', { tabId: state.tab.id });
    window.close();
  } catch (err) {
    status(errorText(err), 'error');
  }
});

$('openBulk').addEventListener('click', async (ev) => {
  ev.preventDefault();
  await send('openBulk', { tabId: state.tab.id });
  window.close();
});

$('openOptions').addEventListener('click', (ev) => {
  ev.preventDefault();
  chrome.runtime.openOptionsPage();
});

async function init() {
  state.settings = await send('getSettings');
  if (setLanguage(state.settings.language)) applyStatic();
  try {
    state.pdfMode = localStorage.getItem(MODE_KEY) || '';
  } catch (e) {}
  if (!['styled', 'raw', 'shot'].includes(state.pdfMode)) {
    state.pdfMode = String(state.settings.format).startsWith('pdf-') ? state.settings.format.slice(4) : 'styled';
  }

  const tabId = Number(params.get('tab')) || ((await chrome.tabs.query({ active: true, currentWindow: true }))[0] || {}).id;
  const info = await send('describeTab', { tabId });
  state.tab = info.tab;
  state.platform = info.platform;
  if (!state.platform) return renderUnsupported();

  renderHeader();
  renderFormats();
  renderInclude();
  $('formatPanel').hidden = false;
  setBusy(true);
  try {
    state.scan = await send('scan', { tabId: state.tab.id });
    renderChat();
  } catch (err) {
    $('chatTitle').textContent = state.tab.title || '';
    $('chatMeta').textContent = errorText(err);
    setBusy(false);
  }
}

init().catch((err) => {
  $('chatMeta').textContent = errorText(err);
});

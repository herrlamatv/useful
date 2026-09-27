/* dlplist - by herrlamatv
 * Popup: Hauptschalter, Zähler, die letzten Links, Knöpfe.
 */

import { t, setLanguage, applyStatic } from './i18n.js';
import { send, el, thumb, itemTitle, itemSubline, confirmButton } from './common.js';
import { CATS, extractUrls } from '../src/links.js';
import { KEYS } from '../src/store.js';

const $ = (id) => document.getElementById(id);
let state = null;
let flashTimer = null;

function flash(text, bad = false) {
  const node = $('flash');
  node.textContent = text;
  node.className = 'flash' + (bad ? ' bad' : '');
  clearTimeout(flashTimer);
  flashTimer = setTimeout(() => (node.textContent = ''), 2600);
}

async function load() {
  state = await send('getState');
  setLanguage(state.settings.language);
  applyStatic();
  render();
}

function render() {
  const { settings, items, counts } = state;
  document.body.classList.toggle('dimmed', !settings.enabled);
  $('master').checked = settings.enabled;

  $('pending').textContent = String(counts.pending);
  $('status').textContent =
    counts.total === 0
      ? t('popup.statusEmpty')
      : t('popup.status', { pending: counts.pending, total: counts.total });

  const chip = $('stateChip');
  chip.textContent = settings.enabled ? t('app.on') : t('app.off');
  chip.className = 'chip ' + (settings.enabled ? 'on' : 'off');

  $('chips').replaceChildren(
    ...CATS.map((cat) => el('span', { class: 'chip' }, [t('cat.' + cat) + ' ', el('span', { class: 'n' }, String(counts[cat] || 0))]))
  );

  const recent = items.slice(-5).reverse();
  $('recent').replaceChildren(
    recent.length
      ? el(
          'div',
          { class: 'list' },
          recent.map((it) =>
            el('div', { class: 'entry' + (it.exported ? ' exported' : '') }, [
              thumb(it),
              el('div', { class: 'txt' }, [
                el('div', { class: 't', title: it.url }, itemTitle(it)),
                el('div', { class: 'd' }, t('cat.' + it.cat) + ' · ' + itemSubline(it))
              ]),
              el(
                'button',
                {
                  class: 'icon',
                  type: 'button',
                  title: t('common.remove'),
                  onclick: () => send('removeItems', { keys: [it.key] }).catch((e) => flash(e.message, true))
                },
                '✕'
              )
            ])
          )
        )
      : el('div', { class: 'empty' }, t('popup.empty'))
  );

  $('addTab').disabled = !settings.enabled;
  $('clearSlot').replaceChildren(
    items.length ? confirmButton(t('popup.clear'), t('common.confirm'), () => send('clearItems', { filter: 'all' })) : ''
  );
}

$('master').addEventListener('change', async (e) => {
  await send('setEnabled', { value: e.target.checked });
});

$('openExport').addEventListener('click', () => {
  chrome.runtime.openOptionsPage();
  window.close();
});

$('openOptions').addEventListener('click', (e) => {
  e.preventDefault();
  chrome.runtime.openOptionsPage();
  window.close();
});

$('addTab').addEventListener('click', async () => {
  try {
    const res = await send('addTab');
    if (res.off) flash(t('toast.off'), true);
    else if (res.added) flash(t('toast.added') + ' · ' + t('cat.' + res.cat));
    else if (res.moved) flash(t('toast.moved'));
    else if (res.dupes) flash(t('toast.dupe'));
    else flash(t('toast.nothing'), true);
  } catch (e) {
    flash(e.message, true);
  }
});

$('undo').addEventListener('click', async () => {
  const res = await send('undoLast');
  flash(res.removed ? t('popup.undone', { n: res.removed }) : t('popup.nothingToUndo'), !res.removed);
});

/* Links ins Popup ziehen */
document.addEventListener('dragover', (e) => e.preventDefault());
document.addEventListener('drop', async (e) => {
  e.preventDefault();
  const text = e.dataTransfer.getData('text/uri-list') + '\n' + e.dataTransfer.getData('text/plain');
  const urls = extractUrls(text);
  if (!urls.length) return;
  const res = await send('addUrls', { urls, source: 'drop' });
  flash(t('toast.addedMany', { n: res.added }));
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area === 'local' && (changes[KEYS.items] || changes[KEYS.settings])) load();
});

load().catch((e) => flash(e.message, true));

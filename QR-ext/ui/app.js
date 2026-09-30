/* QR-ext - by herrlamatv
 * Die eigentliche Oberfläche - Popup und große Ansicht teilen sich alles,
 * nur das Layout (HTML/CSS) und ein paar Kleinigkeiten unterscheiden sich.
 */

import { el, segmented, toggle, row, flash, download, debounce } from './common.js';
import { t, applyStatic, setLanguage, language } from './i18n.js';
import { encode, ECC_LEVELS } from '../src/qr.js';
import { TYPES, emptyFields, buildPayload, summary, fileSlug } from '../src/payload.js';
import { drawToContext, toSvg, colorWarning } from '../src/render.js';
import * as store from '../src/store.js';

const POPUP_HISTORY = 4;
const ECC_PERCENT = { L: '7 %', M: '15 %', Q: '25 %', H: '30 %' };

const $ = (id) => document.getElementById(id);

const state = {
  wide: false,
  type: 'text',
  fields: Object.fromEntries(TYPES.map((type) => [type, emptyFields(type)])),
  settings: store.mergeSettings(null),
  history: [],
  tabUrl: '',
  payload: '',
  qr: null,
  error: null
};

/* ------------------------------ Start -------------------------------- */

export async function mount({ wide }) {
  state.wide = wide;
  const [settings, history] = await Promise.all([store.loadSettings(), store.loadHistory()]);
  state.settings = settings;
  state.history = history;
  setLanguage(settings.lang);

  const params = new URLSearchParams(location.search);
  let initial = params.get('text');
  if (initial === null) initial = await store.popPending().catch(() => null);
  if (!wide) state.tabUrl = await currentTabUrl();
  if (initial === null && !wide) initial = state.tabUrl;
  if (initial) state.fields.text.text = initial;

  renderAll();

  const openWide = $('openWide');
  if (openWide) {
    openWide.addEventListener('click', (e) => {
      e.preventDefault();
      openWideView();
    });
  }

  chrome.storage.onChanged.addListener(onStorageChanged);

  if (wide && location.hash === '#history') {
    requestAnimationFrame(() => $('historyPanel').scrollIntoView({ block: 'start' }));
  }
  if (wide) focusFirstField();
}

async function currentTabUrl() {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    return (tab && (tab.url || tab.pendingUrl)) || '';
  } catch (e) {
    return '';
  }
}

function openWideView(hash = '') {
  const url = chrome.runtime.getURL('ui/qr.html') + hash;
  chrome.tabs.create({ url });
  window.close();
}

function onStorageChanged(changes, area) {
  if (area === 'local' && changes.history) {
    const next = store.cleanHistory(changes.history.newValue);
    if (JSON.stringify(next) !== JSON.stringify(state.history)) {
      state.history = next;
      renderHistory();
    }
  }
  if (area === 'sync' && changes.settings) {
    const next = store.mergeSettings(changes.settings.newValue);
    if (JSON.stringify(next) !== JSON.stringify(state.settings)) {
      const langChanged = next.lang !== state.settings.lang;
      state.settings = next;
      if (langChanged) setLanguage(next.lang);
      langChanged ? renderAll() : (renderStyle(), renderHistory(), renderSettings(), update());
    }
  }
}

function renderAll() {
  applyStatic();
  document.title = t('app.title');
  renderLang();
  renderEditor();
  renderPreview();
  renderStyle();
  renderHistory();
  renderSettings();
  update();
}

/* --------------------------- Einstellungen --------------------------- */

const persistSettings = debounce(() => store.saveSettings(state.settings), 250);

function setSetting(key, value, { rerender = true } = {}) {
  state.settings = Object.assign({}, state.settings, { [key]: value });
  persistSettings();
  if (rerender) renderStyle();
  update();
}

function renderLang() {
  const host = $('lang');
  host.replaceChildren(
    segmented(
      [
        ['auto', t('lang.auto'), t('lang.autoTitle')],
        ['de', 'DE'],
        ['en', 'EN']
      ],
      state.settings.lang,
      (lang) => {
        state.settings = Object.assign({}, state.settings, { lang });
        persistSettings();
        setLanguage(lang);
        renderAll();
      }
    )
  );
}

/* ------------------------------ Editor ------------------------------- */

function renderEditor() {
  const panel = $('editorPanel');
  const types = segmented(
    TYPES.map((type) => [type, t('type.' + type), t('type.' + type + 'Title')]),
    state.type,
    (type) => {
      state.type = type;
      renderEditor();
      update();
      focusFirstField();
    }
  );
  types.classList.add('types');

  panel.replaceChildren(
    el('div', { class: 'panel-body' }, [
      el('div', { class: 'label', style: 'margin-bottom: 8px' }, t('editor.what')),
      types,
      el('div', { class: 'fields', id: 'fields' }, fieldsFor(state.type))
    ])
  );
}

function focusFirstField() {
  const first = document.querySelector('#fields input, #fields textarea');
  if (first) first.focus();
}

/** Eingabefeld, das seinen Wert direkt in state.fields schreibt. */
function input(key, { type = 'text', label, placeholder = '', multiline = false, rows = 3, autocomplete = 'off' } = {}) {
  const f = state.fields[state.type];
  const node = multiline
    ? el('textarea', { class: 'plain', rows, placeholder, spellcheck: 'false' })
    : el('input', { type, placeholder, autocomplete, spellcheck: 'false' });
  node.value = f[key] || '';
  node.addEventListener('input', () => {
    f[key] = node.value;
    scheduleUpdate();
  });
  return label ? el('label', { class: 'field' }, [el('span', {}, label), node]) : node;
}

function fieldsFor(type) {
  const f = state.fields[type];
  switch (type) {
    case 'wifi': {
      const pw = el('input', { type: 'password', autocomplete: 'off', placeholder: t('wifi.passwordPh') });
      pw.value = f.password;
      pw.addEventListener('input', () => {
        f.password = pw.value;
        scheduleUpdate();
      });
      const eye = el('button', { class: 'icon', type: 'button', title: t('wifi.show') }, '👁');
      eye.addEventListener('click', () => {
        pw.type = pw.type === 'password' ? 'text' : 'password';
        eye.title = pw.type === 'password' ? t('wifi.show') : t('wifi.hide');
      });
      const open = f.security === 'nopass';
      return [
        input('ssid', { label: t('wifi.ssid'), placeholder: t('wifi.ssidPh') }),
        el('label', { class: 'field' }, [el('span', {}, t('wifi.password')), el('div', { class: 'pw' }, [pw, eye])]),
        el('div', { class: 'list' }, [
          row(
            t('wifi.security'),
            null,
            segmented(
              [
                ['WPA', 'WPA', t('wifi.wpaTitle')],
                ['WEP', 'WEP'],
                ['nopass', t('wifi.open')]
              ],
              f.security,
              (security) => {
                f.security = security;
                renderEditor();
                update();
              }
            )
          ),
          row(
            t('wifi.hidden'),
            t('wifi.hiddenDesc'),
            toggle(f.hidden, (on) => {
              f.hidden = on;
              update();
            })
          )
        ]),
        open ? el('div', { class: 'hint' }, t('wifi.openHint')) : null
      ].filter(Boolean);
    }
    case 'email':
      return [
        input('to', { type: 'email', label: t('email.to'), placeholder: 'name@example.com' }),
        input('subject', { label: t('email.subject') }),
        input('body', { label: t('email.body'), multiline: true })
      ];
    case 'tel':
      return [
        input('number', { type: 'tel', label: t('tel.number'), placeholder: '+49 170 1234567' }),
        el('div', { class: 'hint' }, t('tel.hint'))
      ];
    case 'contact':
      return [
        el('div', { class: 'grid' }, [
          input('first', { label: t('contact.first') }),
          input('last', { label: t('contact.last') })
        ]),
        input('org', { label: t('contact.org') }),
        el('div', { class: 'grid' }, [
          input('phone', { type: 'tel', label: t('contact.phone'), placeholder: '+49 …' }),
          input('email', { type: 'email', label: t('contact.email'), placeholder: 'name@example.com' })
        ]),
        input('url', { type: 'url', label: t('contact.url'), placeholder: 'https://' })
      ];
    default: {
      const area = input('text', { multiline: true, rows: state.wide ? 5 : 3, placeholder: t('text.placeholder') });
      const counter = el('span', { class: 'hint', id: 'counter' });
      const foot = el('div', { class: 'field-foot' }, [counter]);
      if (state.tabUrl) {
        foot.append(
          el(
            'button',
            {
              class: 'btn ghost small',
              type: 'button',
              title: state.tabUrl,
              onclick: () => {
                f.text = state.tabUrl;
                area.value = state.tabUrl;
                update();
              }
            },
            t('text.thisPage')
          )
        );
      }
      return [area, foot];
    }
  }
}

/* ----------------------------- Vorschau ------------------------------ */

let canvas = null;
let stage = null;
let meta = null;
let buttons = {};

function renderPreview() {
  const panel = $('previewPanel');
  canvas = el('canvas');
  stage = el('div', { class: 'qr-card' });
  meta = el('div', { class: 'qr-meta' });

  buttons = {
    png: el('button', { class: 'btn primary', type: 'button', onclick: savePng }, t('preview.png')),
    copy: el('button', { class: 'btn', type: 'button', onclick: copyPng, title: t('preview.copyTitle') }, t('preview.copy')),
    svg: el('button', { class: 'btn', type: 'button', onclick: saveSvg, title: t('preview.svgTitle') }, 'SVG')
  };

  panel.replaceChildren(
    el('div', { class: 'panel-body' }, [
      el('div', { class: 'qr-stage' }, stage),
      meta,
      el('div', { class: 'actions' }, [buttons.png, buttons.copy, buttons.svg]),
      el('div', { class: 'note', id: 'colorNote', style: 'margin-top: 12px', hidden: true })
    ])
  );
}

const scheduleUpdate = debounce(update, 90);

/** Payload neu bauen, kodieren, zeichnen. */
function update() {
  state.payload = buildPayload(state.type, state.fields[state.type]);
  state.qr = null;
  state.error = null;
  if (state.payload) {
    try {
      state.qr = encode(state.payload, { ecc: state.settings.ecc });
    } catch (e) {
      state.error = e.message === 'tooLong' ? 'tooLong' : 'failed';
    }
  }
  drawPreview();
}

function drawPreview() {
  if (!stage) return;
  const { qr, settings } = state;

  const counter = $('counter');
  if (counter) {
    const bytes = new TextEncoder().encode(state.payload).length;
    counter.textContent = state.payload ? t('text.count', { n: bytes }) : '';
  }

  if (!qr) {
    const msg = state.error ? t('preview.' + state.error) : t('preview.empty.' + state.type);
    stage.replaceChildren(el('div', { class: 'qr-empty' + (state.error ? ' error' : '') }, msg));
    meta.replaceChildren();
  } else {
    const css = state.wide ? 300 : 208;
    const px = Math.round(css * Math.max(1, window.devicePixelRatio || 1));
    canvas.width = px;
    canvas.height = px;
    drawToContext(canvas.getContext('2d'), qr, px, settings);
    canvas.title = state.payload;
    if (stage.firstChild !== canvas) stage.replaceChildren(canvas);
    meta.replaceChildren(
      el('span', { class: 'chip site' }, t('type.' + state.type)),
      el('span', { class: 'chip' }, t('preview.version', { v: qr.version, n: qr.size })),
      el('span', { class: 'chip', title: t('style.eccTitle') }, t('preview.ecc', { ecc: qr.ecc, p: ECC_PERCENT[qr.ecc] }))
    );
  }

  for (const b of Object.values(buttons)) b.disabled = !qr;

  const note = $('colorNote');
  const warn = qr ? colorWarning(settings.fg, settings.bg) : null;
  note.hidden = !warn;
  if (warn) note.textContent = t('style.warn.' + warn);
}

/* ----------------------------- Export -------------------------------- */

function exportBlob() {
  const size = state.settings.size;
  const c = document.createElement('canvas');
  c.width = size;
  c.height = size;
  drawToContext(c.getContext('2d'), state.qr, size, state.settings);
  return new Promise((resolve, reject) => c.toBlob((b) => (b ? resolve(b) : reject(new Error('png'))), 'image/png'));
}

function remember() {
  if (!state.settings.history || !state.payload) return;
  state.history = store.addHistory(state.history, {
    type: state.type,
    fields: state.fields[state.type],
    payload: state.payload
  });
  store.saveHistory(state.history);
  renderHistory();
}

async function savePng() {
  if (!state.qr) return;
  download(await exportBlob(), fileSlug(state.type, state.fields[state.type]) + '.png');
  flash(buttons.png, t('preview.saved'));
  remember();
}

function saveSvg() {
  if (!state.qr) return;
  const svg = toSvg(state.qr, Object.assign({ px: state.settings.size }, state.settings));
  download(svg, fileSlug(state.type, state.fields[state.type]) + '.svg', 'image/svg+xml');
  flash(buttons.svg, '✓');
  remember();
}

async function copyPng() {
  if (!state.qr) return;
  try {
    await navigator.clipboard.write([new ClipboardItem({ 'image/png': exportBlob() })]);
    flash(buttons.copy, t('preview.copied'));
    remember();
  } catch (e) {
    flash(buttons.copy, t('preview.copyFailed'), 2200);
  }
}

/* ------------------------------- Stil -------------------------------- */

function swatch(key) {
  const value = state.settings[key] === 'transparent' ? '#ffffff' : state.settings[key];
  const inp = el('input', { type: 'color', value, title: t('style.' + key) });
  const wrap = el('label', { class: 'swatch', style: 'background:' + value, title: t('style.' + key) }, inp);
  inp.addEventListener('input', () => {
    wrap.style.background = inp.value;
    setSetting(key, inp.value, { rerender: false });
  });
  return wrap;
}

function renderStyle() {
  const panel = $('stylePanel');
  const s = state.settings;
  const open = state.wide || s.styleOpen;
  panel.classList.toggle('open', open);

  const head = state.wide
    ? el('div', { class: 'panel-head' }, [el('h2', {}, t('style.title')), el('p', {}, t('style.desc'))])
    : el(
        'button',
        {
          class: 'panel-toggle',
          type: 'button',
          'aria-expanded': String(open),
          onclick: () => {
            state.settings = Object.assign({}, state.settings, { styleOpen: !open });
            persistSettings();
            renderStyle();
          }
        },
        [
          el('h2', {}, t('style.title')),
          el('span', { class: 'row' }, [
            el('span', { class: 'hint' }, open ? '' : styleSummary()),
            el('span', { class: 'chev' }, '›')
          ])
        ]
      );

  if (!open) {
    panel.replaceChildren(head);
    return;
  }

  const defaults = store.DEFAULT_SETTINGS;
  const colorsChanged = s.fg !== defaults.fg || s.bg !== defaults.bg;

  panel.replaceChildren(
    head,
    el('div', { class: 'panel-body' }, [
      el('div', { class: 'list' }, [
        row(
          t('style.ecc'),
          t('style.eccDesc'),
          segmented(
            ECC_LEVELS.map((l) => [l, l, ECC_PERCENT[l]]),
            s.ecc,
            (ecc) => setSetting('ecc', ecc)
          )
        ),
        row(
          t('style.size'),
          t('style.sizeDesc'),
          segmented(
            store.SIZES.map((n) => [n, String(n)]),
            s.size,
            (size) => setSetting('size', size)
          )
        ),
        row(t('style.margin'), t('style.marginDesc'), toggle(s.margin, (on) => setSetting('margin', on))),
        row(t('style.rounded'), t('style.roundedDesc'), toggle(s.rounded, (on) => setSetting('rounded', on))),
        row(
          t('style.colors'),
          t('style.colorsDesc'),
          el('div', { class: 'swatches' }, [
            colorsChanged
              ? el(
                  'button',
                  {
                    class: 'btn ghost small',
                    type: 'button',
                    onclick: () => {
                      state.settings = Object.assign({}, state.settings, { fg: defaults.fg, bg: defaults.bg });
                      persistSettings();
                      renderStyle();
                      update();
                    }
                  },
                  t('style.reset')
                )
              : null,
            swatch('fg'),
            swatch('bg')
          ])
        )
      ])
    ])
  );
}

function styleSummary() {
  const s = state.settings;
  return [s.ecc, s.size + ' px', s.rounded ? t('style.roundedShort') : null].filter(Boolean).join(' · ');
}

/* ------------------------------ Verlauf ------------------------------ */

function loadEntry(entry) {
  state.type = entry.type;
  state.fields[entry.type] = Object.assign(emptyFields(entry.type), entry.fields);
  renderEditor();
  update();
  if (state.wide) window.scrollTo({ top: 0, behavior: 'smooth' });
}

function formatTime(at) {
  try {
    return new Date(at).toLocaleString(language(), { dateStyle: 'medium', timeStyle: 'short' });
  } catch (e) {
    return '';
  }
}

function renderHistory() {
  const panel = $('historyPanel');
  const all = state.history;
  const list = state.wide ? all : all.slice(0, POPUP_HISTORY);

  const headRight = el('div', { class: 'row' });
  if (!state.wide && all.length > POPUP_HISTORY) {
    headRight.append(
      el('button', { class: 'btn ghost small', type: 'button', onclick: () => openWideView('#history') }, t('history.all', { n: all.length }))
    );
  }
  if (state.wide && all.length) {
    headRight.append(
      el(
        'button',
        {
          class: 'btn danger small',
          type: 'button',
          onclick: (e) => {
            const b = e.currentTarget;
            // Zweistufig statt confirm(): erst fragen, dann löschen.
            if (b.dataset.armed) {
              state.history = [];
              store.saveHistory([]);
              renderHistory();
              return;
            }
            b.dataset.armed = '1';
            b.textContent = t('history.clearConfirm');
            setTimeout(() => {
              if (b.isConnected) {
                delete b.dataset.armed;
                b.textContent = t('history.clear');
              }
            }, 3000);
          }
        },
        t('history.clear')
      )
    );
  }

  let body;
  if (!list.length) {
    body = el('div', { class: 'list' }, el('div', { class: 'empty' }, state.settings.history ? t('history.empty') : t('history.off')));
  } else {
    body = el(
      'div',
      { class: 'list' },
      list.map((entry) =>
        el(
          'div',
          {
            class: 'item',
            title: entry.payload,
            role: 'button',
            tabindex: '0',
            onclick: () => loadEntry(entry),
            onkeydown: (e) => {
              if (e.key === 'Enter') loadEntry(entry);
            }
          },
          [
            el('span', { class: 'chip' }, t('type.' + entry.type)),
            el('div', { class: 'txt' }, [
              el('div', { class: 't' }, summary(entry.type, entry.fields) || entry.payload),
              el('div', { class: 'd' }, formatTime(entry.at))
            ]),
            el(
              'button',
              {
                class: 'icon',
                type: 'button',
                title: t('history.remove'),
                onclick: (e) => {
                  e.stopPropagation();
                  state.history = store.removeHistory(state.history, entry.id);
                  store.saveHistory(state.history);
                  renderHistory();
                }
              },
              '✕'
            )
          ]
        )
      )
    );
  }

  panel.replaceChildren(
    el('div', { class: 'panel-head' }, [
      el('div', { class: 'row between' }, [el('h2', {}, t('history.title')), headRight]),
      state.wide ? el('p', {}, t('history.desc')) : null
    ]),
    el('div', { class: 'panel-body history' }, body)
  );
}

/* ------------------------- Einstellungen (groß) ----------------------- */

function renderSettings() {
  const panel = $('settingsPanel');
  if (!panel) return;
  panel.replaceChildren(
    el('div', { class: 'panel-head' }, el('h2', {}, t('settings.title'))),
    el('div', { class: 'panel-body' }, [
      el('div', { class: 'list' }, [
        row(
          t('settings.history'),
          t('settings.historyDesc'),
          toggle(state.settings.history, (on) => {
            state.settings = Object.assign({}, state.settings, { history: on });
            persistSettings();
            renderHistory();
          })
        ),
        row(
          t('settings.shortcut'),
          t('settings.shortcutDesc'),
          el(
            'button',
            {
              class: 'btn small',
              type: 'button',
              onclick: () => chrome.tabs.create({ url: 'chrome://extensions/shortcuts' })
            },
            t('settings.shortcutBtn')
          )
        ),
        row(t('settings.menu'), t('settings.menuDesc'), null)
      ])
    ])
  );
}

/* Chroofer - by herrlamatv : Popup */

import { send, el, createProfileEditor, segmented, toggleRow, copyToClipboard, errorText } from './common.js';
import { profileLabel } from '../src/profiles.js';
import { t, setLanguage, applyStatic } from './i18n.js';

const $ = (id) => document.getElementById(id);

let state = null;
let editor = null;

applyStatic();

/* Die Regel, die wirklich greift - bei einer geerbten Regel (example.com deckt
   www.example.com mit ab) bearbeiten wir genau diese, nicht eine neue. */
function targetHost() {
  return state.entry ? state.entry.pattern : state.host;
}

function currentMode() {
  if (!state.entry) return 'inherit';
  return state.entry.mode === 'off' ? 'off' : 'spoof';
}

const label = (profile) => profileLabel(profile, t('profile.customUA'));

async function setMode(mode) {
  const host = targetHost();
  if (mode === 'inherit') await send('removeSite', { host });
  else if (mode === 'off') await send('setSite', { host, patch: { enabled: true, mode: 'off' } });
  else {
    const base = (state.entry && state.entry.profile) || state.effective || state.settings.global.profile;
    await send('setSite', { host, patch: { enabled: true, mode: 'spoof', profile: base } });
  }
  await refresh();
}

function patchSite(patch) {
  return send('setSite', { host: targetHost(), patch }).then(refresh);
}

/* ------------------------------- Rendern ------------------------------ */

function renderStatus() {
  const { settings, inherited, inheritedFrom, entry } = state;

  if (!settings.enabled) {
    $('chip').textContent = t('popup.chipOff');
    $('chip').className = 'chip off';
    $('status').textContent = t('popup.statusDisabled');
    return;
  }

  const active = !!state.effective;
  const bySite = !!entry || inherited;

  $('chip').textContent = active ? (bySite ? t('popup.chipSite') : t('popup.chipGlobal')) : t('popup.chipOff');
  $('chip').className = 'chip ' + (active ? (bySite ? 'site' : 'on') : 'off');

  if (inherited) {
    $('status').textContent = t('popup.statusRedirect', { host: inheritedFrom });
  } else if (entry && entry.mode === 'off') {
    $('status').textContent = t('popup.statusSiteOff');
  } else if (entry) {
    $('status').textContent =
      entry.pattern === state.host
        ? t('popup.statusOwn', { profile: label(state.effective) })
        : t('popup.statusInherited', { pattern: entry.pattern, profile: label(state.effective) });
  } else if (!active) {
    $('status').textContent = t('popup.statusGlobalOff');
  } else {
    $('status').textContent = t('popup.statusGlobal', { profile: label(state.effective) });
  }
}

function scopeList(entry) {
  return el('div', { class: 'list' }, [
    toggleRow(
      t('scope.subdomains'),
      t('scope.subdomainsShort', { pattern: entry.pattern }),
      entry.subdomains,
      (v) => patchSite({ subdomains: v })
    ),
    toggleRow(t('scope.redirects'), t('scope.redirectsDesc'), entry.redirects, (v) => patchSite({ redirects: v })),
    toggleRow(t('scope.thirdParty'), t('scope.thirdPartyDesc'), entry.thirdParty, (v) => patchSite({ thirdParty: v }))
  ]);
}

function renderDetail() {
  const holder = $('detail');
  holder.replaceChildren();

  const { settings, entry } = state;
  if (!settings.enabled) return;

  const mode = currentMode();

  if (mode === 'spoof' && entry) {
    editor.set(entry.profile);
    holder.append(
      el('div', { class: 'spacer' }),
      editor.el,
      el('div', { class: 'spacer' }),
      el('div', { class: 'label', style: 'margin-bottom:8px' }, t('scope.title')),
      scopeList(entry)
    );
    return;
  }

  if (mode === 'off') {
    holder.append(el('div', { class: 'spacer' }), el('div', { class: 'note' }, t('popup.offNote')));
    return;
  }

  /* Global */
  const ua = state.effectiveUA;
  holder.append(
    el('div', { class: 'spacer' }),
    el('div', { class: 'row between' }, [
      el('span', { class: 'label' }, ua ? t('editor.sentUA') : t('popup.thisSite')),
      ua
        ? el(
            'button',
            { class: 'btn ghost small', type: 'button', onclick: (ev) => copyToClipboard(ua, ev.target) },
            t('editor.copy')
          )
        : null
    ]),
    el('div', { style: 'height:6px' }),
    el('div', { class: ua ? 'mono ua-box' : 'note' }, ua || t('popup.nothingChanged'))
  );
}

function render() {
  const { settings, supported } = state;

  $('master').checked = settings.enabled;
  document.body.classList.toggle('dimmed', !settings.enabled);

  if (!supported) {
    $('host').textContent = t('popup.noSite');
    $('chip').textContent = '–';
    $('chip').className = 'chip off';
    $('status').textContent = t('popup.unsupported');
    $('modes').replaceChildren();
    $('detail').replaceChildren();
    $('reload').disabled = true;
    $('check').disabled = true;
    return;
  }

  $('host').textContent = state.host;
  $('host').title = state.host;
  renderStatus();

  $('modes').replaceChildren(
    segmented(
      [
        ['inherit', t('popup.modeGlobal')],
        ['spoof', t('popup.modeOwn')],
        ['off', t('popup.modeOff')]
      ],
      currentMode(),
      setMode,
      !settings.enabled
    )
  );

  renderDetail();
}

async function refresh() {
  state = await send('describeTab', {});
  /* Sprache steht in den Einstellungen - weicht sie von der vorab geladenen ab,
     werden die festen Texte im HTML noch einmal ersetzt. */
  if (setLanguage(state.settings.options.language)) applyStatic();
  render();
}

/* ------------------------------ Verdrahten ---------------------------- */

editor = createProfileEditor((profile) => patchSite({ mode: 'spoof', profile }));

$('master').addEventListener('change', async (ev) => {
  await send('setEnabled', { value: ev.target.checked });
  await refresh();
});

$('reload').addEventListener('click', async () => {
  if (state && state.tabId != null) {
    await chrome.tabs.reload(state.tabId, { bypassCache: true });
    window.close();
  }
});

/* Meist stehen hier API-Namen, die in jeder Sprache gleich heissen. */
const LIVE_ROWS = [
  ['userAgent', () => 'navigator.userAgent'],
  ['platform', () => 'navigator.platform'],
  ['vendor', () => 'navigator.vendor'],
  ['languages', () => 'navigator.languages'],
  ['uaPlatform', () => 'uaData.platform'],
  ['uaMobile', () => 'uaData.mobile'],
  ['brands', () => 'uaData.brands'],
  ['screen', () => t('live.screen')],
  ['cores', () => 'hardwareConcurrency'],
  ['memory', () => 'deviceMemory'],
  ['touch', () => 'maxTouchPoints'],
  ['timezone', () => t('live.timezone')]
];

$('check').addEventListener('click', async () => {
  if (!state || state.tabId == null) return;
  const list = $('live');
  list.replaceChildren(el('dt', {}, t('popup.statusLabel')), el('dd', {}, t('popup.reading')));
  $('livePanel').hidden = false;
  try {
    const live = await send('readLive', { tabId: state.tabId });
    list.replaceChildren();
    for (const [key, name] of LIVE_ROWS) {
      const value = live ? live[key] : null;
      list.append(
        el('dt', {}, name()),
        el('dd', {}, value === null || value === undefined || value === '' ? '–' : String(value))
      );
    }
  } catch (err) {
    list.replaceChildren(el('dt', {}, t('error.label')), el('dd', {}, errorText(err)));
  }
});

$('openOptions').addEventListener('click', (ev) => {
  ev.preventDefault();
  chrome.runtime.openOptionsPage();
});

refresh().catch((err) => {
  $('status').textContent = t('error.generic', { message: errorText(err) });
});

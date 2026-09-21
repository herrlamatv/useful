/* Chroofer - by herrlamatv
 * Gemeinsame Bausteine für Popup und Optionen.
 */

import { OS_FAMILIES, BROWSERS, DEFAULT_PROFILE, buildUserAgent } from '../src/profiles.js';
import { t } from './i18n.js';

export function send(type, payload = {}) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(Object.assign({ type }, payload), (res) => {
      const err = chrome.runtime.lastError;
      if (err) return reject(new Error(err.message));
      if (!res) return reject(new Error(t('error.noResponse')));
      if (!res.ok) return reject(new Error(res.error));
      resolve(res.data);
    });
  });
}

/** Fehlertexte aus dem Hintergrunddienst kommen als Code an. */
export function errorText(err) {
  const raw = String((err && err.message) || err);
  const known = t('error.' + raw);
  return known === 'error.' + raw ? raw : known;
}

export function el(tag, attrs = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') node.className = v;
    else if (k === 'text') node.textContent = v;
    else if (k.startsWith('on') && typeof v === 'function') node.addEventListener(k.slice(2), v);
    else if (v !== null && v !== undefined && v !== false) node.setAttribute(k, v === true ? '' : v);
  }
  for (const child of [].concat(children)) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child.nodeType ? child : document.createTextNode(String(child)));
  }
  return node;
}

/* --------------------------- iOS-Bausteine --------------------------- */

/** Schalter im iOS-Stil. */
export function toggle(checked, onChange, { disabled = false, title = '' } = {}) {
  const input = el('input', { type: 'checkbox', checked: !!checked, disabled, title });
  input.addEventListener('change', () => onChange(input.checked));
  return el('label', { class: 'toggle' }, [input, el('span', { class: 'track' })]);
}

/** Zeile mit Titel, Beschreibung und Bedienelement rechts. */
export function row(title, description, control) {
  return el('div', { class: 'item' }, [
    el('div', { class: 'txt' }, [
      el('div', { class: 't' }, title),
      description ? el('div', { class: 'd' }, description) : null
    ]),
    control
  ]);
}

/** Zeile mit Schalter. */
export function toggleRow(title, description, checked, onChange, opts) {
  return row(title, description, toggle(checked, onChange, opts));
}

/** Segmentierte Auswahl. `items` = [[wert, beschriftung], …] */
export function segmented(items, current, onPick, disabled = false) {
  const wrap = el('div', { class: 'seg' });
  for (const [value, label, title] of items) {
    wrap.append(
      el(
        'button',
        {
          type: 'button',
          disabled,
          title: title || null,
          'aria-pressed': String(value === current),
          onclick: () => {
            if (value !== current) onPick(value);
          }
        },
        label
      )
    );
  }
  return wrap;
}

/* --------------------------- Profil-Editor --------------------------- */

function fillOptions(select, entries, selected) {
  select.replaceChildren();
  for (const [value, label] of entries) {
    select.append(el('option', { value, selected: value === selected ? true : null }, label));
  }
  if (!entries.some(([v]) => v === selected) && entries.length) select.value = entries[0][0];
}

let editorCount = 0;

/**
 * Editor für ein Profil: Betriebssystem, OS-Version, Browser, Browser-Version
 * und optional ein eigener User-Agent. Liefert { el, get, set, retranslate }.
 */
export function createProfileEditor(onChange) {
  const listId = 'chroofer-versions-' + ++editorCount;
  const osSel = el('select');
  const osVerSel = el('select');
  const browserSel = el('select');
  const verInput = el('input', { type: 'text', list: listId });
  const verList = el('datalist', { id: listId });

  const customToggleInput = el('input', { type: 'checkbox' });
  const customArea = el('textarea', { hidden: true });
  const preview = el('div', { class: 'mono ua-box' });

  const labelOs = el('span');
  const labelOsVer = el('span');
  const labelBrowser = el('span');
  const labelBrowserVer = el('span');
  const labelSentUA = el('div', { class: 'label' });
  const labelCustom = el('div', { class: 't' });
  const labelCustomDesc = el('div', { class: 'd' });

  let profile = Object.assign({}, DEFAULT_PROFILE);
  let silent = false;
  let customOpen = false;

  const osEntries = Object.entries(OS_FAMILIES).map(([id, fam]) => [id, fam.label]);
  const browserEntries = Object.entries(BROWSERS).map(([id, br]) => [id, br.label]);

  /** Alle Beschriftungen neu setzen - nach einem Sprachwechsel. */
  function retranslate() {
    labelOs.textContent = t('editor.os');
    labelOsVer.textContent = t('editor.osVersion');
    labelBrowser.textContent = t('editor.browser');
    labelBrowserVer.textContent = t('editor.browserVersion');
    labelSentUA.textContent = t('editor.sentUA');
    labelCustom.textContent = t('editor.customUA');
    labelCustomDesc.textContent = t('editor.customUADesc');
    verInput.placeholder = t('editor.versionPlaceholder');
    customArea.placeholder = t('editor.customUAPlaceholder');
  }

  function render() {
    fillOptions(osSel, osEntries, profile.os);
    fillOptions(browserSel, browserEntries, profile.browser);

    const fam = OS_FAMILIES[profile.os] || OS_FAMILIES.windows;
    fillOptions(
      osVerSel,
      fam.versions.map((v) => [v.id, v.label]),
      profile.osVersion
    );
    profile.osVersion = osVerSel.value;

    const br = BROWSERS[profile.browser] || BROWSERS.chrome;
    verList.replaceChildren(...br.versions.map((v) => el('option', { value: v })));
    verInput.value = profile.browserVersion;

    customOpen = customOpen || !!profile.customUA;
    customToggleInput.checked = customOpen;
    customArea.hidden = !customOpen;
    customArea.value = profile.customUA || '';

    preview.textContent = buildUserAgent(profile);
    retranslate();
  }

  function changed() {
    render();
    if (!silent && onChange) onChange(get());
  }

  osSel.addEventListener('change', () => {
    profile.os = osSel.value;
    profile.osVersion = OS_FAMILIES[profile.os].versions[0].id;
    changed();
  });

  osVerSel.addEventListener('change', () => {
    profile.osVersion = osVerSel.value;
    changed();
  });

  browserSel.addEventListener('change', () => {
    profile.browser = browserSel.value;
    profile.browserVersion = BROWSERS[profile.browser].versions[0];
    changed();
  });

  const commitVersion = () => {
    const clean = verInput.value.replace(/[^0-9.]/g, '');
    const next = clean || BROWSERS[profile.browser].versions[0];
    if (next === profile.browserVersion) return;
    profile.browserVersion = next;
    changed();
  };
  verInput.addEventListener('change', commitVersion);
  verInput.addEventListener('blur', commitVersion);

  customToggleInput.addEventListener('change', () => {
    customOpen = customToggleInput.checked;
    customArea.hidden = !customOpen;
    if (!customOpen && profile.customUA) {
      profile.customUA = '';
      changed();
    } else if (customOpen) {
      customArea.focus();
    } else {
      render();
    }
  });

  customArea.addEventListener('change', () => {
    const next = customArea.value.trim();
    if (next === (profile.customUA || '')) return;
    profile.customUA = next;
    changed();
  });

  function get() {
    return Object.assign({}, profile);
  }

  function set(next, quiet = true) {
    silent = quiet;
    profile = Object.assign({}, DEFAULT_PROFILE, next || {});
    customOpen = !!profile.customUA;
    render();
    silent = false;
  }

  const root = el('div', {}, [
    el('div', { class: 'grid' }, [
      el('label', { class: 'field' }, [labelOs, osSel]),
      el('label', { class: 'field' }, [labelOsVer, osVerSel]),
      el('label', { class: 'field' }, [labelBrowser, browserSel]),
      el('label', { class: 'field' }, [labelBrowserVer, verInput])
    ]),
    verList,
    el('div', { class: 'spacer' }),
    labelSentUA,
    el('div', { style: 'height:6px' }),
    preview,
    el('div', { style: 'height:8px' }),
    el('div', { class: 'list' }, [
      el('div', { class: 'item' }, [
        el('div', { class: 'txt' }, [labelCustom, labelCustomDesc]),
        el('label', { class: 'toggle' }, [customToggleInput, el('span', { class: 'track' })])
      ])
    ]),
    customArea
  ]);

  render();
  return { el: root, get, set, retranslate };
}

export function copyToClipboard(text, button) {
  navigator.clipboard.writeText(text).then(
    () => {
      const old = button.textContent;
      button.textContent = t('editor.copied');
      setTimeout(() => (button.textContent = old), 1200);
    },
    () => {}
  );
}

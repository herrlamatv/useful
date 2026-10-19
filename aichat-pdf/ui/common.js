/* ai chat exporter - by herrlamatv
 * Gemeinsame Bausteine für Popup, Optionen, PDF- und Bulk-Seite.
 */

import { t } from './i18n.js';

export function send(type, payload = {}) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(Object.assign({ type }, payload), (res) => {
      const err = chrome.runtime.lastError;
      if (err) return reject(new Error(err.message));
      if (!res) return reject(new Error('no-response'));
      if (!res.ok) return reject(new Error(res.error));
      resolve(res.data);
    });
  });
}

/** Direkt an das Content-Script eines Tabs (Bulk-Seite). */
export function sendTab(tabId, type, payload = {}) {
  return new Promise((resolve, reject) => {
    chrome.tabs.sendMessage(tabId, Object.assign({ type }, payload), (res) => {
      const err = chrome.runtime.lastError;
      if (err) return reject(new Error('no-response'));
      if (!res) return reject(new Error('no-response'));
      if (!res.ok) return reject(new Error(res.error));
      resolve(res.data);
    });
  });
}

/** Fehlertexte aus Service Worker / Content-Script kommen als Code an. */
export function errorText(err) {
  const raw = String((err && err.message) || err);
  const known = t('error.' + raw);
  return known === 'error.' + raw ? t('error.generic', { message: raw }) : known;
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

/** Rundes Häkchen (wie im Pearltrees Cloner). */
export function check(checked, onChange) {
  const input = el('input', { type: 'checkbox', checked: !!checked });
  input.addEventListener('change', () => onChange(input.checked));
  input.addEventListener('click', (ev) => ev.stopPropagation());
  return el('label', { class: 'check' }, [input, el('span', { class: 'box' })]);
}

/** Zeile mit Titel, Beschreibung und Bedienelement rechts. */
export function row(title, description, control) {
  return el('div', { class: 'item' }, [
    el('div', { class: 'txt' }, [el('div', { class: 't' }, title), description ? el('div', { class: 'd' }, description) : null]),
    control
  ]);
}

export function toggleRow(title, description, checked, onChange, opts) {
  return row(title, description, toggle(checked, onChange, opts));
}

/** Segmentierte Auswahl. `items` = [[wert, beschriftung, title?], …] */
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

/** Sprachumschalter Auto · DE · EN. */
export function languageSwitch(current, onPick) {
  return segmented(
    [
      ['auto', t('lang.auto')],
      ['de', 'DE'],
      ['en', 'EN']
    ],
    current,
    onPick
  );
}

/** Blob herunterladen - mit Unterordner und „Speichern unter“ wie gewünscht. */
export async function downloadBlob(blob, filename, { saveAs = false } = {}) {
  const url = URL.createObjectURL(blob);
  try {
    const id = await chrome.downloads.download({ url, filename, saveAs, conflictAction: 'uniquify' });
    await new Promise((resolve) => {
      const done = (delta) => {
        if (delta.id !== id || !delta.state || delta.state.current === 'in_progress') return;
        chrome.downloads.onChanged.removeListener(done);
        resolve();
      };
      chrome.downloads.onChanged.addListener(done);
      setTimeout(resolve, 120000);
    });
    return id;
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 10000);
  }
}

/* dlplist - by herrlamatv
 * Gemeinsame Bausteine für Popup und Export-Seite (Basis aus chroofer).
 */

import { t, language } from './i18n.js';
import { thumbUrl } from '../src/links.js';

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

export function el(tag, attrs = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') node.className = v;
    else if (k === 'text') node.textContent = v;
    else if (k === 'value') node.value = v;
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
  const input = el('input', { type: 'checkbox', disabled, title: title || null });
  input.checked = !!checked;
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

/** Segmentierte Auswahl. `items` = [[wert, beschriftung, tooltip?], …] */
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

/** Auswahlliste. `entries` = [[wert, beschriftung], …] */
export function select(entries, current, onPick, attrs = {}) {
  const node = el('select', attrs);
  for (const [value, label] of entries) {
    const opt = el('option', { value }, label);
    if (value === current) opt.selected = true;
    node.append(opt);
  }
  node.addEventListener('change', () => onPick(node.value));
  return node;
}

/**
 * Textfeld, das beim Tippen (entprellt) und beim Verlassen meldet.
 * Der Fokus bleibt erhalten, weil der Aufrufer das Feld nicht neu baut.
 */
export function input(value, onValue, attrs = {}) {
  const node = el('input', Object.assign({ type: 'text', spellcheck: 'false' }, attrs));
  node.value = value == null ? '' : String(value);
  let timer = null;
  const fire = () => {
    clearTimeout(timer);
    onValue(node.value);
  };
  node.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(fire, 250);
  });
  node.addEventListener('change', fire);
  return node;
}

/** Beschriftetes Feld (klein, grau, darunter das Bedienelement). */
export function field(label, control, attrs = {}) {
  return el('label', Object.assign({ class: 'field' }, attrs), [el('span', {}, label), control]);
}

/* ------------------------------ Diverses ----------------------------- */

export function copyToClipboard(text, button) {
  return navigator.clipboard.writeText(text).then(
    () => {
      if (!button) return true;
      const old = button.textContent;
      button.textContent = t('common.copied');
      setTimeout(() => (button.textContent = old), 1200);
      return true;
    },
    () => false
  );
}

/** Vorschaubild für einen Eintrag: YouTube-Thumbnail oder Buchstaben-Kachel. */
export function thumb(item) {
  const src = item.status === 'unavailable' ? null : thumbUrl(item);
  if (src) {
    const img = el('img', { class: 'thumb', alt: '', loading: 'lazy', referrerpolicy: 'no-referrer' });
    img.src = src;
    img.addEventListener('error', () => img.replaceWith(letterTile(item)), { once: true });
    return img;
  }
  return letterTile(item);
}

function letterTile(item) {
  let host = '';
  try {
    host = new URL(item.url).hostname.replace(/^(www|m|mobile)\./, '');
  } catch (e) {}
  return el('div', { class: 'thumb glyph', 'aria-hidden': 'true' }, (host[0] || '?').toUpperCase());
}

/** Anzeige-Zeile unter dem Titel: Kanal · Host/Pfad */
export function itemSubline(item) {
  let where = item.url;
  try {
    const u = new URL(item.url);
    where = u.hostname.replace(/^www\./, '') + u.pathname + u.search;
  } catch (e) {}
  return item.author ? item.author + ' · ' + where : where;
}

export function itemTitle(item) {
  return item.title || item.url;
}

/** "vor 3 Min." – für den Verlauf. */
export function relTime(ts) {
  const diff = Math.max(0, Date.now() - ts);
  const min = Math.round(diff / 60000);
  if (min < 1) return t('time.now');
  if (min < 60) return t('time.min', { n: min });
  const h = Math.round(min / 60);
  if (h < 24) return t('time.hours', { n: h });
  return new Date(ts).toLocaleDateString(language() === 'de' ? 'de-DE' : 'en-GB');
}

/** Knopf, der erst nach einem zweiten Klick auslöst (statt confirm()). */
export function confirmButton(label, confirmLabel, onConfirm, cls = 'btn danger small') {
  const btn = el('button', { class: cls, type: 'button' }, label);
  let armed = false;
  let timer = null;
  btn.addEventListener('click', () => {
    if (!armed) {
      armed = true;
      btn.textContent = confirmLabel;
      timer = setTimeout(() => {
        armed = false;
        btn.textContent = label;
      }, 3000);
      return;
    }
    clearTimeout(timer);
    armed = false;
    btn.textContent = label;
    onConfirm();
  });
  return btn;
}

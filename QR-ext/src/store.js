/* QR-ext - by herrlamatv
 * Einstellungen und Verlauf. Die Logik ist rein (testbar in Node), die
 * chrome.storage-Aufrufe stehen gesammelt unten.
 *
 *   Einstellungen -> chrome.storage.sync  (wandern mit dem Google-Konto)
 *   Verlauf       -> chrome.storage.local (bleibt auf diesem Rechner)
 *   Rechtsklick   -> chrome.storage.session 'pending' (bis das Popup ihn abholt)
 */

import { TYPES, emptyFields } from './payload.js';
import { ECC_LEVELS } from './qr.js';

export const SIZES = [256, 512, 1024];
export const HISTORY_MAX = 30;
export const PENDING_MAX_AGE = 5 * 60 * 1000;

export const DEFAULT_SETTINGS = {
  ecc: 'M',
  size: 512,
  margin: true,
  rounded: false,
  fg: '#1f1d18',
  bg: '#ffffff',
  lang: 'auto',
  history: true,
  styleOpen: false
};

const HEX = /^#[0-9a-f]{6}$/i;

/** Gespeicherte (evtl. alte oder kaputte) Einstellungen säubern. */
export function mergeSettings(raw) {
  const s = Object.assign({}, DEFAULT_SETTINGS);
  if (!raw || typeof raw !== 'object') return s;
  if (ECC_LEVELS.includes(raw.ecc)) s.ecc = raw.ecc;
  if (SIZES.includes(Number(raw.size))) s.size = Number(raw.size);
  if (typeof raw.margin === 'boolean') s.margin = raw.margin;
  if (typeof raw.rounded === 'boolean') s.rounded = raw.rounded;
  if (HEX.test(raw.fg)) s.fg = raw.fg.toLowerCase();
  if (HEX.test(raw.bg) || raw.bg === 'transparent') s.bg = raw.bg.toLowerCase();
  if (['auto', 'de', 'en'].includes(raw.lang)) s.lang = raw.lang;
  if (typeof raw.history === 'boolean') s.history = raw.history;
  if (typeof raw.styleOpen === 'boolean') s.styleOpen = raw.styleOpen;
  return s;
}

/** Nur bekannte Felder der Vorlage behalten, alles als String/Boolean. */
export function cleanFields(type, fields) {
  const base = emptyFields(type);
  for (const key of Object.keys(base)) {
    const v = fields && fields[key];
    if (v === undefined || v === null) continue;
    base[key] = typeof base[key] === 'boolean' ? !!v : String(v);
  }
  return base;
}

/**
 * Eintrag vorne einfügen. Gleicher Inhalt (payload) wandert nach oben statt
 * doppelt zu erscheinen. Liefert eine neue Liste.
 */
export function addHistory(list, { type, fields, payload, at = Date.now() }, max = HISTORY_MAX) {
  if (!payload || !TYPES.includes(type)) return Array.isArray(list) ? list.slice() : [];
  const entry = { id: String(at) + '-' + Math.random().toString(36).slice(2, 7), type, fields: cleanFields(type, fields), payload, at };
  const rest = (Array.isArray(list) ? list : []).filter((e) => e && e.payload !== payload);
  return [entry].concat(rest).slice(0, max);
}

export function removeHistory(list, id) {
  return (Array.isArray(list) ? list : []).filter((e) => e && e.id !== id);
}

/** Nur gültige Einträge übernehmen (falls der Speicher Müll enthält). */
export function cleanHistory(list) {
  if (!Array.isArray(list)) return [];
  return list
    .filter((e) => e && TYPES.includes(e.type) && typeof e.payload === 'string' && e.payload)
    .map((e) => Object.assign({}, e, { fields: cleanFields(e.type, e.fields) }))
    .slice(0, HISTORY_MAX);
}

/** Rechtsklick-Inhalt nur annehmen, wenn er frisch ist. */
export function takePending(pending, now = Date.now()) {
  if (!pending || typeof pending.text !== 'string' || !pending.text) return null;
  if (!(now - Number(pending.at) <= PENDING_MAX_AGE)) return null;
  return pending.text;
}

/* ----------------------------- chrome.* ------------------------------ */

export async function loadSettings() {
  const { settings } = await chrome.storage.sync.get('settings');
  return mergeSettings(settings);
}

export async function saveSettings(settings) {
  await chrome.storage.sync.set({ settings: mergeSettings(settings) });
}

export async function loadHistory() {
  const { history } = await chrome.storage.local.get('history');
  return cleanHistory(history);
}

export async function saveHistory(list) {
  await chrome.storage.local.set({ history: cleanHistory(list) });
}

export async function setPending(text) {
  await chrome.storage.session.set({ pending: { text, at: Date.now() } });
}

export async function popPending() {
  const { pending } = await chrome.storage.session.get('pending');
  if (pending) await chrome.storage.session.remove('pending');
  return takePending(pending);
}

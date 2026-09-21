/* Chroofer - by herrlamatv
 * Kleine i18n-Schicht für Popup und Optionen.
 *
 * Warum nicht chrome.i18n? Das richtet sich fest nach der Browsersprache und
 * lässt sich zur Laufzeit nicht umstellen. Hier soll man Englisch aber auch
 * auf einem deutschen Chrome sehen können. chrome.i18n bleibt für Name und
 * Beschreibung im Manifest zuständig (_locales/), den Rest macht dieses Modul.
 */

import de from './i18n/de.js';
import en from './i18n/en.js';

export const BUNDLES = { de, en };
export const LANGUAGES = ['auto', 'de', 'en'];

const FALLBACK = 'en';
const PREF_KEY = 'chroofer.lang';

let current = FALLBACK;
let messages = en;
let preference = 'auto';

/** 'auto' | 'de' | 'en'  ->  'de' | 'en' */
export function resolveLanguage(pref) {
  if (pref === 'de' || pref === 'en') return pref;
  let ui = '';
  try {
    ui = (chrome.i18n.getUILanguage() || '').toLowerCase();
  } catch (e) {
    ui = (navigator.language || '').toLowerCase();
  }
  return ui.startsWith('de') ? 'de' : FALLBACK;
}

export function language() {
  return current;
}

export function languagePreference() {
  return preference;
}

/**
 * Sprache setzen. Die Vorgabe landet zusätzlich im localStorage, damit die
 * nächste Seite schon beim ersten Zeichnen richtig steht - ohne Aufflackern.
 * @returns true, wenn sich die tatsächliche Sprache geändert hat
 */
export function setLanguage(pref) {
  preference = LANGUAGES.includes(pref) ? pref : 'auto';
  const next = resolveLanguage(preference);
  try {
    localStorage.setItem(PREF_KEY, preference);
  } catch (e) {}

  const changed = next !== current;
  current = next;
  messages = BUNDLES[next] || en;
  try {
    document.documentElement.lang = next;
  } catch (e) {}
  return changed;
}

export function t(key, vars) {
  let text = messages[key];
  if (text === undefined) text = en[key];
  if (text === undefined) return key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.split('{' + name + '}').join(value);
    }
  }
  return text;
}

/**
 * Einen Text mit genau einem Platzhalter in Knoten zerlegen, damit der
 * eingesetzte Teil eigenes Markup bekommen kann (fett, monospace, …).
 */
export function tParts(key, name, node) {
  const [before, after = ''] = t(key).split('{' + name + '}');
  return [before, node, after];
}

/** Statische Texte im HTML ersetzen: data-i18n, -ph (placeholder), -title. */
export function applyStatic(root = document) {
  for (const node of root.querySelectorAll('[data-i18n]')) {
    node.textContent = t(node.dataset.i18n);
  }
  for (const node of root.querySelectorAll('[data-i18n-ph]')) {
    node.placeholder = t(node.dataset.i18nPh);
  }
  for (const node of root.querySelectorAll('[data-i18n-title]')) {
    node.title = t(node.dataset.i18nTitle);
  }
}

/* Beim Laden sofort die zuletzt gewählte Sprache anwenden - die echte
   Einstellung kommt gleich darauf aus dem Speicher und korrigiert notfalls. */
let stored = 'auto';
try {
  stored = localStorage.getItem(PREF_KEY) || 'auto';
} catch (e) {}
setLanguage(stored);

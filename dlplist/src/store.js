/* dlplist - by herrlamatv
 * Speicher-Schema und reine Listen-Operationen (ohne chrome.* - testbar).
 *
 * chrome.storage.local, mehrere Schlüssel, damit eine neue Zeile nicht jedes
 * Mal die Einstellungen mitschreibt:
 *   dlplist.items     [Eintrag, …]
 *   dlplist.settings  { enabled, language, toast, titles, badge }
 *   dlplist.export    Export-Einstellungen (siehe command.js)
 *   dlplist.presets   eigene Vorlagen
 *   dlplist.history   die letzten Exporte
 */

import { CATS } from './links.js';

export const KEYS = {
  items: 'dlplist.items',
  settings: 'dlplist.settings',
  exp: 'dlplist.export',
  presets: 'dlplist.presets',
  history: 'dlplist.history'
};

export const MAX_ITEMS = 5000;
export const MAX_HISTORY = 10;

export const DEFAULT_SETTINGS = {
  version: 1,
  enabled: true,
  language: 'auto',
  toast: true,
  titles: true,
  badge: true
};

export function mergeSettings(raw) {
  const s = Object.assign({}, DEFAULT_SETTINGS, raw && typeof raw === 'object' ? raw : {});
  for (const k of ['enabled', 'toast', 'titles', 'badge']) s[k] = !!s[k];
  if (!['auto', 'de', 'en'].includes(s.language)) s.language = 'auto';
  return s;
}

/** Neuer Listeneintrag aus einem parseLink()-Ergebnis. */
export function newItem(parsed, now, source) {
  return {
    key: parsed.key,
    url: parsed.url,
    id: parsed.id || null,
    kind: parsed.kind,
    cat: parsed.cat,
    manual: false,
    title: null,
    author: null,
    titleTried: false,
    status: null,
    added: now,
    exported: 0,
    source: source || 'link'
  };
}

/**
 * Geparste Links hinzufügen.
 * - Doppelte (gleicher Schlüssel) bleiben einmal drin.
 * - Kommt ein vorhandenes Video als /shorts/ nochmal, wandert es zu den
 *   Shorts - außer man hat es von Hand verschoben.
 * @returns {{items, added: string[], dupes: string[], moved: string[]}}
 */
export function addParsed(items, parsedList, now, source) {
  const next = items.slice();
  const index = new Map(next.map((it, i) => [it.key, i]));
  const added = [];
  const dupes = [];
  const moved = [];
  for (const p of parsedList) {
    if (!p || !p.ok) continue;
    if (index.has(p.key)) {
      const i = index.get(p.key);
      const cur = next[i];
      if (p.kind === 'short' && cur.cat === 'videos' && !cur.manual) {
        next[i] = Object.assign({}, cur, { kind: 'short', cat: 'shorts', url: p.url });
        moved.push(p.key);
      } else if (!added.includes(p.key)) {
        dupes.push(p.key);
      }
      continue;
    }
    if (next.length >= MAX_ITEMS) break;
    index.set(p.key, next.length);
    next.push(newItem(p, now, source));
    added.push(p.key);
  }
  return { items: next, added, dupes, moved };
}

export function removeKeys(items, keys) {
  const drop = new Set(keys);
  return items.filter((it) => !drop.has(it.key));
}

export function moveKeys(items, keys, cat) {
  if (!CATS.includes(cat)) return items;
  const set = new Set(keys);
  return items.map((it) => (set.has(it.key) ? Object.assign({}, it, { cat, manual: true }) : it));
}

/** at = Zeitstempel oder 0 (= wieder offen). */
export function markExported(items, keys, at) {
  const set = new Set(keys);
  return items.map((it) => (set.has(it.key) ? Object.assign({}, it, { exported: at || 0 }) : it));
}

/** filter: 'all' | 'exported' | 'unavailable' | eine Kategorie */
export function clearItems(items, filter) {
  if (filter === 'all') return [];
  if (filter === 'exported') return items.filter((it) => !it.exported);
  if (filter === 'unavailable') return items.filter((it) => it.status !== 'unavailable');
  if (CATS.includes(filter)) return items.filter((it) => it.cat !== filter);
  return items;
}

/** Ergebnisse vom Titel-Abruf einarbeiten: [{key, title?, author?, status?}] */
export function applyTitles(items, results) {
  const byKey = new Map(results.map((r) => [r.key, r]));
  return items.map((it) => {
    const r = byKey.get(it.key);
    if (!r) return it;
    const next = Object.assign({}, it, { titleTried: true });
    if (r.title) next.title = String(r.title).slice(0, 300);
    if (r.author) next.author = String(r.author).slice(0, 120);
    if (r.status) next.status = r.status;
    return next;
  });
}

/** Einträge, deren Titel noch geholt werden kann. */
export function needsTitle(items, canFetch) {
  return items.filter((it) => it.title == null && !it.titleTried && canFetch(it));
}

export function counts(items) {
  const c = { total: items.length, pending: 0, exported: 0, unavailable: 0 };
  for (const cat of CATS) c[cat] = 0;
  for (const it of items) {
    c[it.cat] = (c[it.cat] || 0) + 1;
    if (it.exported) c.exported++;
    else c.pending++;
    if (it.status === 'unavailable') c.unavailable++;
  }
  return c;
}

/** Badge-Text: 0 → leer, ab 1000 → "1k+". */
export function badgeText(n) {
  if (!n) return '';
  return n > 999 ? '1k+' : String(n);
}

export function addHistory(history, entry) {
  return [entry].concat(Array.isArray(history) ? history : []).slice(0, MAX_HISTORY);
}

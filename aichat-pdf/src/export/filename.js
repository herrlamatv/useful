/* ai chat exporter - by herrlamatv
 * Dateinamen aus Vorlagen bauen - Windows-sicher, weil chrome.downloads bei
 * verbotenen Zeichen sonst mit „Invalid filename“ abbricht.
 *
 * Platzhalter: {platform} {title} {date} {time} {created} {model} {id}
 */

const RESERVED = /^(con|prn|aux|nul|com[0-9]|lpt[0-9])$/i;
/* Steuerzeichen, Windows-Sonderzeichen und unsichtbare Zeichen (Zero-Width, Zeilentrenner, BOM). */
const INVALID = new RegExp('[\\u0000-\\u001f\\u007f<>:"/\\\\|?*\\u200b-\\u200f\\u2028\\u2029\\ufeff]', 'g');

const pad = (n) => String(n).padStart(2, '0');

export function isoDate(date) {
  const d = date instanceof Date ? date : new Date(date);
  if (isNaN(d)) return '';
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}

export function isoTime(date) {
  const d = date instanceof Date ? date : new Date(date);
  if (isNaN(d)) return '';
  return pad(d.getHours()) + '-' + pad(d.getMinutes());
}

/** Ein Pfadstück: verbotene Zeichen raus, keine reservierten Namen, nicht zu lang. */
export function sanitizeSegment(text, max = 100) {
  let s = String(text == null ? '' : text)
    .normalize('NFC')
    .replace(INVALID, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  /* Chrome mag keine Namen, die mit Punkt oder Tilde anfangen oder mit Punkt enden. */
  s = s.replace(/^[.~\s]+/, '').replace(/[.\s]+$/, '');
  if (s.length > max) {
    s = Array.from(s).slice(0, max).join('').replace(/[.\s]+$/, '');
  }
  if (RESERVED.test(s.split('.')[0])) s = '_' + s;
  return s;
}

/** Vorlage füllen. `now` kommt rein, damit Tests feste Zeiten nutzen können. */
export function fillTemplate(template, conv, now = new Date()) {
  const created = conv.createdAt ? new Date(conv.createdAt) : now;
  const values = {
    platform: conv.platformName || conv.platform || 'Chat',
    title: conv.title || 'Chat',
    date: isoDate(now),
    time: isoTime(now),
    created: isoDate(isNaN(created) ? now : created),
    model: conv.model || '',
    id: conv.id || ''
  };
  return String(template || '').replace(/\{(\w+)\}/g, (all, key) =>
    Object.prototype.hasOwnProperty.call(values, key) ? String(values[key]).replace(/[/\\]/g, ' ') : all
  );
}

/**
 * Kompletter relativer Pfad für chrome.downloads, z. B.
 * "AI-Chats/Claude/2026-10-19 Mein Chat.md".
 */
export function buildPath({ folder, file }, conv, ext, now = new Date()) {
  const dirs = fillTemplate(folder || '', conv, now)
    .split(/[/\\]+/)
    .map((part) => sanitizeSegment(part, 60))
    .filter((part) => part && part !== '.' && part !== '..');
  const name = sanitizeSegment(fillTemplate(file || '{title}', conv, now), 120) || 'Chat';
  return dirs.concat(name + (ext ? '.' + ext : '')).join('/');
}

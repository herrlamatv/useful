/* ai chat exporter - by herrlamatv
 * Aus dem, was das Content-Script liefert, wird eine einheitliche
 * Unterhaltung - danach filtern (Auswahl, Rollen, Extras) und an die
 * Exporter geben.
 *
 * raw (vom Content-Script):
 *   { platform, title, url, id, model, dom: [ {id, role, html, md, thinking, sources, attachments, name} ],
 *     api: { kind: 'chatgpt'|'claude', data, assets, origin } | null }
 */

import { parseChatGPT } from './parse/chatgpt.js';
import { parseClaude } from './parse/claude.js';
import { platformById, UNIVERSAL } from './platforms.js';
import { mdToText } from './export/text.js';

/* --------------------------- Schlüssel ------------------------------- */

function fnv(text) {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(36);
}

const plain = (text) => String(text || '').replace(/\s+/g, ' ').trim();

/**
 * Stabiler Schlüssel je Nachricht - Rolle + Anfang des Textes. So passt die
 * Auswahl im Popup auch dann noch, wenn beim Export ältere Nachrichten
 * nachgeladen wurden und sich die Reihenfolge-Nummern verschoben haben.
 */
export function assignKeys(messages) {
  const seen = new Map();
  for (const msg of messages) {
    const base = msg.role + ':' + fnv(plain(mdToText(msg.md)).slice(0, 400));
    const n = seen.get(base) || 0;
    seen.set(base, n + 1);
    msg.key = n ? base + ':' + n : base;
  }
  return messages;
}

/* ------------------------- HTML aus dem DOM -------------------------- */

const htmlText = (html) => plain(String(html || '').replace(/<[^>]+>/g, ' ').replace(/&[a-z#0-9]+;/gi, ' '));

const countImages = (text, re) => (String(text || '').match(re) || []).length;

/** Nur übernehmen, wenn das DOM ungefähr so viel Text (und Bilder) hat wie die API - sonst fehlt da was. */
export function plausible(html, md) {
  const a = htmlText(html).length;
  /* Vergleichbar machen: Bilder raus, Links nur mit ihrem Text (die URL steht im HTML nicht sichtbar). */
  const visible = md.replace(/!\[[^\]]*\]\([^)]*\)/g, '').replace(/\[([^\]]*)\]\((?:[^()\s]|\([^()\s]*\))+\)/g, '$1');
  const b = plain(mdToText(visible)).length;
  if (countImages(html, /<img\b/gi) < countImages(md, /!\[[^\]]*\]\(/g)) return false;
  return a > 0 && a >= b * 0.6;
}

/**
 * Gerendertes HTML von der Seite an die API-Nachrichten hängen (schöner fürs PDF:
 * Mathe, Syntax-Farben). Zuordnung über die Nachrichten-IDs (ChatGPT und Claude
 * schreiben sie ins DOM), sonst über die n-te Nachricht je Rolle.
 *
 * Alles oder nichts: Claude und ChatGPT rendern bei langen Chats nur den
 * sichtbaren Teil. Eine halb aus dem DOM, halb aus Markdown gebaute Datei
 * sähe uneinheitlich aus - dann lieber alles aus dem Markdown.
 */
export function attachDomHtml(apiMessages, domMessages, by = 'id') {
  if (!domMessages || !domMessages.length || !apiMessages.length) return apiMessages;
  const picked = [];
  if (by === 'id') {
    const byId = new Map();
    for (const d of domMessages) for (const id of d.ids || [d.id]) if (id && !byId.has(id)) byId.set(id, d);
    for (const msg of apiMessages) {
      const found = [...new Set((msg.ids || [msg.id]).map((id) => byId.get(id)).filter(Boolean))];
      picked.push(found.map((d) => d.html).filter(Boolean).join('\n'));
    }
  } else {
    const counters = { user: 0, assistant: 0 };
    const byRole = { user: domMessages.filter((m) => m.role === 'user'), assistant: domMessages.filter((m) => m.role === 'assistant') };
    for (const role of ['user', 'assistant']) {
      if (byRole[role].length !== apiMessages.filter((m) => m.role === role).length) return apiMessages;
    }
    for (const msg of apiMessages) picked.push((byRole[msg.role][counters[msg.role]++] || {}).html || '');
  }
  if (!apiMessages.every((msg, i) => picked[i] && plausible(picked[i], msg.md))) return apiMessages;
  apiMessages.forEach((msg, i) => {
    msg.html = picked[i];
  });
  return apiMessages;
}

/* ---------------------------- Aufbauen ------------------------------- */

function normalizeMessage(m) {
  return {
    id: m.id || null,
    ids: Array.isArray(m.ids) && m.ids.length ? m.ids : m.id ? [m.id] : [],
    role: m.role === 'user' ? 'user' : 'assistant',
    name: m.name || null,
    time: m.time || null,
    md: String(m.md || ''),
    html: m.html || '',
    thinking: m.thinking && (m.thinking.md || m.thinking.html) ? { md: m.thinking.md || '', html: m.thinking.html || '' } : null,
    tools: Array.isArray(m.tools) ? m.tools : [],
    sources: Array.isArray(m.sources) ? m.sources.filter((s) => s && s.url) : [],
    attachments: Array.isArray(m.attachments) ? m.attachments.filter((a) => a && a.name) : []
  };
}

export function buildConversation(raw) {
  const platform = platformById(raw.platform) || UNIVERSAL;
  const conv = {
    platform: platform.id,
    platformName: platform.name,
    title: plain(raw.title) || 'Chat',
    url: raw.url || '',
    id: raw.id || '',
    model: raw.model || '',
    createdAt: raw.createdAt || null,
    updatedAt: raw.updatedAt || null,
    source: 'dom',
    messages: []
  };

  let parsed = null;
  if (raw.api && raw.api.data) {
    try {
      if (raw.api.kind === 'chatgpt') parsed = parseChatGPT(raw.api.data, { assets: raw.api.assets });
      else if (raw.api.kind === 'claude') parsed = parseClaude(raw.api.data, { origin: raw.api.origin });
    } catch (e) {
      parsed = null;
    }
  }

  if (parsed && parsed.messages.length) {
    conv.source = 'api';
    conv.title = plain(parsed.title) || conv.title;
    conv.id = parsed.id || conv.id;
    conv.model = parsed.model || conv.model;
    conv.createdAt = parsed.createdAt || conv.createdAt;
    conv.updatedAt = parsed.updatedAt || conv.updatedAt;
    conv.messages = attachDomHtml(parsed.messages.map(normalizeMessage), raw.dom, 'id');
  } else {
    conv.messages = (raw.dom || []).map(normalizeMessage).filter((m) => m.md.trim() || m.html || m.attachments.length);
  }
  assignKeys(conv.messages);
  return conv;
}

/* ----------------------------- Filtern ------------------------------- */

const IMG_MD = /!\[[^\]]*\]\((?:[^()\s]|\([^()\s]*\))+(?:\s+"[^"]*")?\)\s*/g;
const IMG_HTML = /<img\b[^>]*>/gi;

/**
 * keys:  Auswahl aus dem Popup (Array) - null = alles
 * roles: 'all' | 'user' | 'assistant'
 * include: siehe DEFAULT_INCLUDE in settings.js
 */
export function filterConversation(conv, { keys = null, roles = 'all', include = {} } = {}) {
  const wanted = keys && keys.length ? new Set(keys) : null;
  const messages = conv.messages
    .filter((m) => !wanted || wanted.has(m.key))
    .filter((m) => roles === 'all' || m.role === roles)
    .map((m) => {
      const out = Object.assign({}, m);
      if (include.thinking === false) out.thinking = null;
      if (include.sources === false) out.sources = [];
      if (include.attachments === false) out.attachments = [];
      if (include.tools === false) out.tools = [];
      if (include.images === false) {
        out.md = out.md.replace(IMG_MD, '');
        out.html = out.html.replace(IMG_HTML, '');
      }
      return out;
    })
    .filter((m) => m.md.trim() || m.html || m.thinking || m.tools.length || m.attachments.length);
  return Object.assign({}, conv, { messages });
}

/* ------------------------------ Bilder ------------------------------- */

/** Alle Bild-URLs (Markdown und HTML) - zum Einbetten fürs PDF/HTML. */
export function imageUrls(conv) {
  const urls = new Set();
  const add = (u) => {
    if (u && /^(https?|blob):/i.test(u)) urls.add(u);
  };
  for (const m of conv.messages) {
    for (const match of m.md.matchAll(/!\[[^\]]*\]\(((?:[^()\s]|\([^()\s]*\))+)/g)) add(match[1]);
    for (const match of (m.html || '').matchAll(/<img\b[^>]*\bsrc="([^"]+)"/gi)) add(match[1].replace(/&amp;/g, '&'));
  }
  return [...urls];
}

/** URLs durch data:-URLs ersetzen (map: url -> dataUrl). */
export function replaceImageUrls(conv, map) {
  if (!map || !Object.keys(map).length) return conv;
  const swap = (text) => {
    let s = text;
    for (const [url, data] of Object.entries(map)) {
      s = s.split(url).join(data).split(url.replace(/&/g, '&amp;')).join(data);
    }
    return s;
  };
  return Object.assign({}, conv, {
    messages: conv.messages.map((m) => Object.assign({}, m, { md: swap(m.md), html: swap(m.html || '') }))
  });
}

/** Kurzfassung fürs Popup: Rolle, Schlüssel, Vorschau. */
export function summarize(conv) {
  return {
    platform: conv.platform,
    platformName: conv.platformName,
    title: conv.title,
    model: conv.model,
    source: conv.source,
    messages: conv.messages.map((m) => ({
      key: m.key,
      role: m.role,
      preview: plain(mdToText(m.md) || htmlText(m.html)).slice(0, 140),
      thinking: !!m.thinking
    }))
  };
}

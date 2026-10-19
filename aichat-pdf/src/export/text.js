/* ai chat exporter - by herrlamatv
 * Reiner Text: Markdown-Zeichen raus, Inhalt bleibt. Wird für .txt und
 * für das „Raw“-PDF benutzt.
 */

import { isoDate } from './filename.js';

/** Markdown -> lesbarer Text. Code bleibt wörtlich, Links werden „Text (URL)“. */
export function mdToText(markdown) {
  const lines = String(markdown == null ? '' : markdown).replace(/\r\n?/g, '\n').split('\n');
  const out = [];
  let fence = null;
  for (const line of lines) {
    const f = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fence) {
      if (f && f[1][0] === fence[0] && f[1].length >= fence.length && /^\s*[`~]+\s*$/.test(line)) fence = null;
      else out.push('    ' + line);
      continue;
    }
    if (f) {
      fence = f[1];
      continue;
    }
    let s = line;
    if (/^ {0,3}([-*_])(\s*\1){2,}\s*$/.test(s)) {
      out.push('──────────');
      continue;
    }
    if (/^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?\s*$/.test(s)) continue; // Tabellen-Trennzeile
    s = s.replace(/^ {0,3}#{1,6}\s+(.*?)\s*#*\s*$/, '$1');
    s = s
      .replace(/!\[([^\]]*)\]\(([^)\s]+)[^)]*\)/g, (all, alt, url) => '[' + (alt || 'image') + (url.startsWith('data:') ? '' : ': ' + url) + ']')
      .replace(/\[([^\]]+)\]\(([^)\s]+)[^)]*\)/g, (all, text, url) => (text === url ? url : text + ' (' + url + ')'))
      .replace(/`([^`]+)`/g, '$1')
      .replace(/\*\*\*([^*]+)\*\*\*/g, '$1')
      .replace(/\*\*([^*]+)\*\*/g, '$1')
      .replace(/(^|[^\w])__([^_]+)__(?!\w)/g, '$1$2')
      .replace(/(^|[^*\w])\*([^*\s][^*]*?)\*(?!\*)/g, '$1$2')
      .replace(/(^|[^\w])_([^_\s][^_]*?)_(?!\w)/g, '$1$2')
      .replace(/~~([^~]+)~~/g, '$1')
      .replace(/^(\s*)[-*+]\s+\[([ xX])\]\s+/, (all, ind, x) => ind + (x === ' ' ? '☐ ' : '☑ '))
      .replace(/^(\s*)[-*+]\s+/, '$1• ')
      .replace(/\\([\\`*_{}\[\]()#+\-.!|~<>$])/g, '$1');
    if (/^\s*\|.*\|\s*$/.test(s)) s = s.trim().replace(/^\|\s*/, '').replace(/\s*\|$/, '').replace(/\s*\|\s*/g, '  |  ');
    out.push(s.replace(/\s+$/, ''));
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n').trim();
}

function timeText(value) {
  if (!value) return '';
  const d = new Date(value);
  return isNaN(d) ? '' : d.toLocaleString();
}

/** Ganze Unterhaltung als .txt */
export function toText(conv, { labels, include = {} }) {
  const parts = [];
  const meta = [conv.platformName, conv.model, conv.createdAt ? isoDate(conv.createdAt) : ''].filter(Boolean).join(' · ');
  parts.push(conv.title || 'Chat');
  if (meta) parts.push(meta);
  if (conv.url) parts.push(conv.url);
  parts.push('='.repeat(Math.min(60, Math.max(10, (conv.title || 'Chat').length))));

  for (const msg of conv.messages) {
    const who = msg.role === 'user' ? labels.you : msg.name || conv.platformName;
    const when = include.times && msg.time ? ' (' + timeText(msg.time) + ')' : '';
    const block = [who + when + ':'];
    if (msg.attachments && msg.attachments.length) block.push('[' + labels.attachments + ': ' + msg.attachments.map((a) => a.name).join(', ') + ']');
    if (msg.thinking && msg.thinking.md) block.push('[' + labels.thinking + ']\n' + mdToText(msg.thinking.md) + '\n[/' + labels.thinking + ']');
    for (const tool of msg.tools || []) block.push('[' + tool.title + ']\n' + mdToText(tool.md));
    block.push(mdToText(msg.md));
    if (msg.sources && msg.sources.length) {
      block.push(labels.sources + ':\n' + msg.sources.map((s, i) => i + 1 + '. ' + (s.title && s.title !== s.url ? s.title + ' – ' : '') + s.url).join('\n'));
    }
    parts.push(block.join('\n\n'));
    parts.push('-'.repeat(10));
  }
  if (conv.messages.length) parts.pop();
  return parts.join('\n\n') + '\n';
}

/* ai chat exporter - by herrlamatv
 * Markdown-Export. Aufbau so, dass er in Obsidian, GitHub und jedem Editor
 * gut aussieht: Frontmatter, # Titel, ## pro Nachricht, Gedankengang als
 * einklappbares Callout, Quellen als nummerierte Liste.
 */

import { frontmatter } from './frontmatter.js';
import { isoDate } from './filename.js';

function timeText(value) {
  if (!value) return '';
  const d = new Date(value);
  return isNaN(d) ? '' : d.toLocaleString();
}

/** Text als Callout/Zitat einrücken - jede Zeile mit "> ". */
export function quote(text) {
  return String(text)
    .split('\n')
    .map((line) => (line ? '> ' + line : '>'))
    .join('\n');
}

/** Eine Nachricht: „## Wer“, Anhänge, Gedankengang, Inhalt (wörtlich), Quellen. */
export function messageMarkdown(msg, conv, { labels, include = {} }) {
  const who = msg.role === 'user' ? labels.you : msg.name || conv.platformName || 'AI';
  const out = ['## ' + who];
  if (include.times && msg.time) out.push('*' + timeText(msg.time) + '*');
  if (msg.attachments && msg.attachments.length) {
    out.push('*' + labels.attachments + ':* ' + msg.attachments.map((a) => (a.url ? '[' + a.name + '](' + a.url + ')' : a.name)).join(', '));
  }
  if (msg.thinking && msg.thinking.md) out.push(quote('[!note]- ' + labels.thinking + '\n' + msg.thinking.md.trim()));
  for (const tool of msg.tools || []) out.push(quote('[!example]- ' + tool.title + '\n' + tool.md.trim()));
  if (msg.md && msg.md.trim()) out.push(msg.md.trim());
  if (msg.sources && msg.sources.length) {
    out.push('**' + labels.sources + '**\n' + msg.sources.map((s, i) => i + 1 + '. [' + (s.title || s.url).replace(/[[\]]/g, '') + '](' + s.url + ')').join('\n'));
  }
  return out.join('\n\n');
}

export function toMarkdown(conv, { labels, include = {}, frontmatter: withFrontmatter = true, tags = '', exportedAt = new Date() }) {
  const parts = [];
  if (withFrontmatter) parts.push(frontmatter(conv, { tags, exportedAt }).trimEnd());
  parts.push('# ' + (conv.title || 'Chat').replace(/\n/g, ' '));
  if (!withFrontmatter) {
    const meta = [conv.platformName, conv.model, conv.createdAt ? isoDate(conv.createdAt) : '', conv.url ? '[' + labels.source + '](' + conv.url + ')' : '']
      .filter(Boolean)
      .join(' · ');
    if (meta) parts.push('*' + meta + '*');
  }
  for (const msg of conv.messages) parts.push(messageMarkdown(msg, conv, { labels, include }));
  return parts.join('\n\n').replace(/\n{4,}/g, '\n\n\n') + '\n';
}

/* ai chat exporter - by herrlamatv
 * JSON-Export: alles Wesentliche maschinenlesbar, ohne HTML (das ist
 * seitenabhängig und groß) und ohne eingebettete Bilder.
 */

import { mdToText } from './text.js';

export const SCHEMA = 'aichat-export/1';

const stripData = (md) => String(md || '').replace(/!\[([^\]]*)\]\(data:[^)]+\)/g, '![$1](embedded)');

export function toJson(conv, { exportedAt = new Date() } = {}) {
  const data = {
    schema: SCHEMA,
    exportedAt: exportedAt.toISOString(),
    platform: conv.platform,
    platformName: conv.platformName,
    title: conv.title || '',
    url: conv.url || '',
    id: conv.id || '',
    model: conv.model || '',
    createdAt: conv.createdAt || null,
    updatedAt: conv.updatedAt || null,
    source: conv.source || 'dom',
    messages: conv.messages.map((msg) => ({
      role: msg.role,
      name: msg.name || null,
      time: msg.time || null,
      markdown: stripData(msg.md),
      text: mdToText(stripData(msg.md)),
      thinking: msg.thinking && msg.thinking.md ? msg.thinking.md : null,
      tools: (msg.tools || []).map((t) => ({ title: t.title, markdown: t.md })),
      sources: msg.sources || [],
      attachments: (msg.attachments || []).map((a) => ({ name: a.name, url: a.url || null }))
    }))
  };
  return JSON.stringify(data, null, 2) + '\n';
}

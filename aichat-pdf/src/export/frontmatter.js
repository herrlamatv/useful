/* ai chat exporter - by herrlamatv
 * YAML-Kopf für Markdown-Dateien - Obsidian liest daraus Eigenschaften
 * (Titel, Quelle, Datum, Tags), Dataview kann danach filtern.
 */

/** Immer in doppelten Anführungszeichen - dann ist nichts mehr „besonders“. */
export function yamlString(value) {
  const s = String(value == null ? '' : value)
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\r?\n/g, '\\n')
    .replace(/\t/g, '\\t')
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/g, '');
  return '"' + s + '"';
}

/** "ai-chat, Claude Export" -> ['ai-chat', 'claude-export'] */
export function parseTags(text) {
  const out = [];
  for (const raw of String(text || '').split(/[,;\n]/)) {
    const tag = raw
      .trim()
      .replace(/^#/, '')
      .replace(/\s+/g, '-')
      .replace(/[^\p{L}\p{N}_\-/]/gu, '')
      .toLowerCase();
    if (tag && !out.includes(tag)) out.push(tag);
  }
  return out;
}

function isoOrNull(value) {
  if (!value) return null;
  const d = new Date(value);
  return isNaN(d) ? null : d.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

export function frontmatter(conv, { tags = '', exportedAt = new Date() } = {}) {
  const lines = ['---'];
  lines.push('title: ' + yamlString(conv.title || 'Chat'));
  lines.push('platform: ' + yamlString(conv.platformName || conv.platform || ''));
  if (conv.model) lines.push('model: ' + yamlString(conv.model));
  if (conv.url) lines.push('source: ' + yamlString(conv.url));
  const created = isoOrNull(conv.createdAt);
  const updated = isoOrNull(conv.updatedAt);
  if (created) lines.push('created: ' + created);
  if (updated) lines.push('updated: ' + updated);
  lines.push('exported: ' + isoOrNull(exportedAt));
  lines.push('messages: ' + (conv.messages ? conv.messages.length : 0));
  const list = parseTags(tags);
  const platformTag = parseTags(conv.platformName || conv.platform)[0];
  if (platformTag && !list.includes(platformTag)) list.push(platformTag);
  if (list.length) {
    lines.push('tags:');
    for (const tag of list) lines.push('  - ' + yamlString(tag));
  }
  lines.push('---');
  return lines.join('\n') + '\n';
}

/* ai chat exporter - by herrlamatv
 * Claude: Antwort von /api/organizations/{org}/chat_conversations/{id}
 * (?tree=True&rendering_mode=messages&render_all_tools=true) -> Nachrichten.
 *
 * Mit tree=True kommen alle Äste - sichtbar ist der Weg vom
 * current_leaf_message_uuid zurück über parent_message_uuid.
 * Artifacts stehen als Werkzeug-Aufruf „artifacts“ in der Antwort und werden
 * als Code-Block mit Titel eingesetzt.
 */

const ARTIFACT_LANG = {
  'application/vnd.ant.react': 'jsx',
  'text/html': 'html',
  'text/markdown': 'markdown',
  'image/svg+xml': 'svg',
  'application/vnd.ant.mermaid': 'mermaid'
};

function branch(data) {
  const msgs = data.chat_messages || [];
  const byId = new Map(msgs.map((m) => [m.uuid, m]));
  const leaf = data.current_leaf_message_uuid;
  if (leaf && byId.has(leaf) && msgs.some((m) => m.parent_message_uuid)) {
    const list = [];
    const seen = new Set();
    let id = leaf;
    while (id && byId.has(id) && !seen.has(id)) {
      seen.add(id);
      list.push(byId.get(id));
      id = byId.get(id).parent_message_uuid;
    }
    return list.reverse();
  }
  return msgs.slice().sort((a, b) => (a.index || 0) - (b.index || 0));
}

const fence = (lang, code) => {
  const ticks = /```/.test(code) ? '````' : '```';
  return ticks + (lang || '') + '\n' + String(code || '').replace(/\n$/, '') + '\n' + ticks;
};

function artifactMarkdown(input, kept) {
  const id = input.id || input.title || 'artifact';
  const title = input.title || kept.get(id)?.title || 'Artifact';
  const lang = input.language || ARTIFACT_LANG[input.type] || kept.get(id)?.lang || '';
  if (input.command === 'update' && typeof input.old_str === 'string') {
    const prev = kept.get(id);
    if (prev) prev.content = prev.content.replace(input.old_str, input.new_str || '');
    const diff = input.old_str.split('\n').map((l) => '- ' + l).concat((input.new_str || '').split('\n').map((l) => '+ ' + l));
    return '**' + title + '** *(update)*\n\n' + fence('diff', diff.join('\n'));
  }
  if (typeof input.content !== 'string') return '';
  kept.set(id, { title, lang, content: input.content });
  return '**' + title + '**\n\n' + fence(lang, input.content);
}

function absolute(url, origin) {
  if (!url) return '';
  if (/^https?:/i.test(url)) return url;
  return origin ? origin.replace(/\/$/, '') + (url.startsWith('/') ? '' : '/') + url : url;
}

export function parseClaude(data, { origin = 'https://claude.ai' } = {}) {
  const kept = new Map();
  const messages = [];

  for (const m of branch(data)) {
    const role = m.sender === 'human' ? 'user' : 'assistant';
    const msg = { id: m.uuid, role, time: m.created_at || null, md: '', thinking: null, tools: [], sources: [], attachments: [] };
    const md = [];
    const addSource = (url, title) => {
      if (url && /^https?:/i.test(url) && !msg.sources.some((s) => s.url === url)) msg.sources.push({ title: title || url, url });
    };

    const blocks = Array.isArray(m.content) && m.content.length ? m.content : [{ type: 'text', text: m.text || '' }];
    for (const b of blocks) {
      if (!b) continue;
      if (b.type === 'text') {
        if (b.text) md.push(b.text);
        for (const c of b.citations || []) addSource(c.url || (c.details && c.details.url), c.title || (c.details && c.details.title));
      } else if (b.type === 'thinking') {
        const text = b.thinking || (b.summaries || []).map((s) => s.summary).join('\n\n');
        if (text) msg.thinking = { md: msg.thinking ? msg.thinking.md + '\n\n' + text : text };
      } else if (b.type === 'tool_use') {
        const input = b.input || {};
        if (b.name === 'artifacts') {
          const art = artifactMarkdown(input, kept);
          if (art) md.push(art);
        } else if (b.name === 'create_file' && typeof input.file_text === 'string') {
          const name = String(input.path || 'file').split('/').pop();
          md.push('**' + name + '**\n\n' + fence((name.split('.').pop() || '').toLowerCase(), input.file_text));
        } else {
          const shown = Object.keys(input).length ? fence('json', JSON.stringify(input, null, 2)) : '';
          msg.tools.push({ title: 'Tool · ' + (b.name || 'tool'), md: shown || '*(' + (b.name || 'tool') + ')*' });
        }
      } else if (b.type === 'tool_result') {
        const parts = Array.isArray(b.content) ? b.content : [];
        const text = [];
        for (const p of parts) {
          if (p.type === 'knowledge') addSource(p.url, p.title);
          else if (p.type === 'text' && p.text) text.push(p.text);
        }
        if (text.length) msg.tools.push({ title: 'Output · ' + (b.name || 'tool'), md: text.join('\n\n') });
      }
    }

    /* Anhänge: Text-Anhänge (attachments) und Dateien/Bilder (files, files_v2). */
    for (const a of m.attachments || []) msg.attachments.push({ name: a.file_name || 'file', kind: a.file_type || '' });
    const files = [].concat(m.files_v2 || [], (m.files_v2 && m.files_v2.length ? [] : m.files) || []);
    for (const f of files) {
      const name = f.file_name || 'file';
      if (f.file_kind === 'image') {
        const src = absolute((f.preview_asset && f.preview_asset.url) || f.preview_url || (f.thumbnail_asset && f.thumbnail_asset.url) || f.thumbnail_url, origin);
        if (src) md.unshift('![' + name.replace(/[[\]]/g, '') + '](' + src + ')');
        else msg.attachments.push({ name, kind: 'image' });
      } else {
        msg.attachments.push({ name, kind: f.file_kind || '' });
      }
    }

    msg.md = md.join('\n\n').trim();
    if (msg.md || msg.thinking || msg.tools.length || msg.attachments.length) messages.push(msg);
  }

  return {
    title: data.name || '',
    id: data.uuid || '',
    model: data.model || '',
    createdAt: data.created_at || null,
    updatedAt: data.updated_at || null,
    messages
  };
}

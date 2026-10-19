/* ai chat exporter - by herrlamatv
 * ChatGPT: Antwort von /backend-api/conversation/{id} -> Nachrichten.
 *
 * Die Unterhaltung ist ein Baum (jede bearbeitete Frage, jede neu erzeugte
 * Antwort ist ein Ast). Sichtbar ist der Weg von `current_node` zurück zur
 * Wurzel - genau den nehmen wir. Mehrere Assistenten-/Werkzeug-Nachrichten
 * hintereinander (Suche, Code, Bild) werden zu einer Antwort zusammengefasst,
 * so wie die Seite sie auch zeigt.
 */

const toIso = (sec) => (typeof sec === 'number' && sec > 0 ? new Date(sec * 1000).toISOString() : null);

function pathToCurrent(data) {
  const mapping = data.mapping || {};
  const chain = [];
  let id = data.current_node;
  if (!id || !mapping[id]) {
    /* Kein current_node: von der Wurzel immer den letzten Ast nehmen. */
    const root = Object.values(mapping).find((n) => !n.parent || !mapping[n.parent]);
    let node = root;
    while (node) {
      chain.push(node);
      const kids = node.children || [];
      node = kids.length ? mapping[kids[kids.length - 1]] : null;
    }
    return chain;
  }
  const seen = new Set();
  while (id && mapping[id] && !seen.has(id)) {
    seen.add(id);
    chain.push(mapping[id]);
    id = mapping[id].parent;
  }
  return chain.reverse();
}

/** Zitat-Marker im Text durch Links ersetzen, Quellen einsammeln. */
export function resolveCitations(text, metadata = {}, sources = []) {
  let s = String(text || '');
  const add = (url, title) => {
    if (!url || !/^https?:/i.test(url)) return;
    if (!sources.some((x) => x.url === url)) sources.push({ title: title || url, url });
  };

  const refs = (metadata.content_references || []).filter((r) => r && typeof r.matched_text === 'string' && r.matched_text);
  /* Von hinten nach vorn ersetzen, damit die Positionen stimmen. */
  refs.sort((a, b) => (b.start_idx || 0) - (a.start_idx || 0));
  for (const ref of refs) {
    const items = [].concat(ref.items || [], ref.fallback_items || []);
    for (const item of items) {
      add(item.url, item.title);
      for (const extra of item.supporting_websites || []) add(extra.url, extra.title);
    }
    for (const url of ref.safe_urls || []) add(url);
    let replacement = '';
    if (ref.type !== 'sources_footnote' && ref.type !== 'hidden' && ref.type !== 'image_v2' && ref.type !== 'products') {
      if (typeof ref.alt === 'string' && ref.alt.trim()) replacement = ref.alt.trim();
      else if (items[0] && items[0].url) replacement = '[' + (items[0].attribution || items[0].title || 'link').replace(/[[\]]/g, '') + '](' + items[0].url + ')';
    }
    /* Mit Leerzeichen davor - außer es steht schon eins da. */
    const splice = (text, start, end) => {
      const before = text.slice(0, start);
      return before + (replacement && !/\s$/.test(before) && before ? ' ' : '') + replacement + text.slice(end);
    };
    const start = ref.start_idx;
    const end = ref.end_idx;
    if (Number.isInteger(start) && Number.isInteger(end) && s.slice(start, end) === ref.matched_text) {
      s = splice(s, start, end);
    } else {
      for (let at = s.indexOf(ref.matched_text); at !== -1; at = s.indexOf(ref.matched_text, at + replacement.length)) {
        s = splice(s, at, at + ref.matched_text.length);
      }
    }
  }

  for (const c of metadata.citations || []) {
    const meta = c.metadata || c;
    add(meta.url, meta.title);
  }

  return s
    .replace(/\ue200[^\ue201]*\ue201/g, '')
    .replace(/[\ue200-\ue2ff]/g, '')
    .replace(/【[^】]{0,80}】/g, '')
    .replace(/[ \t]+\n/g, '\n');
}

function imagePart(part, assets) {
  const ptr = part.asset_pointer || '';
  const url = (assets && assets[ptr]) || '';
  const alt = (part.metadata && part.metadata.dalle && part.metadata.dalle.prompt) || 'image';
  return url ? '![' + alt.replace(/[[\]\n]/g, ' ').slice(0, 120) + '](' + url + ')' : '*(' + alt.replace(/\n/g, ' ').slice(0, 120) + ')*';
}

/** Inhalt -> Markdown (für Text, Multimodal, Code). */
function contentMarkdown(content, assets) {
  if (!content) return '';
  const type = content.content_type;
  if (type === 'text' || type === 'multimodal_text') {
    return (content.parts || [])
      .map((p) => (typeof p === 'string' ? p : p && p.content_type === 'image_asset_pointer' ? imagePart(p, assets) : p && typeof p.text === 'string' ? p.text : ''))
      .filter((p) => p !== '')
      .join('\n\n');
  }
  if (type === 'code') return '```' + (content.language && content.language !== 'unknown' ? content.language : '') + '\n' + (content.text || '') + '\n```';
  if (type === 'execution_output') return '```\n' + (content.text || '') + '\n```';
  if (typeof content.text === 'string') return content.text;
  if (typeof content.result === 'string') return content.result;
  return '';
}

function thoughtsMarkdown(content) {
  return (content.thoughts || [])
    .map((t) => [t.summary ? '**' + t.summary.trim() + '**' : '', (t.content || '').trim()].filter(Boolean).join('\n\n'))
    .filter(Boolean)
    .join('\n\n');
}

/**
 * data:   JSON der Unterhaltung
 * assets: { 'file-service://file-…': 'https://…' } - vom Content-Script aufgelöst
 */
export function parseChatGPT(data, { assets = {} } = {}) {
  const turns = [];
  let model = data.default_model_slug || '';
  let current = null;

  const startTurn = (role, msg) => {
    current = {
      id: msg.id,
      ids: [msg.id],
      role,
      time: toIso(msg.create_time),
      md: '',
      thinking: null,
      tools: [],
      sources: [],
      attachments: []
    };
    turns.push(current);
    return current;
  };
  const appendMd = (turn, md) => {
    if (md && md.trim()) turn.md = turn.md ? turn.md + '\n\n' + md.trim() : md.trim();
  };

  for (const node of pathToCurrent(data)) {
    const msg = node.message;
    if (!msg || !msg.author || !msg.content) continue;
    const meta = msg.metadata || {};
    const role = msg.author.role;
    const type = msg.content.content_type;
    if (role === 'system' || meta.is_visually_hidden_from_conversation) continue;
    if (type === 'user_editable_context' || type === 'model_editable_context' || type === 'reasoning_recap') continue;

    if (role === 'user') {
      const turn = startTurn('user', msg);
      appendMd(turn, contentMarkdown(msg.content, assets));
      for (const a of meta.attachments || []) turn.attachments.push({ name: a.name || a.id || 'file', kind: a.mime_type || '' });
      continue;
    }

    /* assistant oder tool - gehört zur laufenden Antwort. */
    const turn = current && current.role === 'assistant' ? current : startTurn('assistant', msg);
    if (!turn.ids.includes(msg.id)) turn.ids.push(msg.id);
    if (meta.model_slug) model = meta.model_slug;

    if (type === 'thoughts') {
      const md = thoughtsMarkdown(msg.content);
      if (md) turn.thinking = { md: turn.thinking ? turn.thinking.md + '\n\n' + md : md };
      continue;
    }

    if (role === 'assistant' && (msg.recipient === 'all' || !msg.recipient)) {
      appendMd(turn, resolveCitations(contentMarkdown(msg.content, assets), meta, turn.sources));
      continue;
    }

    /* Werkzeug: erzeugte Bilder gehören sichtbar in die Antwort, der Rest als Werkzeug-Block. */
    const parts = (msg.content.parts || []).filter((p) => p && p.content_type === 'image_asset_pointer');
    if (role === 'tool' && parts.length) {
      appendMd(turn, parts.map((p) => imagePart(p, assets)).join('\n\n'));
      continue;
    }
    const md = contentMarkdown(msg.content, assets);
    if (md.trim()) {
      const name = role === 'tool' ? msg.author.name || 'tool' : msg.recipient || 'tool';
      turn.tools.push({ title: role === 'tool' ? 'Output · ' + name : 'Tool · ' + name, md: md.trim() });
    }
    for (const c of meta.citations || []) {
      const m = c.metadata || c;
      if (m.url && !turn.sources.some((s) => s.url === m.url)) turn.sources.push({ title: m.title || m.url, url: m.url });
    }
  }

  return {
    title: data.title || '',
    id: data.conversation_id || data.id || '',
    model,
    createdAt: toIso(data.create_time),
    updatedAt: toIso(data.update_time),
    messages: turns.filter((t) => t.md || t.thinking || t.tools.length || t.attachments.length)
  };
}

/** Alle Bild-Zeiger einer Unterhaltung - die löst das Content-Script in URLs auf. */
export function assetPointers(data) {
  const out = new Set();
  for (const node of Object.values(data.mapping || {})) {
    const parts = node.message && node.message.content && node.message.content.parts;
    for (const p of parts || []) if (p && p.content_type === 'image_asset_pointer' && p.asset_pointer) out.add(p.asset_pointer);
  }
  return [...out];
}

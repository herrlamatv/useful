/* ai chat exporter - by herrlamatv
 * ChatGPT (chatgpt.com, chat.openai.com).
 *
 * Inhalt kommt aus der internen API (/backend-api/conversation/{id}) - genau
 * das Markdown, das ChatGPT selbst hat, inkl. Gedankengang und Quellen. Das
 * DOM liefert nur das fertig gerenderte HTML für PDF/HTML und ist der
 * Notnagel, falls die API einmal nicht antwortet.
 *
 * DOM (Stand 10/2026): jede Nachricht ist ein Element mit
 * data-chatgpt-search-unit-key="…:user|assistant" und den Nachrichten-IDs in
 * data-chatgpt-search-message-ids. Älteres DOM: data-message-author-role.
 */

(() => {
  const AIX = globalThis.AIX;

  const SEL = {
    unit: '[data-chatgpt-search-unit-key]',
    legacy: '[data-message-author-role]',
    userBubble: '[data-user-message-bubble], .whitespace-pre-wrap',
    assistantBody: '[data-chatgpt-selection-message-id], .markdown',
    codeBlock: '[data-code-block="true"]',
    strip: [
      '[data-markdown-copy="exclude"]',
      '[data-code-block-controls]',
      '.turn-action-controls',
      '[data-testid$="turn-action-button"]',
      '[data-testid="copy-turn-action-button"]'
    ].join(',')
  };

  const chatId = () => (location.pathname.match(/\/c\/([0-9a-f-]{20,})/i) || [])[1] || '';

  let token = null;
  async function headers() {
    if (!token || token.until < Date.now()) {
      const session = await AIX.fetchJson('/api/auth/session');
      if (!session || !session.accessToken) throw new Error('not-logged-in');
      token = { value: session.accessToken, until: Date.now() + 4 * 60 * 1000 };
    }
    const h = { Authorization: 'Bearer ' + token.value };
    const account = AIX.cookie('_account');
    if (/^[0-9a-f-]{36}$/i.test(account)) h['ChatGPT-Account-Id'] = account;
    return h;
  }

  const conversation = async (id) => AIX.fetchJson('/backend-api/conversation/' + id, { headers: await headers() });

  /** Bild-Zeiger (file-service://, sediment://) -> Download-Adresse. */
  async function resolveAssets(data, id) {
    const pointers = new Set();
    for (const node of Object.values(data.mapping || {})) {
      const parts = node.message && node.message.content && node.message.content.parts;
      for (const p of parts || []) if (p && p.content_type === 'image_asset_pointer' && p.asset_pointer) pointers.add(p.asset_pointer);
    }
    const map = {};
    const h = await headers();
    for (const ptr of Array.from(pointers).slice(0, 60)) {
      const file = ptr.replace(/^[a-z-]+:\/\//i, '');
      for (const url of ['/backend-api/files/download/' + file + '?conversation_id=' + id + '&inline=false', '/backend-api/files/' + file + '/download']) {
        try {
          const j = await AIX.fetchJson(url, { headers: h });
          if (j && j.download_url) {
            map[ptr] = new URL(j.download_url, location.origin).href;
            break;
          }
        } catch (e) {}
      }
    }
    return map;
  }

  const toIso = (v) => (typeof v === 'number' ? new Date(v * 1000).toISOString() : v || null);

  AIX.register({
    id: 'chatgpt',
    name: 'ChatGPT',
    match: () => /(^|\.)chatgpt\.com$|^chat\.openai\.com$/.test(location.hostname),
    virtual: true,
    chatId,
    title: () => AIX.pageTitle([/\s*[|-]\s*ChatGPT$/i]),

    turns() {
      const units = AIX.outermost(AIX.all(SEL.unit)).filter((u) => /:(user|assistant)$/.test(u.getAttribute('data-chatgpt-search-unit-key')));
      if (units.length) {
        return units.map((node) => {
          const ids = [...new Set((node.getAttribute('data-chatgpt-search-message-ids') || '').split(/\s+/).filter(Boolean))];
          return { node, role: /:user$/.test(node.getAttribute('data-chatgpt-search-unit-key')) ? 'user' : 'assistant', id: ids[0] || null, ids };
        });
      }
      const legacy = AIX.all(SEL.legacy).filter((n) => /^(user|assistant)$/.test(n.getAttribute('data-message-author-role')));
      if (legacy.length) {
        return legacy.map((node) => ({ node, role: node.getAttribute('data-message-author-role'), id: node.getAttribute('data-message-id') }));
      }
      return AIX.guessTurns();
    },

    content(node, role) {
      if (role === 'user') return AIX.first(node, SEL.userBubble);
      const bodies = AIX.all(SEL.assistantBody, node);
      return bodies.length === 1 ? bodies[0] : node;
    },

    strip: SEL.strip,

    /* Code-Blöcke sind kein <pre> mehr: Kopfzeile (Sprache, Knöpfe) + <code>. */
    prepare(clone) {
      for (const block of AIX.all(SEL.codeBlock, clone)) {
        const label = AIX.first(block, ['[data-markdown-copy="exclude"] .truncate', '[data-markdown-copy="exclude"] [class*="truncate"]']);
        const lang = label ? label.textContent.trim().toLowerCase().replace(/\s+/g, '-') : '';
        const code = AIX.first(block, ['[data-code-block-source] code', 'code']);
        const pre = document.createElement('pre');
        const inner = document.createElement('code');
        if (code) for (const child of Array.from(code.childNodes)) inner.append(child);
        if (lang) pre.setAttribute('data-aix-lang', lang);
        pre.append(inner);
        block.replaceWith(pre);
      }
    },

    async api() {
      const id = chatId();
      if (!id) return null;
      const data = await conversation(id);
      return { kind: 'chatgpt', data, assets: await resolveAssets(data, id) };
    },

    bulk: {
      async list(offset = 0) {
        const j = await AIX.fetchJson('/backend-api/conversations?offset=' + offset + '&limit=100&order=updated', { headers: await headers() });
        const items = (j.items || []).map((i) => ({ id: i.id, title: i.title || '', created: toIso(i.create_time), updated: toIso(i.update_time) }));
        const next = offset + items.length;
        return { items, total: j.total || null, next: items.length && (!j.total || next < j.total) ? next : null };
      },
      async get(id) {
        return { kind: 'chatgpt', data: await conversation(id), assets: {} };
      }
    }
  });
})();

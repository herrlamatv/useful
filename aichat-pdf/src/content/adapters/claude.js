/* ai chat exporter - by herrlamatv
 * Claude (claude.ai).
 *
 * Inhalt aus der internen API (…/chat_conversations/{id}?tree=True&…) -
 * Markdown, Gedankengang, Artifacts, Quellen. Die Organisation steht im
 * Cookie lastActiveOrg, sonst fragen wir /api/organizations.
 *
 * DOM (Stand 10/2026): Nachrichten-Zeilen mit data-cds="UserMessage" bzw.
 * data-testid="assistant-message", jeweils mit data-turn-key = Nachrichten-
 * UUID der API. Claude rendert nur den sichtbaren Teil (virtuelle Liste).
 */

(() => {
  const AIX = globalThis.AIX;

  const SEL = {
    rows: '[data-cds="UserMessage"], [data-cds="AssistantMessage"], [data-testid="assistant-message"]',
    legacy: '[data-testid="user-message"], .font-claude-response, .font-claude-message',
    userBody: '[data-testid="user-message"]',
    scroller: '[data-autoscroll-container="true"]',
    strip: [
      '[data-find-omitted]',
      '[data-testid="message-actions"]',
      '[data-testid="code-block-copy"]',
      '[data-testid^="action-bar"]',
      '[data-testid="file-thumbnail"]'
    ].join(',')
  };

  const chatId = () => (location.pathname.match(/\/chat\/([0-9a-f-]{36})/i) || [])[1] || '';

  let orgId = '';
  async function org() {
    if (orgId) return orgId;
    const fromCookie = AIX.cookie('lastActiveOrg');
    if (/^[0-9a-f-]{36}$/i.test(fromCookie)) return (orgId = fromCookie);
    const orgs = await AIX.fetchJson('/api/organizations');
    const chosen = (orgs || []).find((o) => (o.capabilities || []).includes('chat')) || (orgs || [])[0];
    if (!chosen) throw new Error('not-logged-in');
    return (orgId = chosen.uuid);
  }

  const conversation = async (id) =>
    AIX.fetchJson('/api/organizations/' + (await org()) + '/chat_conversations/' + id + '?tree=True&rendering_mode=messages&render_all_tools=true');

  const roleOf = (node) =>
    node.getAttribute('data-cds') === 'UserMessage' || node.getAttribute('data-testid') === 'user-message' ? 'user' : 'assistant';

  AIX.register({
    id: 'claude',
    name: 'Claude',
    match: () => location.hostname === 'claude.ai',
    virtual: true,
    chatId,
    title: () => AIX.pageTitle([/\s*-\s*Claude$/i]),
    scroller: () => document.querySelector(SEL.scroller),

    turns() {
      let rows = AIX.outermost(AIX.all(SEL.rows));
      if (!rows.length) rows = AIX.outermost(AIX.all(SEL.legacy));
      if (!rows.length) return AIX.guessTurns();
      return rows.map((node) => {
        const keyed = node.closest('[data-turn-key]') || node.querySelector('[data-turn-key]');
        const id = keyed ? keyed.getAttribute('data-turn-key') : null;
        return { node, role: roleOf(node), id };
      });
    },

    content(node, role) {
      return role === 'user' ? AIX.first(node, SEL.userBody) || node : node;
    },

    strip: SEL.strip,

    attachments(node, role) {
      if (role !== 'user') return [];
      return AIX.all('[data-testid="file-thumbnail"]', node)
        .map((el) => ({ name: (el.getAttribute('title') || el.getAttribute('aria-label') || (el.querySelector('img') || {}).alt || AIX.text(el)).trim() }))
        .filter((a) => a.name);
    },

    async api() {
      const id = chatId();
      if (!id) return null;
      return { kind: 'claude', data: await conversation(id), origin: location.origin };
    },

    bulk: {
      async list(offset = 0) {
        const list = await AIX.fetchJson('/api/organizations/' + (await org()) + '/chat_conversations?limit=100&offset=' + offset);
        const items = (list || []).map((c) => ({ id: c.uuid, title: c.name || '', created: c.created_at || null, updated: c.updated_at || null }));
        return { items, total: null, next: items.length === 100 ? offset + 100 : null };
      },
      async get(id) {
        return { kind: 'claude', data: await conversation(id), origin: location.origin };
      }
    }
  });
})();

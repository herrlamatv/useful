/* ai chat exporter - by herrlamatv
 * Gemini (gemini.google.com) - live geprüft 10/2026:
 * <user-query> / <model-response>, Inhalt in .query-text bzw. <message-content>,
 * Mathe als .math-inline / .math-block mit data-math (TeX), Code in <code-block>
 * mit der Sprache als Text in der Kopfzeile, Scroll-Container <infinite-scroller>.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'gemini',
      name: 'Gemini',
      match: () => location.hostname === 'gemini.google.com',
      chatId: () => (location.pathname.match(/\/app\/([0-9a-f]+)/i) || [])[1] || '',
      user: 'user-query',
      assistant: 'model-response',
      userBody: ['.query-text', '.query-content'],
      assistantBody: ['message-content', '.markdown'],
      thinking: ['model-thoughts .thoughts-content', 'model-thoughts [class*="thoughts-content"]'],
      sources: ['sources-list', '.sources-list'],
      scroller: ['infinite-scroller', '#chat-history'],
      titleSuffix: /\s*-\s*Google Gemini$/i,
      strip: [
        'mat-icon', 'gem-icon', 'gem-icon-button', 'gem-popover', '.code-block-decoration', '.table-footer',
        'message-actions', '.luminous-actions-container', '.file-preview-container', 'sources-carousel-inline',
        '.response-footer', 'thinking-overlay', 'model-thoughts'
      ].join(','),
      codeLang(pre) {
        const block = pre.closest('code-block');
        if (!block) return '';
        const label = AIX.all('span', block).find((s) => !s.closest('pre') && !s.children.length && /^[\w+#.\- ]{1,24}$/.test(s.textContent.trim()));
        return label ? label.textContent.trim().toLowerCase().replace(/\s+/g, '-') : '';
      }
    })
  );
})();

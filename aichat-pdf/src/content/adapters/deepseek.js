/* ai chat exporter - by herrlamatv
 * DeepSeek (chat.deepseek.com). Nicht live geprüft (kein Login).
 * Beiträge .ds-message - Antworten enthalten .ds-markdown, der Gedankengang
 * („DeepThink“) steckt in einem eigenen Block davor.
 */

(() => {
  const AIX = globalThis.AIX;
  const THINK = ['.ds-think-content', '[class*="think-content"]', '[class*="thinking"]'];

  AIX.register(
    AIX.domAdapter({
      id: 'deepseek',
      name: 'DeepSeek',
      match: () => location.hostname === 'chat.deepseek.com',
      chatId: () => (location.pathname.match(/\/s\/([\w-]+)/) || [])[1] || '',
      turns: () =>
        AIX.outermost(AIX.all('.ds-message')).map((node) => ({
          node,
          role: AIX.all('.ds-markdown', node).some((md) => !THINK.some((t) => md.closest(t))) ? 'assistant' : 'user'
        })),
      thinking: THINK,
      titleSuffix: /\s*[-|]\s*DeepSeek.*$/i,
      strip: '[class*="copy"], .sr-only'
    })
  );
})();

/* ai chat exporter - by herrlamatv
 * Le Chat von Mistral (chat.mistral.ai). Nicht live geprüft (kein Login).
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'lechat',
      name: 'Le Chat',
      match: () => location.hostname === 'chat.mistral.ai',
      chatId: () => (location.pathname.match(/\/chat\/([\w-]+)/) || [])[1] || '',
      user: ['[data-message-author-role="user"]', '[data-role="user"]', '[class*="user-message"]'],
      assistant: ['[data-message-author-role="assistant"]', '[data-role="assistant"]', '[data-message-part-type="answer"]', '[class*="assistant-message"]'],
      thinking: ['[data-message-part-type="reasoning"]', '[class*="reasoning"]'],
      titleSuffix: /\s*[|-]\s*(Le Chat|Mistral|Vibe Chat).*$/i,
      strip: '[class*="actions"], .sr-only'
    })
  );
})();

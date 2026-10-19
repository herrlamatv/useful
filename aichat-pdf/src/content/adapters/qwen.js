/* ai chat exporter - by herrlamatv
 * Qwen Chat (chat.qwen.ai, Open-WebUI-Abkömmling). Nicht live geprüft (kein Login).
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'qwen',
      name: 'Qwen',
      match: () => location.hostname === 'chat.qwen.ai',
      chatId: () => (location.pathname.match(/\/c\/([\w-]+)/) || [])[1] || '',
      user: ['.chat-user', '.user-message', '[class*="user-message"]'],
      assistant: ['.chat-assistant', '.response-message-content', '[class*="assistant-message"]'],
      thinking: ['.thinking-chain-container', 'details[type="reasoning"]', '[class*="thinking"]'],
      titleSuffix: /\s*[|-]\s*Qwen.*$/i,
      strip: '[class*="message-footer"], [class*="buttons"], .sr-only'
    })
  );
})();

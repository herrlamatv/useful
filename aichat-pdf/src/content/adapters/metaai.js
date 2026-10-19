/* ai chat exporter - by herrlamatv
 * Meta AI (meta.ai). Nicht live geprüft (kein Login) - die Klassennamen sind
 * verschleiert, darum nur wenige Hinweise und sonst raten.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'metaai',
      name: 'Meta AI',
      match: () => /(^|\.)meta\.ai$/.test(location.hostname),
      chatId: () => (location.pathname.match(/\/prompt\/([\w-]+)/) || [])[1] || '',
      user: ['[data-testid="user-message"]', '[data-testid*="user-message"]'],
      assistant: ['[data-testid="assistant-message"]', '[data-testid*="bot-response"]'],
      titleSuffix: /\s*[|-]\s*Meta AI.*$/i
    })
  );
})();

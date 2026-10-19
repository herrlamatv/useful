/* ai chat exporter - by herrlamatv
 * Grok (grok.com und x.com/i/grok).
 * Nicht live geprüft (Nutzungsbedingungen-Seite) - Selektoren nach bekanntem
 * Aufbau von grok.com: Beiträge [id^="response-"], eigene rechts (.items-end),
 * Antworten links (.items-start), Inhalt in .message-bubble. Auf x.com und bei
 * geändertem DOM wird geraten.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'grok',
      name: 'Grok',
      match: () => location.hostname === 'grok.com' || (location.hostname === 'x.com' && /^\/i\/grok/.test(location.pathname)),
      chatId: () => (location.pathname.match(/\/c\/([\w-]+)/) || location.search.match(/conversation=(\d+)/) || [])[1] || '',
      user: ['[id^="response-"].items-end', 'div.items-end > .message-bubble'],
      assistant: ['[id^="response-"].items-start', 'div.items-start > .message-bubble'],
      userBody: ['.message-bubble'],
      assistantBody: ['.response-content-markdown', '.message-bubble'],
      thinking: ['.thinking-container', '[class*="thinking-container"]'],
      titleSuffix: /\s*[-|]\s*Grok$/i,
      strip: '.action-buttons, [class*="action-buttons"], .sr-only'
    })
  );
})();

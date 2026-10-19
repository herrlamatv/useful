/* ai chat exporter - by herrlamatv
 * Poe (poe.com). Nicht live geprüft (kein Login). CSS-Module mit festen
 * Präfixen: Message_rightSideMessageBubble (du) / Message_leftSideMessageBubble.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'poe',
      name: 'Poe',
      match: () => location.hostname === 'poe.com',
      chatId: () => (location.pathname.match(/\/chat\/([\w-]+)/) || [])[1] || '',
      user: ['[class*="Message_rightSideMessageBubble"]', '[class*="Message_humanMessageBubble"]'],
      assistant: ['[class*="Message_leftSideMessageBubble"]', '[class*="Message_botMessageBubble"]'],
      userBody: ['[class*="Markdown_markdownContainer"]'],
      assistantBody: ['[class*="Markdown_markdownContainer"]'],
      titleSuffix: /\s*[|-]\s*Poe$/i,
      strip: '[class*="MessageActions"], [class*="ChatMessageActionBar"], [class*="Message_messageMetadata"]'
    })
  );
})();

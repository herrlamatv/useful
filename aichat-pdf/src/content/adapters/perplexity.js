/* ai chat exporter - by herrlamatv
 * Perplexity (perplexity.ai). Nicht live geprüft (kein Login).
 * Frage: [class*="group/query"], Antwort: [id^="markdown-content-"].
 * Quellen: Links in Zitat-Chips -> eigene Liste unter der Antwort.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'perplexity',
      name: 'Perplexity',
      match: () => /(^|\.)perplexity\.ai$/.test(location.hostname),
      chatId: () => (location.pathname.match(/\/search\/([\w.-]+)/) || [])[1] || '',
      user: ['[class*="group/query"]', '[data-testid="user-query"]', 'h1[class*="query"]'],
      assistant: ['[id^="markdown-content-"]', '[data-testid="answer"]'],
      sources: ['a[class*="citation"]', '[class*="citation"] a[href]'],
      titleSuffix: /\s*[|-]\s*Perplexity$/i,
      strip: '[class*="copy"], [class*="toolbar"], .sr-only'
    })
  );
})();

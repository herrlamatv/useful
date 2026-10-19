/* ai chat exporter - by herrlamatv
 * Universal-Modus für jede andere Seite: Beiträge werden geraten
 * (AIX.guessTurns) - oder du zeigst per Klick auf den Chat-Bereich (picker.js).
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register({
    id: 'universal',
    name: 'Universal',
    match: () => true,
    turns: () => AIX.guessTurns(),
    content: (node) => node,
    title: () => AIX.pageTitle()
  });
})();

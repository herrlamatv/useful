/* ai chat exporter - by herrlamatv
 * Google AI Studio (aistudio.google.com).
 * Nicht live geprüft (Nutzungsbedingungen-Dialog) - Selektoren nach bekanntem
 * Aufbau: <ms-chat-turn> mit .chat-turn-container.user / .model, Gedanken in
 * <ms-thought-chunk>. Passt nichts, wird geraten.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'aistudio',
      name: 'AI Studio',
      match: () => location.hostname === 'aistudio.google.com',
      virtual: true,
      chatId: () => (location.pathname.match(/\/prompts\/([\w-]+)/) || [])[1] || '',
      user: ['ms-chat-turn:has(.chat-turn-container.user)', '[data-turn-role="User"]'],
      assistant: ['ms-chat-turn:has(.chat-turn-container.model)', '[data-turn-role="Model"]'],
      userBody: ['.turn-content', 'ms-prompt-chunk'],
      assistantBody: ['.turn-content', 'ms-prompt-chunk'],
      thinking: ['ms-thought-chunk', '.thought-panel'],
      scroller: ['ms-autoscroll-container', '.chat-view-container'],
      titleSuffix: /\s*\|\s*Google AI Studio$/i,
      strip: '.actions-container, .author-label, ms-chat-turn-options, .turn-footer, mat-icon, .token-count, .turn-information'
    })
  );
})();

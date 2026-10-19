/* ai chat exporter - by herrlamatv
 * Microsoft Copilot (copilot.microsoft.com, das inzwischen auf copilot.com
 * umleitet) und Microsoft 365 Copilot (m365.cloud.microsoft).
 * Nicht live geprüft (kein Login) - Selektoren nach bekanntem Aufbau:
 * data-content="user-message" / "ai-message", bei M365 die Fluent-AI-Bausteine.
 */

(() => {
  const AIX = globalThis.AIX;

  AIX.register(
    AIX.domAdapter({
      id: 'copilot',
      name: 'Copilot',
      match: () => /(^|\.)copilot\.(microsoft\.)?com$|^m365\.cloud\.microsoft$/.test(location.hostname),
      chatId: () => (location.pathname.match(/\/chats\/([\w-]+)/) || [])[1] || '',
      user: ['[data-content="user-message"]', '[data-testid="user-message"]', '[class*="fai-UserMessage"]', '[class*="group/user-message"]'],
      assistant: ['[data-content="ai-message"]', '[data-testid="ai-message"]', '[class*="fai-CopilotMessage"]', '[class*="group/ai-message"]'],
      titleSuffix: /\s*[|-]\s*(Microsoft\s*)?(365\s*)?Copilot.*$/i,
      strip: '[data-testid*="action"], [class*="message-actions"], [class*="MessageActions"], .sr-only'
    })
  );
})();

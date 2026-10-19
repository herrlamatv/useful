/* ai chat exporter - by herrlamatv
 * Welche Seite ist welcher Chat? Wird vom Service Worker, vom Popup und von
 * der Bulk-Seite benutzt. Die Content-Script-Einträge im Manifest müssen dazu
 * passen - test/test.mjs prüft das.
 */

const CORE = ['src/content/core.js', 'src/content/html2md.js'];

export const PLATFORMS = Object.freeze([
  {
    id: 'chatgpt',
    name: 'ChatGPT',
    matches: ['https://chatgpt.com/*', 'https://chat.openai.com/*'],
    bulk: true
  },
  { id: 'claude', name: 'Claude', matches: ['https://claude.ai/*'], bulk: true },
  /* Grok gibt es auch unter x.com/i/grok - x.com ist eine Single-Page-App,
     darum läuft das Script auf ganz x.com und prüft den Pfad selbst. */
  { id: 'grok', name: 'Grok', matches: ['https://grok.com/*', 'https://x.com/*'], path: { 'x.com': /^\/i\/grok/ } },
  {
    id: 'copilot',
    name: 'Copilot',
    matches: ['https://copilot.microsoft.com/*', 'https://copilot.com/*', 'https://www.copilot.com/*', 'https://m365.cloud.microsoft/*']
  },
  { id: 'gemini', name: 'Gemini', matches: ['https://gemini.google.com/*'] },
  { id: 'aistudio', name: 'AI Studio', matches: ['https://aistudio.google.com/*'] },
  { id: 'perplexity', name: 'Perplexity', matches: ['https://www.perplexity.ai/*', 'https://perplexity.ai/*'] },
  { id: 'deepseek', name: 'DeepSeek', matches: ['https://chat.deepseek.com/*'] },
  { id: 'lechat', name: 'Le Chat', matches: ['https://chat.mistral.ai/*'] },
  { id: 'qwen', name: 'Qwen', matches: ['https://chat.qwen.ai/*'] },
  { id: 'metaai', name: 'Meta AI', matches: ['https://www.meta.ai/*'] },
  { id: 'poe', name: 'Poe', matches: ['https://poe.com/*'] }
]);

export const UNIVERSAL = Object.freeze({ id: 'universal', name: 'Universal', matches: [], bulk: false });

/** Alle Match-Muster - für Kontextmenü und Manifest-Abgleich. */
export const ALL_MATCHES = PLATFORMS.flatMap((p) => p.matches);

/** Dateien, die für eine Plattform injiziert werden (Reihenfolge zählt). */
export function scriptFiles(id) {
  const files = CORE.concat('src/content/adapters/' + id + '.js');
  return id === 'universal' ? files.concat('src/content/picker.js') : files.concat('src/content/buttons.js');
}

function hostMatches(pattern, host) {
  const m = /^https:\/\/([^/]+)\//.exec(pattern);
  return !!m && m[1] === host;
}

/**
 * URL -> Plattform. Unbekannte, aber normale Webseiten bekommen den
 * Universal-Modus, interne Seiten (chrome://, Web Store, …) gar nichts.
 */
export function platformFor(url) {
  let u;
  try {
    u = new URL(url);
  } catch (e) {
    return null;
  }
  if (u.protocol !== 'https:' && u.protocol !== 'http:') return null;
  if (u.host === 'chromewebstore.google.com' || u.host === 'chrome.google.com') return null;
  for (const p of PLATFORMS) {
    if (u.protocol !== 'https:') break;
    if (!p.matches.some((m) => hostMatches(m, u.host))) continue;
    const rule = p.path && p.path[u.host];
    if (rule && !rule.test(u.pathname)) return UNIVERSAL;
    return p;
  }
  return UNIVERSAL;
}

export function platformById(id) {
  return PLATFORMS.find((p) => p.id === id) || (id === 'universal' ? UNIVERSAL : null);
}

/* ai chat exporter - by herrlamatv
 * Einstellungen: Vorgaben, Prüfen beim Laden, Speichern.
 */

export const STORAGE_KEY = 'aichat';

export const FORMATS = Object.freeze(['md', 'pdf-styled', 'pdf-raw', 'pdf-shot', 'html', 'txt', 'json']);
export const LANGUAGES = Object.freeze(['auto', 'de', 'en']);

export const DEFAULT_INCLUDE = Object.freeze({
  thinking: true,    // Gedankengang / Reasoning, soweit die Seite ihn liefert
  sources: true,     // Quellen-Links (Websuche, Zitate)
  images: true,      // Bilder - im PDF/HTML eingebettet, im Markdown als Link
  attachments: true, // Dateinamen hochgeladener Anhänge
  tools: false,      // Werkzeug-Aufrufe (Code-Interpreter, Suche, …) als eigene Blöcke
  times: false       // Uhrzeit an jeder Nachricht
});

export const DEFAULT_SETTINGS = Object.freeze({
  version: 1,
  language: 'auto',
  format: 'md',                   // Standard für Schnell-Export (Alt+Shift+M)
  pdfMethod: 'direct',            // 'direct' = sofort als Datei, 'print' = Chrome-Druckdialog
  paper: 'a4',                    // 'a4' | 'letter'
  fontSize: 'm',                  // 's' | 'm' | 'l'
  include: { ...DEFAULT_INCLUDE },
  folder: 'AI-Chats/{platform}',  // Unterordner in Downloads
  file: '{date} {title}',         // Dateiname ohne Endung
  saveAs: false,                  // jedes Mal „Speichern unter…“ fragen
  frontmatter: true,              // YAML-Kopf im Markdown (Obsidian)
  tags: 'ai-chat',                // Tags im Frontmatter, mit Komma getrennt
  buttons: true                   // kleine Export-Knöpfe an jeder Antwort
});

const pick = (value, allowed, fallback) => (allowed.includes(value) ? value : fallback);
const str = (value, fallback, max = 200) => (typeof value === 'string' ? value.slice(0, max) : fallback);
const bool = (value, fallback) => (typeof value === 'boolean' ? value : fallback);

export function mergeSettings(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const include = {};
  for (const key of Object.keys(DEFAULT_INCLUDE)) {
    include[key] = bool(src.include && src.include[key], DEFAULT_INCLUDE[key]);
  }
  return {
    version: 1,
    language: pick(src.language, LANGUAGES, 'auto'),
    format: pick(src.format, FORMATS, DEFAULT_SETTINGS.format),
    pdfMethod: pick(src.pdfMethod, ['direct', 'print'], DEFAULT_SETTINGS.pdfMethod),
    paper: pick(src.paper, ['a4', 'letter'], DEFAULT_SETTINGS.paper),
    fontSize: pick(src.fontSize, ['s', 'm', 'l'], DEFAULT_SETTINGS.fontSize),
    include,
    folder: str(src.folder, DEFAULT_SETTINGS.folder),
    file: str(src.file, DEFAULT_SETTINGS.file) || DEFAULT_SETTINGS.file,
    saveAs: bool(src.saveAs, DEFAULT_SETTINGS.saveAs),
    frontmatter: bool(src.frontmatter, DEFAULT_SETTINGS.frontmatter),
    tags: str(src.tags, DEFAULT_SETTINGS.tags),
    buttons: bool(src.buttons, DEFAULT_SETTINGS.buttons)
  };
}

export async function loadSettings() {
  const data = await chrome.storage.local.get(STORAGE_KEY);
  return mergeSettings(data[STORAGE_KEY]);
}

export async function saveSettings(settings) {
  const clean = mergeSettings(settings);
  await chrome.storage.local.set({ [STORAGE_KEY]: clean });
  return clean;
}

/** Ein Feld ändern - verschachtelt mit Punkt: 'include.thinking'. */
export function setPath(settings, key, value) {
  const next = JSON.parse(JSON.stringify(settings));
  const parts = String(key).split('.');
  let obj = next;
  while (parts.length > 1) {
    const part = parts.shift();
    if (!obj[part] || typeof obj[part] !== 'object') obj[part] = {};
    obj = obj[part];
  }
  obj[parts[0]] = value;
  return mergeSettings(next);
}

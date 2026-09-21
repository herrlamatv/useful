/* Chroofer - by herrlamatv
 * Einstellungen laden/speichern und pro Host die passende Regel auflösen.
 */

import { DEFAULT_PROFILE, OS_FAMILIES, BROWSERS } from './profiles.js';

export const STORAGE_KEY = 'chroofer';

export const DEFAULT_OPTIONS = Object.freeze({
  headers: true,             // User-Agent / Sec-CH-UA per Netzwerk-Regel ersetzen
  highEntropyHeaders: false, // High-Entropy Client Hints aktiv mitsenden statt entfernen
  navigator: true,           // navigator.* im Seitenkontext überschreiben
  hideWebdriver: true,       // navigator.webdriver = false
  languages: false,
  languagesValue: 'de-DE,de,en-US,en',
  timezone: false,
  timezoneValue: 'Europe/Berlin',
  screen: false,
  screenValue: { width: 1920, height: 1080, colorDepth: 24, dpr: 1 },
  hardware: false,
  hardwareValue: { cores: 8, memory: 8 },
  hideEngineTraces: false,   // window.chrome verstecken, wenn Nicht-Chromium gespooft wird
  badge: true,
  language: 'auto'           // 'auto' folgt der Browsersprache, sonst 'de' | 'en'
});

export const LANGUAGES = ['auto', 'de', 'en'];

/* Geltungsbereich einer Website-Regel. */
export const DEFAULT_SCOPE = Object.freeze({
  subdomains: true,   // auch sub.meinedomain.de
  redirects: false,   // Profil bleibt aktiv, wenn die Seite auf eine andere Domain weiterleitet
  thirdParty: true    // auch Anfragen, die die Seite an fremde Domains schickt
});

/* Global startet auf "nicht spoofen": frisch installiert verändert Chroofer
   erst einmal nichts. Die Website-Regeln unten greifen trotzdem sofort. */
export const DEFAULT_SETTINGS = Object.freeze({
  version: 2,
  enabled: true,
  global: { mode: 'off', profile: { ...DEFAULT_PROFILE } },
  sites: [],
  options: { ...DEFAULT_OPTIONS }
});

/* Realistische Kombis für den Zufallsmodus. */
export const RANDOM_POOL = Object.freeze([
  { os: 'windows', osVersion: '11', browser: 'chrome',  browserVersion: '142' },
  { os: 'windows', osVersion: '11', browser: 'edge',    browserVersion: '142' },
  { os: 'windows', osVersion: '10', browser: 'chrome',  browserVersion: '140' },
  { os: 'windows', osVersion: '11', browser: 'firefox', browserVersion: '145' },
  { os: 'macos',   osVersion: '15', browser: 'chrome',  browserVersion: '142' },
  { os: 'macos',   osVersion: '15', browser: 'safari',  browserVersion: '18.5' },
  { os: 'macos',   osVersion: '14', browser: 'firefox', browserVersion: '144' },
  { os: 'linux',   osVersion: 'x86_64', browser: 'chrome',  browserVersion: '141' },
  { os: 'linux',   osVersion: 'ubuntu', browser: 'firefox', browserVersion: '145' },
  { os: 'android', osVersion: '15', browser: 'chrome',  browserVersion: '142' },
  { os: 'ios',     osVersion: '18', browser: 'safari',  browserVersion: '18.5' }
]);

export function mergeSettings(raw) {
  const s = raw && typeof raw === 'object' ? raw : {};
  const g = s.global && typeof s.global === 'object' ? s.global : {};
  const o = s.options && typeof s.options === 'object' ? s.options : {};
  return {
    version: 2,
    enabled: s.enabled !== false,
    global: {
      mode: ['spoof', 'off', 'random'].includes(g.mode) ? g.mode : DEFAULT_SETTINGS.global.mode,
      profile: Object.assign({}, DEFAULT_PROFILE, g.profile || {})
    },
    sites: Array.isArray(s.sites) ? dedupe(s.sites.map(normalizeSite).filter(Boolean)) : [],
    options: Object.assign({}, DEFAULT_OPTIONS, o, {
      screenValue: Object.assign({}, DEFAULT_OPTIONS.screenValue, o.screenValue || {}),
      hardwareValue: Object.assign({}, DEFAULT_OPTIONS.hardwareValue, o.hardwareValue || {}),
      language: LANGUAGES.includes(o.language) ? o.language : DEFAULT_OPTIONS.language
    })
  };
}

function dedupe(sites) {
  const seen = new Map();
  for (const site of sites) seen.set(site.pattern, site);
  return [...seen.values()];
}

function normalizeSite(entry) {
  if (!entry || typeof entry !== 'object') return null;
  const pattern = normalizeHost(entry.pattern);
  if (!pattern) return null;
  return {
    pattern,
    enabled: entry.enabled !== false,
    mode: ['spoof', 'off', 'random'].includes(entry.mode) ? entry.mode : 'spoof',
    profile: Object.assign({}, DEFAULT_PROFILE, entry.profile || {}),
    subdomains: entry.subdomains !== false,
    redirects: entry.redirects === true,
    thirdParty: entry.thirdParty !== false,
    note: typeof entry.note === 'string' ? entry.note.slice(0, 120) : ''
  };
}

export function newSite(pattern, profile) {
  return normalizeSite(Object.assign({ pattern, profile }, DEFAULT_SCOPE));
}

/** "https://www.Example.com/x" | "*.example.com" -> "www.example.com" bzw. "example.com" */
export function normalizeHost(input) {
  if (!input) return '';
  let v = String(input).trim().toLowerCase();
  if (!v) return '';
  v = v.replace(/^\*\./, '');
  if (v.includes('://')) {
    try {
      v = new URL(v).hostname;
    } catch {
      v = v.split('://')[1] || v;
    }
  }
  v = v.split('/')[0].split('?')[0].split('#')[0];
  v = v.replace(/:\d+$/, '');
  v = v.replace(/^\.+|\.+$/g, '');
  if (!/^[a-z0-9.\-_]+$/.test(v)) return '';
  return v.includes('.') || v === 'localhost' ? v : '';
}

export function hostOfUrl(url) {
  try {
    const u = new URL(url);
    if (!/^https?:$/.test(u.protocol)) return '';
    return u.hostname.toLowerCase();
  } catch {
    return '';
  }
}

/**
 * Treffendste Regel für einen Host.
 * Exakter Treffer schlägt Subdomain-Treffer, längeres Muster schlägt kürzeres.
 * Pausierte Regeln (enabled: false) zählen nicht mit.
 */
export function findSiteEntry(settings, host) {
  if (!host) return null;
  let best = null;
  let bestScore = -1;
  for (const site of settings.sites) {
    if (!site.enabled) continue;
    let score = -1;
    if (host === site.pattern) score = 1000 + site.pattern.length;
    else if (site.subdomains && host.endsWith('.' + site.pattern)) score = site.pattern.length;
    if (score > bestScore) {
      bestScore = score;
      best = site;
    }
  }
  return bestScore >= 0 ? best : null;
}

/* Stabiler Hash, damit derselbe Schlüssel immer dasselbe Zufallsprofil ergibt. */
function hashKey(key) {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) {
    h ^= key.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}

export function randomProfileFor(key) {
  const base = RANDOM_POOL[hashKey(key || 'chroofer') % RANDOM_POOL.length];
  return Object.assign({}, DEFAULT_PROFILE, base);
}

const GLOBAL_SCOPE = Object.freeze({ subdomains: true, redirects: false, thirdParty: true });

/**
 * Entscheidet für einen Host: spoofen - ja/nein, mit welchem Profil, in welchem
 * Geltungsbereich. -> { active, profile, source, scope, entry }
 */
export function resolveForHost(settings, host) {
  if (!settings.enabled) {
    return { active: false, profile: null, source: 'disabled', scope: GLOBAL_SCOPE, entry: null };
  }

  const entry = findSiteEntry(settings, host);
  if (entry) {
    const scope = { subdomains: entry.subdomains, redirects: entry.redirects, thirdParty: entry.thirdParty };
    if (entry.mode === 'off') {
      return { active: false, profile: null, source: 'site-off', scope, entry };
    }
    /* Gewürfelt wird über das Muster, damit Netzwerk-Regel und Injektion für
       www.example.com dasselbe Profil benutzen wie für example.com. */
    const profile = entry.mode === 'random' ? randomProfileFor(entry.pattern) : entry.profile;
    return { active: true, profile, source: 'site', random: entry.mode === 'random', scope, entry };
  }

  const g = settings.global;
  if (g.mode === 'off') {
    return { active: false, profile: null, source: 'global-off', scope: GLOBAL_SCOPE, entry: null };
  }
  /* Im globalen Zufallsmodus setzt der Service Worker das Sitzungsprofil ein. */
  return {
    active: true,
    profile: g.profile,
    source: 'global',
    random: g.mode === 'random',
    scope: GLOBAL_SCOPE,
    entry: null
  };
}

export async function loadSettings() {
  const raw = await chrome.storage.local.get(STORAGE_KEY);
  return mergeSettings(raw[STORAGE_KEY]);
}

export async function saveSettings(settings) {
  const clean = mergeSettings(settings);
  await chrome.storage.local.set({ [STORAGE_KEY]: clean });
  return clean;
}

/** Profil auf plausible Werte begrenzen (z. B. nach einem Import). */
export function sanitizeProfile(profile) {
  const p = Object.assign({}, DEFAULT_PROFILE, profile || {});
  if (!OS_FAMILIES[p.os]) p.os = DEFAULT_PROFILE.os;
  const fam = OS_FAMILIES[p.os];
  if (!fam.versions.some((v) => v.id === p.osVersion)) p.osVersion = fam.versions[0].id;
  if (!BROWSERS[p.browser]) p.browser = DEFAULT_PROFILE.browser;
  p.browserVersion =
    String(p.browserVersion || BROWSERS[p.browser].versions[0]).replace(/[^0-9.]/g, '') ||
    BROWSERS[p.browser].versions[0];
  p.customUA = typeof p.customUA === 'string' ? p.customUA.slice(0, 512) : '';
  return p;
}

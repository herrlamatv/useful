/* dlplist - by herrlamatv
 * Baut aus Liste + Einstellungen den yt-dlp-Befehl bzw. die ganze Datei
 * (.txt, .bat, .ps1, .sh, Linkliste). Rein, ohne chrome.* - testbar.
 *
 * Grundregel: Argumente bleiben intern ROH (ohne Anführungszeichen) und
 * werden erst beim Ausgeben für die jeweilige Shell gequotet:
 *   bat  "…"  mit % → %%            cmd  "…"  (zum Einfügen in cmd.exe)
 *   ps   '…'  mit ' → ''            sh   '…'  mit ' → '\''
 */

import { CATS, isYouTube } from './links.js';

/* ------------------------------------------------------------------ */
/* Optionen                                                             */
/* ------------------------------------------------------------------ */

export const QUALITIES = ['best', '2160', '1440', '1080', '720', '480', 'mp3', 'm4a', 'audio'];
export const AUDIO_QUALITIES = new Set(['mp3', 'm4a', 'audio']);
export const CONTAINERS = ['mp4', 'mkv', 'original'];
export const TEMPLATES = {
  default: '%(title)s - %(uploader)s [%(id)s].%(ext)s',
  title: '%(title)s.%(ext)s',
  date: '%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s',
  channel: '%(uploader)s/%(title)s [%(id)s].%(ext)s'
};
export const COOKIE_MODES = ['none', 'file', 'browser'];
export const BROWSERS = ['firefox', 'chrome', 'edge', 'brave', 'opera', 'vivaldi', 'chromium', 'safari'];
const CHROMIUM = new Set(['chrome', 'edge', 'brave', 'opera', 'vivaldi', 'chromium']);
export const SPONSORBLOCK = ['off', 'mark', 'remove'];
export const OUTPUT_TYPES = ['txt', 'bat', 'ps1', 'sh', 'links'];
export const SHELLS = ['cmd', 'ps', 'sh'];
export const MODES = ['batch', 'each'];
export const AFTER = ['mark', 'clear', 'none'];
export const LOCATIONS = ['fixed', 'here'];
export const JS_RUNTIMES = ['', 'node', 'quickjs', 'bun'];

export const COMPAT_SORT = 'vcodec:h264,lang,quality,res,fps,hdr:12,acodec:aac';
export const ARCHIVE_FILE = 'dlplist-archive.txt';
export const YTDLP_DOWNLOAD = 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe';

/* Zeilenlänge, ab der aufgeteilt wird (cmd.exe: 8191, CreateProcess: 32767). */
export const LIMITS = { bat: 7800, cmd: 7800, ps: 30000, sh: 30000 };

export const DEFAULT_EXPORT = {
  version: 1,
  cats: {
    videos: { include: true, quality: '1080', folder: '' },
    shorts: { include: true, quality: 'best', folder: '' },
    lists: { include: true, quality: '1080', folder: '', perList: false, limit: 0, breakOnExisting: false },
    other: { include: true, quality: 'best', folder: '' }
  },
  onlyNew: true,
  container: 'mp4',
  recode: true,
  compat: false,
  template: 'default',
  templateCustom: '',
  cookies: 'none',
  cookieFile: 'cookies.txt',
  cookieBrowser: 'firefox',
  extras: {
    thumbnail: false,
    metadata: false,
    chapters: false,
    subs: false,
    subLangs: 'de,en',
    autoSubs: false,
    sponsorblock: 'off',
    archive: false,
    rate: '',
    fragments: 0,
    sleep: false,
    restrict: false,
    simulate: false,
    extraArgs: ''
  },
  env: {
    location: 'fixed',
    path: '%USERPROFILE%\\Desktop\\dw',
    exe: '',
    saveTo: '',
    ffmpeg: '',
    jsRuntime: ''
  },
  output: {
    type: 'bat',
    shell: 'cmd',
    mode: 'batch',
    saveAs: true,
    after: 'mark',
    name: 'dlplist-{date}-{time}'
  },
  bat: {
    pause: true,
    check: true,
    update: false,
    consoleTitle: true,
    openFolder: false,
    shutdown: false
  }
};

/* ------------------------------------------------------------------ */
/* Einstellungen zusammenführen                                         */
/* ------------------------------------------------------------------ */

function isObj(v) {
  return !!v && typeof v === 'object' && !Array.isArray(v);
}

/** Rohdaten in die Form der Vorgabe bringen - Typen passend, Unbekanntes raus. */
function mergeShape(def, raw) {
  const out = {};
  for (const [k, dv] of Object.entries(def)) {
    const rv = isObj(raw) ? raw[k] : undefined;
    if (isObj(dv)) out[k] = mergeShape(dv, rv);
    else if (typeof dv === 'boolean') out[k] = rv === undefined ? dv : !!rv;
    else if (typeof dv === 'number') out[k] = rv !== '' && rv !== null && Number.isFinite(Number(rv)) ? Number(rv) : dv;
    else if (typeof dv === 'string') out[k] = typeof rv === 'string' ? rv : dv;
    else out[k] = dv;
  }
  return out;
}

function oneOf(value, list, fallback) {
  return list.includes(value) ? value : fallback;
}

function intIn(n, min, max) {
  return Math.min(max, Math.max(min, Math.round(Number(n) || 0)));
}

export function mergeExport(raw) {
  const c = mergeShape(DEFAULT_EXPORT, raw);
  for (const cat of CATS) {
    c.cats[cat].quality = oneOf(c.cats[cat].quality, QUALITIES, DEFAULT_EXPORT.cats[cat].quality);
  }
  c.cats.lists.limit = intIn(c.cats.lists.limit, 0, 100000);
  c.container = oneOf(c.container, CONTAINERS, 'mp4');
  c.template = oneOf(c.template, Object.keys(TEMPLATES).concat('custom'), 'default');
  c.cookies = oneOf(c.cookies, COOKIE_MODES, 'none');
  c.cookieBrowser = oneOf(c.cookieBrowser, BROWSERS, 'firefox');
  c.extras.sponsorblock = oneOf(c.extras.sponsorblock, SPONSORBLOCK, 'off');
  c.extras.fragments = intIn(c.extras.fragments, 0, 32);
  c.env.location = oneOf(c.env.location, LOCATIONS, 'fixed');
  c.env.jsRuntime = oneOf(c.env.jsRuntime, JS_RUNTIMES, '');
  c.output.type = oneOf(c.output.type, OUTPUT_TYPES, 'bat');
  c.output.shell = oneOf(c.output.shell, SHELLS, 'cmd');
  c.output.mode = oneOf(c.output.mode, MODES, 'batch');
  c.output.after = oneOf(c.output.after, AFTER, 'mark');
  return c;
}

/* ------------------------------------------------------------------ */
/* Vorlagen (nur das Format - Pfad und Ausgabe bleiben, wie sie sind)   */
/* ------------------------------------------------------------------ */

const FORMAT_KEYS = ['cats', 'container', 'recode', 'compat', 'template', 'templateCustom', 'cookies', 'cookieFile', 'cookieBrowser', 'extras'];

export function formatOf(cfg) {
  const c = mergeExport(cfg);
  const out = {};
  for (const k of FORMAT_KEYS) out[k] = c[k];
  return JSON.parse(JSON.stringify(out));
}

function deepMerge(a, b) {
  if (!isObj(b)) return b === undefined ? a : b;
  const out = Object.assign({}, isObj(a) ? a : {});
  for (const [k, v] of Object.entries(b)) out[k] = isObj(v) && isObj(out[k]) ? deepMerge(out[k], v) : v;
  return out;
}

export function applyFormat(cfg, format) {
  const picked = {};
  for (const k of FORMAT_KEYS) if (isObj(format) && format[k] !== undefined) picked[k] = format[k];
  return mergeExport(deepMerge(mergeExport(cfg), picked));
}

export function presetMatches(cfg, format) {
  return JSON.stringify(formatOf(applyFormat(cfg, format))) === JSON.stringify(formatOf(cfg));
}

const OFF_EXTRAS = {
  thumbnail: false,
  metadata: false,
  chapters: false,
  subs: false,
  autoSubs: false,
  sponsorblock: 'off',
  archive: false,
  sleep: false
};

function qualities(videos, shorts, lists, other) {
  return { videos: { quality: videos }, shorts: { quality: shorts }, lists: { quality: lists }, other: { quality: other } };
}

const SCRIPTS = {
  cats: qualities('1080', 'best', '1080', 'best'),
  container: 'mp4',
  recode: true,
  compat: false,
  template: 'default',
  cookies: 'none',
  extras: OFF_EXTRAS
};

/** Fest eingebaute Vorlagen. Die ersten vier entsprechen den Skripten in useful/yt-dlp/. */
export const BUILTIN_PRESETS = [
  { id: 'scripts', format: SCRIPTS },
  { id: 'best', format: Object.assign({}, SCRIPTS, { cats: qualities('best', 'best', 'best', 'best') }) },
  { id: 'cookie', format: Object.assign({}, SCRIPTS, { cookies: 'file', cookieFile: 'cookies.txt' }) },
  { id: 'mp3', format: Object.assign({}, SCRIPTS, { cats: qualities('mp3', 'mp3', 'mp3', 'mp3') }) },
  {
    id: 'library',
    format: Object.assign({}, SCRIPTS, {
      compat: true,
      extras: Object.assign({}, OFF_EXTRAS, { thumbnail: true, metadata: true, sponsorblock: 'mark', archive: true })
    })
  }
];

/* ------------------------------------------------------------------ */
/* Säubern, Pfade, Quoting                                              */
/* ------------------------------------------------------------------ */

const CTRL = /[\u0000-\u001f\u007f]/g;

/** " und Steuerzeichen entfernen - die würden jedes Quoting sprengen. */
export function sanitize(s) {
  return String(s == null ? '' : s).replace(CTRL, '').replace(/"/g, '');
}

/**
 * Pfad verstehen. Umgebungsvariablen am Anfang werden je Ziel übersetzt:
 *   %USERPROFILE%\x  ~\x  $HOME/x  $env:USERPROFILE\x   → base 'home'
 *   %APPDATA%\x  $env:APPDATA\x                          → base 'env:APPDATA'
 * Intern gilt der Backslash als Trenner, am Ende steht nie einer.
 */
export function parsePath(raw) {
  let s = sanitize(raw).trim();
  if (!s) return null;
  let base = null;
  let m;
  if (
    (m = /^%USERPROFILE%/i.exec(s)) ||
    (m = /^\$\{?env:USERPROFILE\}?/i.exec(s)) ||
    (m = /^~(?=[\\/]|$)/.exec(s)) ||
    (m = /^\$\{HOME\}/.exec(s)) ||
    (m = /^\$HOME(?![A-Za-z0-9_])/.exec(s))
  ) {
    base = 'home';
  } else if ((m = /^%([A-Za-z_][A-Za-z0-9_]*)%/.exec(s)) || (m = /^\$\{?env:([A-Za-z_][A-Za-z0-9_]*)\}?/i.exec(s))) {
    base = 'env:' + m[1].toUpperCase();
  }
  if (m && base) s = s.slice(m[0].length);

  const unc = !base && /^[\\/]{2}[^\\/]/.test(s);
  s = s.replace(/\//g, '\\').replace(/\\{2,}/g, '\\');
  if (unc) s = '\\' + s;
  if (s.length > 1) s = s.replace(/\\+$/, '');
  if (s === '\\' && base) s = '';
  if (/^[A-Za-z]:\\?$/.test(s)) s = s.slice(0, 2) + '\\.'; // C:\ → C:\.  (sonst \" im Quoting)
  if (base && s && !s.startsWith('\\')) s = '\\' + s;
  return { base, rest: s };
}

/** Unterordner säubern: keine Laufwerke, kein .., keine verbotenen Zeichen. */
export function cleanFolder(raw) {
  return sanitize(raw)
    .replace(/[<>|?*:]/g, '')
    .split(/[\\/]+/)
    .map((p) => p.trim())
    .filter((p) => p && p !== '.' && p !== '..')
    .join('\\');
}

function joinPath(p, sub) {
  if (!sub) return p;
  if (!p) return { base: null, rest: sub };
  const rest = p.rest && p.rest !== '\\' ? p.rest.replace(/\\\.$/, '') + '\\' + sub : '\\' + sub;
  return { base: p.base, rest: p.base || p.rest ? rest : sub };
}

const SAFE = /^[A-Za-z0-9._-]+$/;

export function quote(s, target) {
  switch (target) {
    case 'bat':
      return '"' + s.replace(/%/g, '%%') + '"';
    case 'cmd':
      return '"' + s + '"';
    case 'ps':
      return "'" + s.replace(/'/g, "''") + "'";
    case 'sh':
      return "'" + s.replace(/'/g, "'\\''") + "'";
    default:
      throw new Error('unknown target ' + target);
  }
}

function envName(base, target) {
  if (base === 'home') return target === 'sh' ? 'HOME' : 'USERPROFILE';
  return base.slice(4);
}

/** Pfad ohne äußere Anführungszeichen (für set "X=…"). Nur bat/cmd. */
function pathInner(p, target) {
  const rest = target === 'bat' ? p.rest.replace(/%/g, '%%') : p.rest;
  return p.base ? '%' + envName(p.base, target) + '%' + rest : rest;
}

export function renderPath(p, target) {
  if (!p) return quote('.', target);
  if (!p.base) return quote(target === 'sh' ? p.rest.replace(/\\/g, '/') : p.rest || '.', target);
  switch (target) {
    case 'bat':
    case 'cmd':
      return '"' + pathInner(p, target) + '"';
    case 'ps':
      return '"${env:' + envName(p.base, 'ps') + '}' + p.rest.replace(/[`$"]/g, (c) => '`' + c) + '"';
    case 'sh':
      return '"${' + envName(p.base, 'sh') + '}' + p.rest.replace(/\\/g, '/').replace(/[\\$`"]/g, (c) => '\\' + c) + '"';
    default:
      throw new Error('unknown target ' + target);
  }
}

/* Token-Arten: 'text' (bleibt nackt, wenn harmlos), Q() (immer quoten), P() (Pfad) */
const Q = (v) => ({ v: String(v) });
const P = (path) => ({ path });

function renderToken(tok, target) {
  if (typeof tok === 'string') return SAFE.test(tok) ? tok : quote(tok, target);
  if (tok.path) return renderPath(tok.path, target);
  return quote(tok.v, target);
}

/** Zusatzargumente wie in einer Shell zerlegen ("…" und '…' halten zusammen). */
export function splitArgs(s) {
  const out = [];
  let cur = '';
  let q = null;
  let has = false;
  for (const ch of String(s || '')) {
    if (q) {
      if (ch === q) q = null;
      else cur += ch;
      continue;
    }
    if (ch === '"' || ch === "'") {
      q = ch;
      has = true;
      continue;
    }
    if (/\s/.test(ch)) {
      if (cur || has) out.push(cur);
      cur = '';
      has = false;
      continue;
    }
    cur += ch;
  }
  if (cur || has) out.push(cur);
  return out.map((x) => x.replace(CTRL, '').replace(/"/g, ''));
}

/** Nur saubere http(s)-Adressen ohne Leerzeichen und " (new URL kodiert sie). */
function safeUrl(url) {
  try {
    const u = new URL(url);
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return null;
    const href = u.href;
    if (/[\s"]/.test(href)) return null;
    return href;
  } catch (e) {
    return null;
  }
}

/* ------------------------------------------------------------------ */
/* Argumente pro Kategorie                                              */
/* ------------------------------------------------------------------ */

function templateOf(cfg) {
  if (cfg.template === 'custom') {
    const custom = sanitize(cfg.templateCustom).trim();
    return custom || TEMPLATES.default;
  }
  return TEMPLATES[cfg.template] || TEMPLATES.default;
}

/**
 * yt-dlp-Argumente für eine Kategorie - ohne Programm und ohne Links.
 * Reihenfolge wie in den eigenen Skripten: -f, -S, merge, recode/remux,
 * -x …, cookies, -P, -o, Extras.
 */
export function categoryArgs(cat, cfgRaw, warn = () => {}) {
  const cfg = mergeExport(cfgRaw);
  const cc = cfg.cats[cat];
  const x = cfg.extras;
  const audio = AUDIO_QUALITIES.has(cc.quality);
  const args = [];

  // Format
  if (cc.quality === 'mp3') args.push('-f', Q('bestaudio/best'), '-x', '--audio-format', 'mp3', '--audio-quality', '320K');
  else if (cc.quality === 'm4a') args.push('-f', Q('bestaudio[ext=m4a]/bestaudio/best'), '-x', '--audio-format', 'm4a');
  else if (cc.quality === 'audio') args.push('-f', Q('bestaudio/best'), '-x');
  else {
    const h = cc.quality;
    args.push('-f', Q(h === 'best' ? 'bestvideo+bestaudio/best' : `bestvideo[height<=${h}]+bestaudio/best[height<=${h}]`));
    if (cfg.compat) {
      args.push('-S', Q(COMPAT_SORT));
      if (h === '2160' || h === '1440') warn('compatHighRes', { cat }, 'warn');
    }
    if (cfg.container === 'mp4') args.push('--merge-output-format', 'mp4', cfg.recode ? '--recode-video' : '--remux-video', 'mp4');
    else if (cfg.container === 'mkv') args.push('--merge-output-format', 'mkv', '--remux-video', 'mkv');
    if (x.thumbnail && cfg.container === 'original') warn('thumbOriginal', {}, 'warn');
  }

  // Cookies
  if (cfg.cookies === 'file') {
    const file = parsePath(cfg.cookieFile) || parsePath('cookies.txt');
    args.push('--cookies', P(file));
  } else if (cfg.cookies === 'browser') {
    args.push('--cookies-from-browser', cfg.cookieBrowser);
  }

  // Speicherort: optional woanders + Unterordner
  const folder = cleanFolder(cc.folder);
  const saveTo = parsePath(cfg.env.saveTo);
  const where = joinPath(saveTo, folder);
  if (where) args.push('-P', P(where));

  // Dateiname
  let template = templateOf(cfg);
  if (cat === 'lists' && cc.perList) template = '%(playlist_title,playlist_id)s/' + template;
  args.push('-o', Q(template));

  // Extras
  if (x.thumbnail) args.push('--embed-thumbnail');
  if (x.metadata) args.push('--embed-metadata');
  if (x.chapters && !x.metadata) args.push('--embed-chapters'); // --embed-metadata bettet Kapitel schon ein
  if (x.subs) {
    if (audio) warn('audioNoSubs', { cat }, 'info');
    else {
      const langs = String(x.subLangs || '').replace(/[^A-Za-z0-9,.*_-]/g, '') || 'de,en';
      args.push('--embed-subs', '--sub-langs', Q(langs));
      if (x.autoSubs) args.push('--write-auto-subs');
    }
  }
  if (x.sponsorblock === 'mark') args.push('--sponsorblock-mark', 'all');
  else if (x.sponsorblock === 'remove') args.push('--sponsorblock-remove', 'default');

  const breakOn = cat === 'lists' && cc.breakOnExisting;
  if (x.archive || breakOn) args.push('--download-archive', Q(ARCHIVE_FILE));
  if (breakOn && !x.archive) warn('archiveForced', {}, 'info');
  if (cat === 'lists' && cc.limit > 0) args.push('-I', Q('1:' + cc.limit));
  if (breakOn) args.push('--break-on-existing', '--break-per-input');

  const rate = String(x.rate || '').trim();
  if (rate) {
    if (/^\d+(\.\d+)?[KMG]?$/i.test(rate)) args.push('-r', rate.toUpperCase());
    else warn('badRate', { value: rate }, 'warn');
  }
  if (x.fragments > 1) args.push('-N', String(x.fragments));
  if (x.sleep) {
    args.push('--sleep-requests', '0.75', '--sleep-interval', '10', '--max-sleep-interval', '20');
    if (x.subs && !audio) args.push('--sleep-subtitles', '5');
  }
  if (x.restrict) args.push('--restrict-filenames');
  const ffmpeg = parsePath(cfg.env.ffmpeg);
  if (ffmpeg) args.push('--ffmpeg-location', P(ffmpeg));
  if (cfg.env.jsRuntime) args.push('--no-js-runtimes', '--js-runtimes', cfg.env.jsRuntime);
  if (x.simulate) args.push('--simulate');
  for (const extra of splitArgs(x.extraArgs)) args.push(extra);
  return args;
}

/* ------------------------------------------------------------------ */
/* Plan: was wird exportiert?                                           */
/* ------------------------------------------------------------------ */

export function planExport(items, cfgRaw) {
  const cfg = mergeExport(cfgRaw);
  const warnings = [];
  const warn = (code, vars = {}, level = 'warn') => {
    const id = code + JSON.stringify(vars);
    if (!warnings.some((w) => w.id === id)) warnings.push({ id, code, vars, level });
  };
  const groups = [];
  let skipped = 0;
  let bad = 0;
  for (const cat of CATS) {
    const cc = cfg.cats[cat];
    if (!cc.include) continue;
    const chosen = [];
    for (const it of items) {
      if (it.cat !== cat) continue;
      if (cfg.onlyNew && it.exported) {
        skipped++;
        continue;
      }
      const url = safeUrl(it.url);
      if (!url) {
        bad++;
        continue;
      }
      chosen.push(Object.assign({}, it, { url }));
    }
    if (!chosen.length) continue;
    groups.push({ cat, quality: cc.quality, items: chosen, urls: chosen.map((it) => it.url), args: categoryArgs(cat, cfg, warn) });
    if (cat === 'lists' && !cc.limit && !cc.breakOnExisting) warn('listsUnbounded', {}, 'info');
  }

  const total = groups.reduce((n, g) => n + g.urls.length, 0);
  if (!total) warn(skipped ? 'noItemsSkipped' : 'noItems', skipped ? { n: skipped } : {}, 'info');
  if (bad) warn('badUrl', { n: bad }, 'warn');
  const gone = groups.reduce((n, g) => n + g.items.filter((it) => it.status === 'unavailable').length, 0);
  if (gone) warn('unavailable', { n: gone }, 'warn');
  if (cfg.cookies === 'browser' && CHROMIUM.has(cfg.cookieBrowser)) warn('chromiumCookies', { browser: cfg.cookieBrowser }, 'warn');
  if (cfg.extras.simulate) warn('simulate', {}, 'info');
  for (const [field, value] of [
    ['path', cfg.env.path],
    ['exe', cfg.env.exe],
    ['saveTo', cfg.env.saveTo],
    ['ffmpeg', cfg.env.ffmpeg],
    ['template', cfg.template === 'custom' ? cfg.templateCustom : '']
  ]) {
    if (/["\u0000-\u001f]/.test(String(value || ''))) warn('sanitized', { field }, 'warn');
  }
  // In den Zusatzargumenten sind Anführungszeichen normales Quoting - nur Steuerzeichen melden
  if (/[\u0000-\u001f]/.test(cfg.extras.extraArgs)) warn('sanitized', { field: 'extraArgs' }, 'warn');

  return {
    cfg,
    groups,
    total,
    skipped,
    keys: groups.flatMap((g) => g.items.map((it) => it.key)),
    hasYouTube: groups.some((g) => g.items.some(isYouTube)),
    warnings
  };
}

/* ------------------------------------------------------------------ */
/* Texte für die erzeugten Dateien                                      */
/* ------------------------------------------------------------------ */

const FOLD = {
  ä: 'ae', ö: 'oe', ü: 'ue', Ä: 'Ae', Ö: 'Oe', Ü: 'Ue', ß: 'ss',
  '–': '-', '—': '-', '·': '-', '…': '...', '’': "'", '‘': "'", '„': '', '“': '', '”': '', '«': '', '»': '', '×': 'x', '→': '->'
};

/** Auf ASCII falten (cmd.exe liest .bat in der OEM-Codepage). */
export function asciiFold(s) {
  return String(s)
    .replace(/[äöüÄÖÜß–—·…’‘„“”«»×→]/g, (c) => FOLD[c])
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^\x20-\x7e]/g, '')
    .replace(/ {2,}/g, ' ')
    .trim();
}

/** Text für echo/title in .bat: ASCII, Sonderzeichen mit ^, % verdoppelt. */
function batText(s) {
  return asciiFold(s)
    .replace(/"/g, "'")
    .replace(/[\^&|<>]/g, (c) => '^' + c)
    .replace(/%/g, '%%');
}

/** Text für ::-Kommentare in .bat: riskante Zeichen einfach weglassen. */
function batComment(s) {
  return asciiFold(s).replace(/[%^&|<>"]/g, '');
}

/** Einzeiliger Kommentar für ps/sh/txt. */
function lineComment(s) {
  return String(s).replace(CTRL, ' ').trim();
}

function pad2(n) {
  return String(n).padStart(2, '0');
}

function stamp(now) {
  const d = now.getFullYear() + '-' + pad2(now.getMonth() + 1) + '-' + pad2(now.getDate());
  return { date: d, time: pad2(now.getHours()) + pad2(now.getMinutes()), human: d + ' ' + pad2(now.getHours()) + ':' + pad2(now.getMinutes()) };
}

export function makeFilename(pattern, now, ext, count) {
  const s = stamp(now);
  let name = String(pattern || '')
    .split('{date}').join(s.date)
    .split('{time}').join(s.time)
    .split('{count}').join(String(count));
  name = sanitize(name).replace(/[\\/:*?<>|]/g, '-').replace(/^[.\s]+|[.\s]+$/g, '').slice(0, 120);
  return (name || 'dlplist') + '.' + ext;
}

/* ------------------------------------------------------------------ */
/* Ausgabe                                                              */
/* ------------------------------------------------------------------ */

const TYPE_INFO = {
  bat: { target: 'bat', ext: 'bat', mime: 'application/octet-stream', bom: false, eol: '\r\n' },
  ps1: { target: 'ps', ext: 'ps1', mime: 'application/octet-stream', bom: true, eol: '\r\n' },
  sh: { target: 'sh', ext: 'sh', mime: 'application/octet-stream', bom: false, eol: '\n' },
  links: { target: null, ext: 'txt', mime: 'text/plain', bom: false, eol: '\r\n' }
};

function typeInfo(cfg) {
  if (cfg.output.type !== 'txt') return TYPE_INFO[cfg.output.type];
  const shell = cfg.output.shell;
  return { target: shell, ext: 'txt', mime: 'text/plain', bom: false, eol: shell === 'sh' ? '\n' : '\r\n' };
}

/** Links einer Gruppe so auf Zeilen verteilen, dass keine zu lang wird. */
function chunkUrls(baseLen, rendered, budget, warn) {
  const chunks = [];
  let cur = [];
  let len = baseLen;
  for (const u of rendered) {
    if (baseLen + 1 + u.length > budget) warn('longUrl', {}, 'warn');
    if (cur.length && len + 1 + u.length > budget) {
      chunks.push(cur);
      cur = [];
      len = baseLen;
    }
    cur.push(u);
    len += 1 + u.length;
  }
  if (cur.length) chunks.push(cur);
  return chunks;
}

function makeCtx(plan, opts) {
  const t = opts.t || ((k) => k);
  const now = opts.now instanceof Date ? opts.now : new Date(opts.now || Date.now());
  const cfg = plan.cfg;
  const s = stamp(now);
  const summary = plan.groups.map((g) => t('cat.' + g.cat) + ' ' + g.urls.length).join(', ');
  const header = t('script.header', { date: s.human, n: plan.total, summary: summary || '-' });
  const exePath = parsePath(cfg.env.exe);
  const savePath = cfg.env.location === 'fixed' ? parsePath(cfg.env.path) : null;
  return {
    t,
    now,
    cfg,
    plan,
    header,
    exePath,
    savePath,
    here: !savePath,
    title: t('script.title', { n: plan.total }),
    heading: (g) =>
      t('script.heading', {
        cat: t('cat.' + g.cat),
        n: g.urls.length,
        links: t(g.urls.length === 1 ? 'script.link' : 'script.links'),
        quality: t('quality.' + g.quality)
      }),
    label: (it) => asciiSafeLabel(it),
    warnings: [],
    warn(code, vars = {}, level = 'warn') {
      const id = code + JSON.stringify(vars);
      if (!this.warnings.some((w) => w.id === id)) this.warnings.push({ id, code, vars, level });
    }
  };
}

function asciiSafeLabel(it) {
  const text = it.title ? it.title + (it.author ? ' - ' + it.author : '') : it.url;
  return String(text).replace(CTRL, ' ').slice(0, 110);
}

function argsFor(group, ctx, target, withTitle) {
  const toks = group.args.slice();
  if (withTitle && ctx.cfg.bat.consoleTitle) toks.push('--console-title');
  return toks.map((tok) => renderToken(tok, target)).join(' ');
}

/* ------------------------------- .bat -------------------------------- */

function renderBat(ctx) {
  const { cfg, plan, t } = ctx;
  const L = [];
  const exe = ctx.exePath ? renderPath(ctx.exePath, 'bat') : 'yt-dlp';
  const calls = cfg.output.mode === 'each' ? plan.total : null;
  const subs = [];

  L.push('@echo off');
  L.push('setlocal');
  // yt-dlp.exe im Ordner immer finden - manche Umgebungen schalten das per Variable ab
  if (!ctx.exePath) L.push('set "NoDefaultCurrentDirectoryInExePath="');
  L.push('title ' + batText(ctx.title));
  L.push(':: ' + batComment(ctx.header));
  L.push(':: ' + batComment(t('script.generated')));
  listing(L, ctx, (s) => ':: ' + batComment(s));
  L.push('');

  if (ctx.savePath) {
    L.push('set "SAVE_DIR=' + pathInner(ctx.savePath, 'bat') + '"');
    L.push('if not exist "%SAVE_DIR%\\" mkdir "%SAVE_DIR%"');
  } else {
    L.push('set "SAVE_DIR=%~dp0"');
  }
  L.push('pushd "%SAVE_DIR%" || goto :nodir');
  if (cfg.bat.check) {
    L.push(ctx.exePath ? 'if not exist ' + exe + ' goto :noexe' : 'where /q yt-dlp || goto :noexe');
    if (!cfg.env.jsRuntime && plan.hasYouTube) L.push('where /q deno || echo   ' + batText(t('script.denoHint')));
  }
  if (cfg.bat.update) L.push(exe + ' -U');
  L.push('set "FAILS=0"');
  if (calls) L.push('set "N=0"');
  L.push('echo.');
  L.push('echo   ' + batText(ctx.title));
  L.push('echo   ' + batText(t('script.folder')) + ' "%CD%"');
  L.push('echo.');
  L.push('');

  let lineCount = 0;
  for (const g of plan.groups) {
    const base = exe + ' ' + argsFor(g, ctx, 'bat', true);
    L.push('echo   ' + batText(ctx.heading(g)));
    if (calls) {
      const label = 'dl_' + g.cat;
      for (const url of g.urls) {
        L.push('set "U=' + url.replace(/%/g, '%%') + '"');
        L.push('call :' + label);
      }
      subs.push(':' + label, 'set /a N+=1', 'echo   [%N%/' + calls + '] "%U%"', base + ' "%U%"', 'if errorlevel 1 set /a FAILS+=1', 'exit /b', '');
    } else {
      const rendered = g.urls.map((u) => quote(u, 'bat'));
      for (const chunk of chunkUrls(base.length, rendered, LIMITS.bat, ctx.warn.bind(ctx))) {
        L.push(base + ' ' + chunk.join(' '));
        L.push('if errorlevel 1 set /a FAILS+=1');
        lineCount++;
      }
    }
    L.push('');
  }

  const total = calls || lineCount;
  L.push('echo.');
  L.push('if "%FAILS%"=="0" echo   ' + batText(t('script.done')));
  L.push('if not "%FAILS%"=="0" echo   ' + batText(t('script.doneFails', { total })).replace('{fails}', '%FAILS%'));
  if (cfg.bat.openFolder) L.push('start "" "%CD%"');
  if (cfg.bat.shutdown) {
    L.push('echo   ' + batText(t('script.shutdown')));
    L.push('shutdown /s /t 120');
  }
  L.push('popd');
  if (cfg.bat.pause) L.push('pause');
  L.push('exit /b 0');
  L.push('');
  if (subs.length) L.push(...subs);
  L.push(':nodir');
  L.push('echo   ' + batText(t('script.noDir')) + ' "%SAVE_DIR%"');
  L.push('pause');
  L.push('exit /b 1');
  L.push('');
  L.push(':noexe');
  L.push('echo   ' + batText(t('script.noExe')) + ' "%CD%"');
  L.push('echo   Download: ' + YTDLP_DOWNLOAD);
  L.push('popd');
  L.push('pause');
  L.push('exit /b 1');

  // Nicht-ASCII (z. B. Umlaut im Pfad) → Konsole auf UTF-8 stellen
  if (L.some((line) => /[^\x00-\x7f]/.test(line))) {
    L.splice(1, 0, 'chcp 65001 >nul');
    ctx.warn('utf8', {}, 'info');
  }
  return L;
}

/** Titelliste als Kommentar (hilft, die Datei später wiederzuerkennen). */
function listing(L, ctx, fmt) {
  const MAX = 300;
  let n = 0;
  for (const g of ctx.plan.groups) {
    for (const it of g.items) {
      if (n++ >= MAX) continue;
      L.push(fmt('  [' + ctx.t('cat.' + g.cat) + '] ' + ctx.label(it)));
    }
  }
  if (n > MAX) L.push(fmt('  … +' + (n - MAX)));
}

/* --------------------------- .txt für cmd ---------------------------- */

function renderTxtCmd(ctx) {
  const { cfg, plan, t } = ctx;
  const L = [];
  const exe = ctx.exePath ? renderPath(ctx.exePath, 'cmd') : 'yt-dlp';
  L.push('rem ' + lineComment(ctx.header));
  L.push('rem ' + lineComment(t(ctx.here ? 'script.pasteCmdHere' : 'script.pasteCmd')));
  if (ctx.savePath) L.push('cd /d "' + pathInner(ctx.savePath, 'cmd') + '"');
  if (cfg.bat.update) L.push(exe + ' -U');
  for (const g of plan.groups) {
    const base = exe + ' ' + argsFor(g, ctx, 'cmd', false);
    const rendered = g.urls.map((u) => quote(u, 'cmd'));
    if (cfg.output.mode === 'each') for (const u of rendered) L.push(base + ' ' + u);
    else for (const chunk of chunkUrls(base.length, rendered, LIMITS.cmd, ctx.warn.bind(ctx))) L.push(base + ' ' + chunk.join(' '));
  }
  return L;
}

/* ---------------------------- PowerShell ----------------------------- */

const PS_FIND_EXE = "$ytdlp = if (Test-Path -LiteralPath '.\\yt-dlp.exe') { '.\\yt-dlp.exe' } else { 'yt-dlp' }";

function psExeLine(ctx) {
  return ctx.exePath ? '$ytdlp = ' + renderPath(ctx.exePath, 'ps') : PS_FIND_EXE;
}

function renderPs1(ctx) {
  const { cfg, plan, t } = ctx;
  const L = [];
  const q = (s) => quote(s, 'ps');
  const pause = cfg.bat.pause ? ' ' + 'Read-Host ' + q(t('script.pressEnter')) + ' | Out-Null;' : '';

  L.push('# ' + lineComment(ctx.header));
  L.push('# ' + lineComment(t('script.generated')));
  L.push('# ' + lineComment(t('script.runPs1')));
  listing(L, ctx, (s) => '# ' + lineComment(s));
  L.push('');
  L.push('$Host.UI.RawUI.WindowTitle = ' + q(ctx.title));
  if (ctx.savePath) {
    L.push('$SaveDir = ' + renderPath(ctx.savePath, 'ps'));
    L.push('New-Item -ItemType Directory -Force -Path $SaveDir | Out-Null');
    L.push('Set-Location -LiteralPath $SaveDir');
  } else {
    L.push('Set-Location -LiteralPath $PSScriptRoot');
  }
  L.push(psExeLine(ctx));
  if (cfg.bat.check) {
    L.push('if (-not (Get-Command $ytdlp -ErrorAction SilentlyContinue)) {');
    L.push('  Write-Host (' + q(t('script.noExe') + ' ') + ' + (Get-Location)) -ForegroundColor Red');
    L.push("  Write-Host 'Download: " + YTDLP_DOWNLOAD + "'");
    L.push('  ' + (pause ? pause.trim() + ' ' : '') + 'exit 1');
    L.push('}');
    if (!cfg.env.jsRuntime && plan.hasYouTube) {
      L.push('if (-not (Get-Command deno -ErrorAction SilentlyContinue)) { Write-Host ' + q(t('script.denoHint')) + ' -ForegroundColor Yellow }');
    }
  }
  if (cfg.bat.update) L.push('& $ytdlp -U');
  L.push('$fails = 0');
  if (cfg.output.mode === 'each') L.push('$n = 0');
  L.push('Write-Host ' + q(ctx.title));
  L.push('Write-Host (' + q(t('script.folder') + ' ') + ' + (Get-Location))');
  L.push('');

  let lineCount = 0;
  for (const g of plan.groups) {
    const args = argsFor(g, ctx, 'ps', true);
    L.push('Write-Host ' + q(ctx.heading(g)) + ' -ForegroundColor Cyan');
    const rendered = g.urls.map(q);
    if (cfg.output.mode === 'each') {
      pushArray(L, rendered);
      L.push('foreach ($u in $urls) {');
      L.push('  $n++');
      L.push("  Write-Host ('[{0}/{1}] {2}' -f $n, " + plan.total + ', $u)');
      L.push('  & $ytdlp ' + args + ' $u');
      L.push('  if ($LASTEXITCODE -ne 0) { $fails++ }');
      L.push('}');
    } else {
      for (const chunk of chunkUrls(args.length + 16, rendered, LIMITS.ps, ctx.warn.bind(ctx))) {
        pushArray(L, chunk);
        L.push('& $ytdlp ' + args + ' $urls');
        L.push('if ($LASTEXITCODE -ne 0) { $fails++ }');
        lineCount++;
      }
    }
    L.push('');
  }

  const total = cfg.output.mode === 'each' ? plan.total : lineCount;
  const failText = q(t('script.doneFails', { total }).replace('{fails}', '{0}'));
  L.push('if ($fails -eq 0) { Write-Host ' + q(t('script.done')) + ' -ForegroundColor Green }');
  L.push('else { Write-Host (' + failText + ' -f $fails) -ForegroundColor Yellow }');
  if (cfg.bat.openFolder) L.push('Invoke-Item -LiteralPath .');
  if (cfg.bat.shutdown) {
    L.push('Write-Host ' + q(t('script.shutdown')) + ' -ForegroundColor Yellow');
    L.push('shutdown /s /t 120');
  }
  if (cfg.bat.pause) L.push('Read-Host ' + q(t('script.pressEnter')) + ' | Out-Null');
  return L;
}

function pushArray(L, rendered) {
  L.push('$urls = @(');
  for (const u of rendered) L.push('  ' + u);
  L.push(')');
}

function renderTxtPs(ctx) {
  const { cfg, plan, t } = ctx;
  const L = [];
  L.push('# ' + lineComment(ctx.header));
  L.push('# ' + lineComment(t(ctx.here ? 'script.pastePsHere' : 'script.pastePs')));
  if (ctx.savePath) L.push('Set-Location -LiteralPath ' + renderPath(ctx.savePath, 'ps'));
  L.push(psExeLine(ctx));
  if (cfg.bat.update) L.push('& $ytdlp -U');
  for (const g of plan.groups) {
    const base = '& $ytdlp ' + argsFor(g, ctx, 'ps', false);
    const rendered = g.urls.map((u) => quote(u, 'ps'));
    if (cfg.output.mode === 'each') for (const u of rendered) L.push(base + ' ' + u);
    else for (const chunk of chunkUrls(base.length, rendered, LIMITS.ps, ctx.warn.bind(ctx))) L.push(base + ' ' + chunk.join(' '));
  }
  return L;
}

/* ------------------------------- Bash -------------------------------- */

const SH_FIND_EXE = 'if [ -x ./yt-dlp ]; then YTDLP=./yt-dlp; elif [ -x ./yt-dlp.exe ]; then YTDLP=./yt-dlp.exe; else YTDLP=yt-dlp; fi';

function shExeLine(ctx) {
  return ctx.exePath ? 'YTDLP=' + renderPath(ctx.exePath, 'sh') : SH_FIND_EXE;
}

function renderSh(ctx) {
  const { cfg, plan, t } = ctx;
  const L = [];
  const q = (s) => quote(s, 'sh');

  L.push('#!/usr/bin/env bash');
  L.push('# ' + lineComment(ctx.header));
  L.push('# ' + lineComment(t('script.generated')));
  listing(L, ctx, (s) => '# ' + lineComment(s));
  L.push('');
  if (ctx.savePath) {
    L.push('SAVE_DIR=' + renderPath(ctx.savePath, 'sh'));
    L.push('mkdir -p "$SAVE_DIR" && cd "$SAVE_DIR" || { echo ' + q(t('script.noDir')) + ' "$SAVE_DIR"; exit 1; }');
  } else {
    L.push('cd "$(dirname "$0")" || exit 1');
  }
  L.push(shExeLine(ctx));
  if (cfg.bat.check) L.push('command -v "$YTDLP" >/dev/null 2>&1 || { echo ' + q(t('script.noExe')) + ' "$PWD"; exit 1; }');
  if (cfg.bat.update) L.push('"$YTDLP" -U');
  L.push('fails=0');
  if (cfg.output.mode === 'each') L.push('n=0');
  L.push('echo ' + q(ctx.title));
  L.push('');

  let lineCount = 0;
  for (const g of plan.groups) {
    const args = argsFor(g, ctx, 'sh', false);
    L.push('echo ' + q(ctx.heading(g)));
    const rendered = g.urls.map(q);
    if (cfg.output.mode === 'each') {
      L.push('for u in \\');
      rendered.forEach((u, i) => L.push('  ' + u + (i < rendered.length - 1 ? ' \\' : '; do')));
      L.push('  n=$((n+1)); echo "[$n/' + plan.total + '] $u"');
      L.push('  "$YTDLP" ' + args + ' "$u" || fails=$((fails+1))');
      L.push('done');
    } else {
      for (const chunk of chunkUrls(args.length + 10, rendered, LIMITS.sh, ctx.warn.bind(ctx))) {
        L.push('"$YTDLP" ' + args + ' \\');
        chunk.forEach((u, i) => L.push('  ' + u + (i < chunk.length - 1 ? ' \\' : ' || fails=$((fails+1))')));
        lineCount++;
      }
    }
    L.push('');
  }

  const total = cfg.output.mode === 'each' ? plan.total : lineCount;
  const [before, after = ''] = t('script.doneFails', { total }).split('{fails}');
  L.push('if [ "$fails" -eq 0 ]; then echo ' + q(t('script.done')) + '; else echo ' + q(before) + '"$fails"' + q(after) + '; fi');
  if (cfg.bat.pause) L.push('read -r -p ' + q(t('script.pressEnter') + ' ') + ' _');
  return L;
}

function renderTxtSh(ctx) {
  const { cfg, plan, t } = ctx;
  const L = [];
  L.push('# ' + lineComment(ctx.header));
  L.push('# ' + lineComment(t(ctx.here ? 'script.pasteShHere' : 'script.pasteSh')));
  if (ctx.savePath) L.push('cd ' + renderPath(ctx.savePath, 'sh'));
  L.push(shExeLine(ctx));
  if (cfg.bat.update) L.push('"$YTDLP" -U');
  for (const g of plan.groups) {
    const base = '"$YTDLP" ' + argsFor(g, ctx, 'sh', false);
    const rendered = g.urls.map((u) => quote(u, 'sh'));
    if (cfg.output.mode === 'each') for (const u of rendered) L.push(base + ' ' + u);
    else for (const chunk of chunkUrls(base.length, rendered, LIMITS.sh, ctx.warn.bind(ctx))) L.push(base + ' ' + chunk.join(' '));
  }
  return L;
}

/* ----------------------------- Linkliste ----------------------------- */

function renderLinks(ctx, filename) {
  const { plan, t } = ctx;
  const L = [];
  L.push('# ' + lineComment(ctx.header));
  L.push('# ' + lineComment(t('script.linksHint', { file: filename })));
  for (const g of plan.groups) {
    L.push('');
    L.push('# ' + lineComment(ctx.heading(g)));
    for (const u of g.urls) L.push(u);
  }
  return L;
}

/**
 * Alles in einem: Plan + fertiger Text.
 * @param items  Listeneinträge
 * @param cfgRaw Export-Einstellungen
 * @param opts   { t(key, vars), now }
 * @returns {{ text, filename, ext, mime, bom, eol, target, count, keys, groups, warnings }}
 */
export function buildExport(items, cfgRaw, opts = {}) {
  const plan = planExport(items, cfgRaw);
  const ctx = makeCtx(plan, opts);
  const cfg = plan.cfg;
  const info = typeInfo(cfg);
  const filename = makeFilename(cfg.output.name + (cfg.output.type === 'links' ? '-links' : ''), ctx.now, info.ext, plan.total);

  if (cfg.env.location === 'fixed' && !ctx.savePath) ctx.warn('noPath', {}, 'warn');
  if (ctx.here && cfg.output.type !== 'links') ctx.warn(cfg.output.type === 'txt' ? 'hereTxt' : 'hereFile', {}, 'info');

  let lines;
  switch (cfg.output.type) {
    case 'bat':
      lines = renderBat(ctx);
      break;
    case 'ps1':
      lines = renderPs1(ctx);
      break;
    case 'sh':
      lines = renderSh(ctx);
      break;
    case 'links':
      lines = renderLinks(ctx, filename);
      break;
    default:
      lines = cfg.output.shell === 'ps' ? renderTxtPs(ctx) : cfg.output.shell === 'sh' ? renderTxtSh(ctx) : renderTxtCmd(ctx);
  }

  return {
    text: lines.join(info.eol) + info.eol,
    filename,
    ext: info.ext,
    mime: info.mime,
    bom: info.bom,
    eol: info.eol,
    target: info.target,
    count: plan.total,
    skipped: plan.skipped,
    keys: plan.keys,
    groups: plan.groups.map((g) => ({ cat: g.cat, quality: g.quality, n: g.urls.length })),
    warnings: plan.warnings.concat(ctx.warnings)
  };
}

/** Nur die Links (zum Kopieren), eine pro Zeile. */
export function linksOnly(items, cfgRaw) {
  return planExport(items, cfgRaw).groups.flatMap((g) => g.urls).join('\n');
}

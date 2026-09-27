/* dlplist - by herrlamatv
 * Links erkennen, säubern und einsortieren. Rein, ohne chrome.* - läuft
 * genauso im Service Worker, auf der Export-Seite und in den Node-Tests.
 *
 * Ergebnis von parseLink():
 *   { ok: true, key, url, id, kind, cat }   oder   { ok: false, reason }
 *
 *   key   eindeutiger Schlüssel gegen Doppelte (yt:ID, ytl:LISTE, ytc:@kanal, url:…)
 *   url   saubere Adresse, die yt-dlp bekommt
 *   kind  video | short | live | clip | playlist | channel | search | reel | other
 *   cat   videos | shorts | lists | other
 */

export const CATS = ['videos', 'shorts', 'lists', 'other'];

const KIND_CAT = {
  video: 'videos',
  live: 'videos',
  clip: 'videos',
  short: 'shorts',
  reel: 'shorts',
  playlist: 'lists',
  channel: 'lists',
  search: 'lists',
  other: 'other'
};

const YT_ID = /^[A-Za-z0-9_-]{11}$/;
const YT_LIST = /^[A-Za-z0-9_-]{2,64}$/;
const YT = 'https://www.youtube.com';

/* Kanal-Unterseiten, die yt-dlp als eigene Liste versteht. */
const CHANNEL_TABS = new Set(['videos', 'shorts', 'streams', 'playlists', 'podcasts', 'releases', 'courses']);

/* Tracking-Parameter, die bei fremden Seiten wegfallen. */
const TRACKING = /^(utm_[a-z_]+|fbclid|gclid|dclid|gbraid|wbraid|msclkid|igsh|igshid|mc_cid|mc_eid|si|_ga|ref_src|ref_url)$/i;

/* ------------------------------------------------------------------ */
/* Säubern                                                              */
/* ------------------------------------------------------------------ */

/** Anführungszeichen, spitze Klammern und Satzzeichen am Rand entfernen. */
export function cleanInput(raw) {
  let s = String(raw == null ? '' : raw).trim();
  // <https://…>  "https://…"  'https://…'  `https://…`
  s = s.replace(/^[<"'`«„“‘]+/, '');
  // Satzzeichen und Anführungszeichen am Ende, und eine schließende Klammer ohne öffnende
  for (;;) {
    const before = s;
    s = s.replace(/[.,;:!?>"'`»“”’]+$/, '');
    for (const [open, close] of [['(', ')'], ['[', ']'], ['{', '}']]) {
      if (s.endsWith(close) && count(s, open) < count(s, close)) s = s.slice(0, -1);
    }
    if (s === before) break;
  }
  if (/^\/\//.test(s)) s = 'https:' + s;
  if (/^(?:www\.|m\.|music\.)?(?:youtube\.com|youtu\.be|youtube-nocookie\.com)\//i.test(s)) s = 'https://' + s;
  if (/^www\.[^\s/]+\.[a-z]{2,}(?:\/|$)/i.test(s)) s = 'https://' + s;
  return s;
}

function count(s, ch) {
  return s.split(ch).length - 1;
}

function toUrl(s, base) {
  try {
    const u = base ? new URL(s, base) : new URL(s);
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return null;
    if (!u.hostname) return null;
    return u;
  } catch (e) {
    return null;
  }
}

/** Weiterleitungen (Google, YouTube-Beschreibungen, Facebook) auspacken. */
function unwrap(u, depth = 0) {
  if (depth > 3) return u;
  const host = u.hostname.toLowerCase();
  const p = u.searchParams;
  let inner = null;
  if (/(^|\.)google\.[a-z.]+$/.test(host) && u.pathname === '/url') inner = p.get('q') || p.get('url');
  else if (isYouTubeHost(host) && u.pathname === '/redirect') inner = p.get('q');
  else if (isYouTubeHost(host) && u.pathname === '/attribution_link') inner = p.get('u') && toUrl(p.get('u'), YT)?.href;
  else if (/^l[m]?\.facebook\.com$/.test(host) && u.pathname === '/l.php') inner = p.get('u');
  else if (host === 'out.reddit.com') inner = p.get('url');
  if (!inner) return u;
  const next = toUrl(cleanInput(inner));
  return next ? unwrap(next, depth + 1) : u;
}

function isYouTubeHost(host) {
  return /(^|\.)youtube\.com$/.test(host) || /(^|\.)youtube-nocookie\.com$/.test(host);
}

/* ------------------------------------------------------------------ */
/* Erkennen                                                             */
/* ------------------------------------------------------------------ */

function result(kind, key, url, id = null) {
  return { ok: true, key, url, id, kind, cat: KIND_CAT[kind] };
}

function fail(reason) {
  return { ok: false, reason };
}

function video(id, kind = 'video') {
  return result(kind, 'yt:' + id, YT + '/watch?v=' + id, id);
}

function playlist(list) {
  return result('playlist', 'ytl:' + list, YT + '/playlist?list=' + list, list);
}

function parseYouTube(u) {
  const host = u.hostname.toLowerCase();
  const segs = u.pathname.split('/').filter(Boolean);
  const p = u.searchParams;

  if (host === 'youtu.be') {
    return YT_ID.test(segs[0] || '') ? video(segs[0]) : fail('notMedia');
  }

  const head = (segs[0] || '').toLowerCase();
  const second = segs[1] || '';

  switch (head) {
    case 'watch': {
      const v = p.get('v') || '';
      if (YT_ID.test(v)) return video(v);
      const list = p.get('list') || '';
      if (YT_LIST.test(list)) return playlist(list);
      return fail('notMedia');
    }
    case 'shorts':
      if (YT_ID.test(second)) return result('short', 'yt:' + second, YT + '/shorts/' + second, second);
      return fail('notMedia');
    case 'live':
      if (YT_ID.test(second)) return video(second, 'live');
      return fail('notMedia');
    case 'embed':
      if (second === 'videoseries') {
        const list = p.get('list') || '';
        return YT_LIST.test(list) ? playlist(list) : fail('notMedia');
      }
      return YT_ID.test(second) ? video(second) : fail('notMedia');
    case 'v':
    case 'e':
      return YT_ID.test(second) ? video(second) : fail('notMedia');
    case 'clip':
      if (/^[A-Za-z0-9_-]{8,}$/.test(second)) {
        return result('clip', 'ytclip:' + second, YT + '/clip/' + second, second);
      }
      return fail('notMedia');
    case 'playlist': {
      const list = p.get('list') || '';
      return YT_LIST.test(list) ? playlist(list) : fail('notMedia');
    }
    case 'results': {
      const q = (p.get('search_query') || p.get('q') || '').trim();
      if (!q) return fail('notMedia');
      return result('search', 'yts:' + q.toLowerCase(), YT + '/results?search_query=' + encodeURIComponent(q).replace(/%20/g, '+'));
    }
    case 'hashtag':
      if (!second) return fail('notMedia');
      return result('search', 'yth:' + second.toLowerCase(), YT + '/hashtag/' + encodeURIComponent(decodeSafe(second)));
    case 'channel':
      if (!/^UC[A-Za-z0-9_-]{22}$/.test(second)) return fail('notMedia');
      return channel('/channel/' + second, 'channel/' + second, segs[2]);
    case 'c':
    case 'user':
      if (!second) return fail('notMedia');
      return channel('/' + head + '/' + second, head + '/' + second.toLowerCase(), segs[2]);
    default:
      if (head.startsWith('@') && head.length > 1) {
        const handle = segs[0];
        return channel('/' + handle, handle.toLowerCase(), segs[1]);
      }
      return fail('notMedia');
  }
}

function channel(path, keyBase, tab) {
  const t = (tab || '').toLowerCase();
  const suffix = CHANNEL_TABS.has(t) ? '/' + t : '';
  return result('channel', 'ytc:' + keyBase + suffix, YT + path + suffix);
}

function decodeSafe(s) {
  try {
    return decodeURIComponent(s);
  } catch (e) {
    return s;
  }
}

/** Kurzvideo-Plattformen: landen bei den Shorts (wie in der readme: Reels, TikToks → Best). */
function isReel(host, segs) {
  if (/(^|\.)tiktok\.com$/.test(host)) {
    if (host === 'vm.tiktok.com' || host === 'vt.tiktok.com') return segs.length > 0;
    return segs[1] === 'video' || segs[0] === 't';
  }
  if (/(^|\.)instagram\.com$/.test(host)) return segs[0] === 'reel' || segs[0] === 'reels';
  if (/(^|\.)facebook\.com$/.test(host)) return segs[0] === 'reel';
  return false;
}

/** Host für den Schlüssel vereinheitlichen (www./m. weg, twitter → x). */
function keyHost(host) {
  let h = host.toLowerCase().replace(/^(www|m|mobile)\./, '');
  if (h === 'twitter.com') h = 'x.com';
  return h;
}

function parseOther(u) {
  const host = u.hostname.toLowerCase();
  const segs = u.pathname.split('/').filter(Boolean);
  const reel = isReel(host, segs);

  // Tracking raus - auf dem Rohtext, damit die Kodierung der Seite erhalten bleibt
  // (URLSearchParams würde z. B. %20 zu + machen). Kurzvideos brauchen gar keine Parameter.
  const kept = reel
    ? []
    : u.search
        .slice(1)
        .split('&')
        .filter((pair) => pair && !TRACKING.test(decodeSafe(pair.split('=')[0])));
  let path = u.pathname;
  if (path.length > 1) path = path.replace(/\/+$/, '');
  const query = kept.length ? '?' + kept.join('&') : '';
  const url = u.protocol + '//' + u.host + path + query;

  // Schlüssel: Parameter sortiert, damit ?a=1&b=2 und ?b=2&a=1 als gleich gelten
  const keyQuery = kept.length ? '?' + kept.slice().sort().join('&') : '';
  const key = 'url:' + keyHost(host) + (path === '/' ? '' : path) + keyQuery;
  return result(reel ? 'reel' : 'other', key, url);
}

/** Einen Link verstehen. Siehe Kopf der Datei für das Ergebnis. */
export function parseLink(raw) {
  const cleaned = cleanInput(raw);
  if (!cleaned) return fail('invalid');
  let u = toUrl(cleaned);
  if (!u) return fail('invalid');
  u = unwrap(u);
  const host = u.hostname.toLowerCase();
  if (host === 'youtu.be' || isYouTubeHost(host)) return parseYouTube(u);
  return parseOther(u);
}

/* ------------------------------------------------------------------ */
/* Text durchsuchen                                                     */
/* ------------------------------------------------------------------ */

const URL_IN_TEXT = /(?:https?:\/\/|www\.|(?:m\.|music\.)?youtube\.com\/|youtu\.be\/)[^\s<>"'`]+/gi;

/** Alle Links aus beliebigem Text (Einfügen, Markierung, Drag & Drop), ohne Doppelte. */
export function extractUrls(text) {
  const out = [];
  const seen = new Set();
  const str = String(text || '');
  let m;
  URL_IN_TEXT.lastIndex = 0;
  while ((m = URL_IN_TEXT.exec(str)) !== null) {
    // Markdown [text](url) - die Klammer gehört nicht dazu
    const s = cleanInput(m[0]);
    if (s && !seen.has(s)) {
      seen.add(s);
      out.push(s);
    }
  }
  return out;
}

/* ------------------------------------------------------------------ */
/* Hilfen für Anzeige und Titel                                         */
/* ------------------------------------------------------------------ */

const YT_VIDEO_KINDS = new Set(['video', 'short', 'live']);

/** Vorschaubild (nur YouTube-Videos/Shorts, ohne Berechtigung ladbar). */
export function thumbUrl(item) {
  if (item && YT_VIDEO_KINDS.has(item.kind) && item.id && YT_ID.test(item.id)) {
    return 'https://i.ytimg.com/vi/' + item.id + '/mqdefault.jpg';
  }
  return null;
}

/** oEmbed-Adresse für Titel und Kanal - Shorts werden in watch-Form gefragt. */
export function oembedUrl(item) {
  if (!item) return null;
  let target = null;
  if (YT_VIDEO_KINDS.has(item.kind) && item.id) target = YT + '/watch?v=' + item.id;
  else if (item.kind === 'playlist' && item.id) target = YT + '/playlist?list=' + item.id;
  if (!target) return null;
  return YT + '/oembed?format=json&url=' + encodeURIComponent(target);
}

export function isYouTube(item) {
  return !!item && /^(yt|ytl|ytc|yts|yth|ytclip):/.test(item.key || '');
}

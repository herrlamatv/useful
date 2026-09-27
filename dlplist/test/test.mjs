/* dlplist - by herrlamatv
 * Tests ohne Abhängigkeiten:
 *
 *   node test/test.mjs
 *
 * Link-Erkennung, Listen-Operationen, Befehls-Generator (inkl. Golden-Tests
 * gegen die eigenen .bat-Skripte in useful/yt-dlp/), Übersetzungen, Manifest.
 * Unter Windows zusätzlich ein echter Lauf: die erzeugte .bat/.ps1/.txt/.sh
 * ruft ein Fake-yt-dlp.exe auf, das seine Argumente protokolliert
 * (abschalten mit DLPLIST_E2E=0).
 */

import { readFileSync, readdirSync, existsSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { tmpdir } from 'node:os';
import { execFileSync, spawnSync } from 'node:child_process';

import { parseLink, extractUrls, cleanInput, thumbUrl, oembedUrl, CATS } from '../src/links.js';
import {
  addParsed,
  removeKeys,
  moveKeys,
  markExported,
  clearItems,
  applyTitles,
  needsTitle,
  counts,
  badgeText,
  mergeSettings,
  addHistory
} from '../src/store.js';
import {
  mergeExport,
  buildExport,
  planExport,
  categoryArgs,
  parsePath,
  renderPath,
  quote,
  splitArgs,
  cleanFolder,
  makeFilename,
  applyFormat,
  formatOf,
  presetMatches,
  linksOnly,
  asciiFold,
  BUILTIN_PRESETS,
  QUALITIES,
  TEMPLATES,
  LIMITS
} from '../src/command.js';
import enMessages from '../ui/i18n/en.js';
import deMessages from '../ui/i18n/de.js';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
process.chdir(ROOT);

let fails = 0;
const ok = (name, cond, extra) => {
  if (cond) console.log('  ok   ' + name);
  else {
    fails++;
    console.log('  FAIL ' + name + (extra !== undefined ? '  -> ' + extra : ''));
  }
};
const head = (title) => console.log('\n== ' + title + ' ==');

const translator = (bundle) => (key, vars) => {
  let text = bundle[key] ?? enMessages[key] ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) text = text.split('{' + k + '}').join(String(v));
  return text;
};
const tEn = translator(enMessages);
const tDe = translator(deMessages);
const NOW = new Date(2026, 8, 21, 19, 30);

const W = 'https://www.youtube.com/watch?v=dQw4w9WgXcQ';
const S = 'https://www.youtube.com/shorts/aqz-KE-bpKQ';

/** Liste aus Links bauen (wie der Service Worker). */
function listOf(...urls) {
  return addParsed([], urls.map(parseLink), 1000, 'test').items;
}

/* -------------------------------------------------------------------- */
head('Link-Erkennung');

const PL = 'https://www.youtube.com/playlist?list=PLabc123';
const LINK_CASES = [
  [W, 'videos', W],
  [W + '&list=PLx&index=3&pp=ygU&t=42s', 'videos', W],
  ['https://m.youtube.com/watch?feature=share&v=dQw4w9WgXcQ', 'videos', W],
  ['https://youtu.be/dQw4w9WgXcQ?si=abc&t=42', 'videos', W],
  ['youtu.be/dQw4w9WgXcQ', 'videos', W],
  ['https://music.youtube.com/watch?v=dQw4w9WgXcQ', 'videos', W],
  ['https://WWW.YOUTUBE.COM/watch?v=dQw4w9WgXcQ#t=30', 'videos', W],
  ['<' + W + '>.', 'videos', W],
  ['"' + W + '",', 'videos', W],
  [S + '?feature=share', 'shorts', S],
  [S + '/', 'shorts', S],
  ['https://www.youtube.com/live/aqz-KE-bpKQ?si=x', 'videos', 'https://www.youtube.com/watch?v=aqz-KE-bpKQ'],
  ['https://www.youtube.com/embed/dQw4w9WgXcQ?start=10', 'videos', W],
  ['https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ', 'videos', W],
  ['https://www.youtube.com/v/dQw4w9WgXcQ', 'videos', W],
  ['https://www.youtube.com/clip/UgkxAbCdEfGhIjKl', 'videos', 'https://www.youtube.com/clip/UgkxAbCdEfGhIjKl'],
  ['https://www.youtube.com/embed/videoseries?list=PLabc123', 'lists', PL],
  [PL + '&si=x', 'lists', PL],
  ['https://www.youtube.com/watch?list=PLabc123', 'lists', PL],
  ['https://www.youtube.com/@Handle', 'lists', 'https://www.youtube.com/@Handle'],
  ['https://www.youtube.com/@Handle/featured', 'lists', 'https://www.youtube.com/@Handle'],
  ['https://www.youtube.com/@Handle/shorts', 'lists', 'https://www.youtube.com/@Handle/shorts'],
  ['https://www.youtube.com/channel/UCuAXFkgsw1L7xaCfnd5JJOw/videos', 'lists', 'https://www.youtube.com/channel/UCuAXFkgsw1L7xaCfnd5JJOw/videos'],
  ['https://www.youtube.com/c/SomeName', 'lists', 'https://www.youtube.com/c/SomeName'],
  ['https://www.youtube.com/user/SomeName/videos', 'lists', 'https://www.youtube.com/user/SomeName/videos'],
  ['https://www.youtube.com/results?search_query=lofi+beats', 'lists', 'https://www.youtube.com/results?search_query=lofi+beats'],
  ['https://www.google.com/url?sa=t&url=https%3A%2F%2Fyoutu.be%2FdQw4w9WgXcQ', 'videos', W],
  ['https://www.youtube.com/redirect?event=x&q=https%3A%2F%2Fvimeo.com%2F123', 'other', 'https://vimeo.com/123'],
  ['https://www.youtube.com/attribution_link?u=/watch%3Fv%3DdQw4w9WgXcQ%26feature%3Dshare', 'videos', W],
  ['https://www.tiktok.com/@user/video/7300000000000000000?is_from_webapp=1', 'shorts', 'https://www.tiktok.com/@user/video/7300000000000000000'],
  ['https://vm.tiktok.com/ZMabc/', 'shorts', 'https://vm.tiktok.com/ZMabc'],
  ['https://www.instagram.com/reel/Cxyz123/?igsh=abc', 'shorts', 'https://www.instagram.com/reel/Cxyz123'],
  ['https://vimeo.com/123?utm_source=x&fbclid=y#t=10', 'other', 'https://vimeo.com/123'],
  ['https://vimeo.com/123?a=1&b=%20x', 'other', 'https://vimeo.com/123?a=1&b=%20x'],
  ['https://www.twitch.tv/videos/123456', 'other', 'https://www.twitch.tv/videos/123456']
];
for (const [input, cat, url] of LINK_CASES) {
  const p = parseLink(input);
  ok(input + ' → ' + cat, p.ok && p.cat === cat && p.url === url, JSON.stringify(p));
}

const NULL_CASES = [
  ['https://www.youtube.com/watch?v=tooShort', 'notMedia'],
  ['https://www.youtube.com/', 'notMedia'],
  ['https://www.youtube.com/feed/subscriptions', 'notMedia'],
  ['https://www.youtube.com/account', 'notMedia'],
  ['mailto:a@b.de', 'invalid'],
  ['javascript:alert(1)', 'invalid'],
  ['chrome://extensions', 'invalid'],
  ['', 'invalid'],
  ['kein link', 'invalid']
];
for (const [input, reason] of NULL_CASES) {
  const p = parseLink(input);
  ok('abgelehnt (' + reason + '): ' + (input || '(leer)'), !p.ok && p.reason === reason, JSON.stringify(p));
}

const key = (u) => parseLink(u).key;
ok('watch, youtu.be, shorts und embed teilen den Schlüssel', new Set([key(W), key('youtu.be/dQw4w9WgXcQ'), key('https://www.youtube.com/shorts/dQw4w9WgXcQ'), key('https://www.youtube.com/embed/dQw4w9WgXcQ')]).size === 1);
ok('twitter.com und x.com sind derselbe Link', key('https://twitter.com/a/status/1') === key('https://x.com/a/status/1'));
ok('Parameter-Reihenfolge egal', key('https://vimeo.com/1?a=1&b=2') === key('https://vimeo.com/1?b=2&a=1'));
ok('Kanal-Handle ohne Groß/klein', key('https://www.youtube.com/@Handle') === key('https://www.youtube.com/@handle'));
ok('Kanal-Tabs sind verschieden', key('https://www.youtube.com/@x/videos') !== key('https://www.youtube.com/@x/shorts'));

ok('cleanInput: Markdown-Klammer', cleanInput('https://vimeo.com/1)') === 'https://vimeo.com/1');
ok('cleanInput: Klammer im Link bleibt', cleanInput('https://en.wikipedia.org/wiki/Foo_(bar)') === 'https://en.wikipedia.org/wiki/Foo_(bar)');
const extracted = extractUrls('Schau mal: https://youtu.be/dQw4w9WgXcQ, und [hier](https://vimeo.com/123). Oder www.example.com/x! Doppelt: https://youtu.be/dQw4w9WgXcQ');
ok('extractUrls findet alles, ohne Doppelte', JSON.stringify(extracted) === JSON.stringify(['https://youtu.be/dQw4w9WgXcQ', 'https://vimeo.com/123', 'https://www.example.com/x']), JSON.stringify(extracted));
ok('extractUrls in leerem Text', extractUrls('').length === 0 && extractUrls(null).length === 0);

const [video] = listOf(W);
ok('Vorschaubild für Videos', thumbUrl(video) === 'https://i.ytimg.com/vi/dQw4w9WgXcQ/mqdefault.jpg');
ok('kein Vorschaubild für fremde Seiten', thumbUrl(listOf('https://vimeo.com/1')[0]) === null);
ok('oEmbed fragt Shorts in watch-Form', oembedUrl(listOf(S)[0]).includes(encodeURIComponent('https://www.youtube.com/watch?v=aqz-KE-bpKQ')));
ok('oEmbed auch für Playlists', oembedUrl(listOf(PL)[0]).includes(encodeURIComponent(PL)));
ok('kein oEmbed für Kanäle', oembedUrl(listOf('https://www.youtube.com/@x')[0]) === null);

/* -------------------------------------------------------------------- */
head('Liste');

{
  let r = addParsed([], [parseLink(W), parseLink(W), parseLink('youtu.be/dQw4w9WgXcQ')], 1, 'test');
  ok('Doppelte im selben Stapel zählen nicht als "schon drin"', r.items.length === 1 && r.added.length === 1 && r.dupes.length === 0);
  ok('nochmal hinzufügen → schon drin', addParsed(r.items, [parseLink(W)], 2, 'test').dupes.length === 1);

  r = addParsed(r.items, [parseLink('https://www.youtube.com/shorts/dQw4w9WgXcQ')], 2, 'test');
  ok('Video als /shorts/ nochmal → wandert zu den Shorts', r.moved.length === 1 && r.items[0].cat === 'shorts' && r.items[0].url === 'https://www.youtube.com/shorts/dQw4w9WgXcQ');

  let items = moveKeys(r.items, [r.items[0].key], 'videos');
  ok('Verschieben setzt manual', items[0].cat === 'videos' && items[0].manual === true);
  r = addParsed(items, [parseLink('https://www.youtube.com/shorts/dQw4w9WgXcQ')], 3, 'test');
  ok('von Hand verschoben → kein automatischer Umzug', r.moved.length === 0 && r.items[0].cat === 'videos');
  ok('unbekannte Kategorie wird ignoriert', moveKeys(items, [items[0].key], 'nope') === items);

  items = listOf(W, S, PL, 'https://vimeo.com/1');
  ok('Reihenfolge und Kategorien', items.map((i) => i.cat).join() === 'videos,shorts,lists,other');
  const c = counts(items);
  ok('Zählung', c.total === 4 && c.videos === 1 && c.shorts === 1 && c.lists === 1 && c.other === 1 && c.pending === 4);

  items = markExported(items, [items[0].key, items[1].key], 5);
  ok('markExported', counts(items).exported === 2 && counts(items).pending === 2);
  ok('clearItems exported', clearItems(items, 'exported').length === 2);
  ok('clearItems Kategorie', clearItems(items, 'lists').length === 3);
  ok('clearItems all', clearItems(items, 'all').length === 0);
  ok('removeKeys', removeKeys(items, [items[3].key]).length === 3);
  ok('markExported(0) = wieder offen', counts(markExported(items, [items[0].key], 0)).exported === 1);

  items = applyTitles(items, [{ key: items[0].key, title: 'Titel', author: 'Kanal' }, { key: items[1].key, status: 'unavailable' }]);
  ok('applyTitles', items[0].title === 'Titel' && items[0].author === 'Kanal' && items[0].titleTried && items[1].status === 'unavailable' && items[1].titleTried);
  ok('clearItems unavailable', clearItems(items, 'unavailable').length === 3);
  ok('needsTitle überspringt Versuchte', needsTitle(items, () => true).map((i) => i.cat).join() === 'lists,other');

  ok('Badge-Text', badgeText(0) === '' && badgeText(7) === '7' && badgeText(1200) === '1k+');
  ok('mergeSettings repariert Unsinn', mergeSettings({ enabled: 0, language: 'fr' }).enabled === false && mergeSettings({ language: 'fr' }).language === 'auto');
  let h = [];
  for (let i = 0; i < 14; i++) h = addHistory(h, { at: i });
  ok('Verlauf hält die letzten 10', h.length === 10 && h[0].at === 13);
}

/* -------------------------------------------------------------------- */
head('Pfade und Quoting');

ok('parsePath %USERPROFILE%', JSON.stringify(parsePath('%USERPROFILE%\\Desktop\\dw\\')) === JSON.stringify({ base: 'home', rest: '\\Desktop\\dw' }));
ok('parsePath ~/Videos', JSON.stringify(parsePath('~/Videos')) === JSON.stringify({ base: 'home', rest: '\\Videos' }));
ok('parsePath $env:APPDATA', JSON.stringify(parsePath('$env:APPDATA\\x')) === JSON.stringify({ base: 'env:APPDATA', rest: '\\x' }));
ok('parsePath C:\\ → C:\\.', parsePath('C:\\').rest === 'C:\\.' && parsePath('D:').rest === 'D:\\.');
ok('parsePath UNC bleibt', parsePath('\\\\server\\share\\').rest === '\\\\server\\share');
ok('parsePath leer', parsePath('  ') === null);
ok('parsePath entfernt "', parsePath('"C:\\x"').rest === 'C:\\x');

const home = parsePath('%USERPROFILE%\\Desktop\\dw');
ok('renderPath bat', renderPath(home, 'bat') === '"%USERPROFILE%\\Desktop\\dw"');
ok('renderPath cmd', renderPath(home, 'cmd') === '"%USERPROFILE%\\Desktop\\dw"');
ok('renderPath ps', renderPath(home, 'ps') === '"${env:USERPROFILE}\\Desktop\\dw"');
ok('renderPath sh', renderPath(home, 'sh') === '"${HOME}/Desktop/dw"');
ok('renderPath ps maskiert $', renderPath(parsePath('%USERPROFILE%\\a$b'), 'ps') === '"${env:USERPROFILE}\\a`$b"');
ok('renderPath bat verdoppelt % im Rest', renderPath(parsePath('D:\\100%'), 'bat') === '"D:\\100%%"');

const tricky = "a%b&c'd!e^f";
ok('quote bat', quote(tricky, 'bat') === "\"a%%b&c'd!e^f\"");
ok('quote cmd', quote(tricky, 'cmd') === "\"a%b&c'd!e^f\"");
ok('quote ps', quote(tricky, 'ps') === "'a%b&c''d!e^f'");
ok('quote sh', quote(tricky, 'sh') === "'a%b&c'\\''d!e^f'");

ok('splitArgs', JSON.stringify(splitArgs('--parse-metadata "title:%(artist)s" --no-part \'x y\' ""')) === JSON.stringify(['--parse-metadata', 'title:%(artist)s', '--no-part', 'x y', '']));
ok('cleanFolder', cleanFolder('../Kanäle//x/..\\y:*') === 'Kanäle\\x\\y');
ok('makeFilename', makeFilename('dlplist-{date}-{time}-{count}', NOW, 'bat', 3) === 'dlplist-2026-09-21-1930-3.bat');
ok('makeFilename säubert', makeFilename('a/b:c*', NOW, 'txt', 1) === 'a-b-c-.txt' && makeFilename('', NOW, 'txt', 1) === 'dlplist.txt');
ok('asciiFold', asciiFold('Größe – Müll … 🎉 café') === 'Groesse - Muell ... cafe');

/* -------------------------------------------------------------------- */
head('Einstellungen und Vorlagen');

{
  const bad = mergeExport({ cats: { videos: { quality: '999' } }, container: 'webm', output: { type: 'exe' }, extras: { fragments: 99 } });
  ok('mergeExport repariert Unsinn', bad.cats.videos.quality === '1080' && bad.container === 'mp4' && bad.output.type === 'bat' && bad.extras.fragments === 32);
  const def = mergeExport(null);
  ok('Standard: Videos 1080p, Shorts Best, .bat, Desktop\\dw', def.cats.videos.quality === '1080' && def.cats.shorts.quality === 'best' && def.output.type === 'bat' && def.env.path === '%USERPROFILE%\\Desktop\\dw');
  const scripts = BUILTIN_PRESETS.find((p) => p.id === 'scripts');
  ok('Standard entspricht der Vorlage "Deine Skripte"', presetMatches(def, scripts.format));
  const best = applyFormat(def, BUILTIN_PRESETS.find((p) => p.id === 'best').format);
  ok('Vorlage "Alles Best"', CATS.every((c) => best.cats[c].quality === 'best') && !presetMatches(best, scripts.format));
  const withPath = mergeExport({ env: { path: 'D:\\x' }, output: { type: 'ps1' }, cats: { other: { include: false } } });
  const applied = applyFormat(withPath, BUILTIN_PRESETS.find((p) => p.id === 'mp3').format);
  ok('Vorlagen lassen Pfad, Ausgabe und Einbeziehen in Ruhe', applied.env.path === 'D:\\x' && applied.output.type === 'ps1' && applied.cats.other.include === false && applied.cats.videos.quality === 'mp3');
  const own = formatOf(best);
  ok('eigene Vorlage: formatOf → applyFormat', presetMatches(applyFormat(def, own), own) && JSON.stringify(formatOf(applyFormat(def, own))) === JSON.stringify(own));
  for (const p of BUILTIN_PRESETS) ok('Vorlage "' + p.id + '" passt nach dem Anwenden', presetMatches(applyFormat(def, p.format), p.format));
}

/* -------------------------------------------------------------------- */
head('yt-dlp-Argumente');

const argStr = (cat, cfg) => categoryArgs(cat, cfg).map((x) => (typeof x === 'string' ? x : x.v ?? JSON.stringify(x.path))).join(' ');
for (const q of QUALITIES) ok('Qualität ' + q + ' erzeugt -f', argStr('videos', { cats: { videos: { quality: q } } }).startsWith('-f '));
{
  const mp3 = argStr('videos', { cats: { videos: { quality: 'mp3' } }, extras: { subs: true } });
  ok('MP3: -x, 320K, kein merge/recode, keine Untertitel', mp3.includes('-x --audio-format mp3 --audio-quality 320K') && !mp3.includes('merge-output') && !mp3.includes('recode') && !mp3.includes('embed-subs'));
  ok('recode aus → remux', argStr('videos', { recode: false }).includes('--remux-video mp4') && !argStr('videos', { recode: false }).includes('recode'));
  ok('mkv → remux mkv', argStr('videos', { container: 'mkv' }).includes('--merge-output-format mkv --remux-video mkv'));
  ok('original → kein Container-Flag', !argStr('videos', { container: 'original' }).includes('output-format'));
  ok('Kapitel nur ohne Metadaten', argStr('videos', { extras: { metadata: true, chapters: true } }).includes('--embed-metadata') && !argStr('videos', { extras: { metadata: true, chapters: true } }).includes('--embed-chapters'));
  ok('Kapitel allein', argStr('videos', { extras: { chapters: true } }).includes('--embed-chapters'));
  const lists = argStr('lists', { cats: { lists: { breakOnExisting: true, limit: 5, perList: true } } });
  ok('Playlists: Archiv, -I, break-per-input, Ordner pro Playlist', lists.includes('--download-archive dlplist-archive.txt') && lists.includes('-I 1:5') && lists.includes('--break-on-existing --break-per-input') && lists.includes('%(playlist_title,playlist_id)s/'));
  ok('break-on-existing nur bei Playlists', !argStr('videos', { cats: { lists: { breakOnExisting: true } } }).includes('break-on'));
  ok('Tempolimit geprüft', argStr('videos', { extras: { rate: '5m' } }).includes(' -r 5M') && !argStr('videos', { extras: { rate: 'schnell' } }).includes(' -r '));
  ok('SponsorBlock', argStr('videos', { extras: { sponsorblock: 'remove' } }).includes('--sponsorblock-remove default') && argStr('videos', { extras: { sponsorblock: 'mark' } }).includes('--sponsorblock-mark all'));
  ok('Untertitel', argStr('videos', { extras: { subs: true, subLangs: 'de, en;rm', autoSubs: true } }).includes('--embed-subs --sub-langs de,enrm --write-auto-subs'));
  ok('JS-Runtime', argStr('videos', { env: { jsRuntime: 'node' } }).includes('--no-js-runtimes --js-runtimes node'));
  ok('Anti-Block', argStr('videos', { extras: { sleep: true } }).includes('--sleep-requests 0.75 --sleep-interval 10 --max-sleep-interval 20'));
  ok('Unterordner als -P', argStr('shorts', { cats: { shorts: { folder: 'Shorts' } } }).includes('-P ' + JSON.stringify({ base: null, rest: 'Shorts' })));
  ok('Kompatibel', argStr('videos', { compat: true }).includes('-S vcodec:h264,lang,quality,res,fps,hdr:12,acodec:aac'));

  const warns = planExport(listOf(W, S), { compat: true, cats: { videos: { quality: '2160' }, shorts: { quality: 'm4a' } }, extras: { subs: true, thumbnail: true, rate: 'x' }, container: 'original', cookies: 'browser', cookieBrowser: 'chrome' }).warnings.map((w) => w.code);
  for (const code of ['compatHighRes', 'audioNoSubs', 'thumbOriginal', 'badRate', 'chromiumCookies']) ok('Warnung ' + code, warns.includes(code), warns.join());
  const none = planExport([], null).warnings.map((w) => w.code);
  ok('Warnung noItems', none.includes('noItems'));
  const skipped = planExport(markExported(listOf(W), [key(W)], 1), null);
  ok('Nur neue: Exportierte werden übersprungen', skipped.total === 0 && skipped.skipped === 1 && skipped.warnings.some((w) => w.code === 'noItemsSkipped'));
  ok('Nur neue aus: alle', planExport(markExported(listOf(W), [key(W)], 1), { onlyNew: false }).total === 1);
  ok('ausgeschaltete Kategorie fehlt', planExport(listOf(W, S), { cats: { shorts: { include: false } } }).groups.map((g) => g.cat).join() === 'videos');
}

/* -------------------------------------------------------------------- */
head('Golden-Tests gegen useful/yt-dlp/*.bat');

function ytdlpLine(file, url) {
  const path = join(ROOT, '..', 'yt-dlp', file);
  if (!existsSync(path)) return null;
  const line = readFileSync(path, 'utf8').split(/\r?\n/).find((l) => l.startsWith('yt-dlp '));
  return line ? line.replace('"%URL%"', '"' + url + '"') : null;
}
function ourLine(items, cfg) {
  const res = buildExport(items, mergeExport(Object.assign({ bat: { consoleTitle: false } }, cfg)), { t: tEn, now: NOW });
  return res.text.split('\r\n').find((l) => l.startsWith('yt-dlp -f'));
}
{
  const defs = mergeExport(null);
  const golden = [
    ['dw-1080p.bat', W, listOf(W), {}],
    ['dw-best.bat', S, listOf(S), {}],
    ['Cook-dw-1080p.bat', W, listOf(W), applyFormat(defs, BUILTIN_PRESETS.find((p) => p.id === 'cookie').format)],
    ['dw-best.bat', W, listOf(W), applyFormat(defs, BUILTIN_PRESETS.find((p) => p.id === 'best').format)]
  ];
  for (const [file, url, items, cfg] of golden) {
    const expected = ytdlpLine(file.startsWith('Cook') ? 'cookie/' + file : file, url);
    if (expected === null) {
      console.log('  skip ' + file + ' (nicht gefunden)');
      continue;
    }
    const got = ourLine(items, Object.assign({}, cfg, { bat: { consoleTitle: false } }));
    ok('Byte für Byte wie ' + file, got === expected, '\n       erwartet: ' + expected + '\n       bekommen: ' + got);
  }
}

/* -------------------------------------------------------------------- */
head('Dateien');

{
  const items = listOf(W, "https://example.com/it's/a!b^c?x=1&y=%20z", S);
  items[0].title = 'Rick Astley – Größe & 100% <live>';
  const build = (output, extra = {}) => buildExport(items, mergeExport(Object.assign({ output }, extra)), { t: tDe, now: NOW });

  const bat = build({ type: 'bat' });
  const batLines = bat.text.split('\r\n');
  ok('.bat: CRLF, kein BOM, octet-stream', bat.eol === '\r\n' && !bat.bom && bat.mime === 'application/octet-stream' && !/[^\r]\n/.test(bat.text));
  ok('.bat: rein ASCII (auch mit deutschen Texten)', /^[\x00-\x7f]*$/.test(bat.text), bat.text.match(/[^\x00-\x7f]/)?.[0]);
  ok('.bat: kein enabledelayedexpansion, kein call yt-dlp, keine for-Schleife über Links', !/enabledelayedexpansion/i.test(bat.text) && !/call\s+yt-dlp/i.test(bat.text) && !/for\s+%%\w+\s+in\s*\(/i.test(bat.text));
  ok('.bat: Vorlage mit %%', bat.text.includes('-o "%%(title)s - %%(uploader)s [%%(id)s].%%(ext)s"'));
  ok('.bat: % in Links verdoppelt', bat.text.includes('y=%%20z'));
  ok('.bat: Titel im Kommentar ohne riskante Zeichen', batLines.some((l) => l.startsWith(':: [Videos] Rick Astley - Groesse  100 live') || l.startsWith(':: [Videos] Rick Astley - Groesse 100 live')), batLines.find((l) => l.includes('Rick')));
  ok('.bat: fester Pfad mit pushd und mkdir', bat.text.includes('set "SAVE_DIR=%USERPROFILE%\\Desktop\\dw"') && bat.text.includes('pushd "%SAVE_DIR%" || goto :nodir') && bat.text.includes('mkdir "%SAVE_DIR%"'));
  ok('.bat: echo-Texte maskiert', batLines.filter((l) => l.startsWith('echo ')).every((l) => !/(^|[^^])[&|<>]/.test(l.slice(5).replace(/"[^"]*"/g, ''))));
  ok('.bat: Fehlerzähler', (bat.text.match(/if errorlevel 1 set \/a FAILS\+=1/g) || []).length === 3);

  const cmd = build({ type: 'txt', shell: 'cmd' });
  ok('.txt cmd: einfache %, cd /d', cmd.text.includes('-o "%(title)s') && !cmd.text.includes('%%') && cmd.text.includes('cd /d "%USERPROFILE%\\Desktop\\dw"'));
  ok('.txt cmd: rem-Kommentare', cmd.text.startsWith('rem '));

  const ps1 = build({ type: 'ps1' });
  ok('.ps1: BOM, CRLF', ps1.bom && ps1.eol === '\r\n');
  ok(".ps1: ' verdoppelt, Pfad mit $env", ps1.text.includes("'https://example.com/it''s/a!b%5Ec?x=1&y=%20z'") && ps1.text.includes('$SaveDir = "${env:USERPROFILE}\\Desktop\\dw"'));
  ok('.ps1: sucht yt-dlp.exe im Ordner', ps1.text.includes("Test-Path -LiteralPath '.\\yt-dlp.exe'") && ps1.text.includes('& $ytdlp '));

  const sh = build({ type: 'sh' });
  ok('.sh: LF, Shebang, $HOME', sh.eol === '\n' && sh.text.startsWith('#!/usr/bin/env bash\n') && sh.text.includes('SAVE_DIR="${HOME}/Desktop/dw"'));
  ok(".sh: ' maskiert", sh.text.includes("'https://example.com/it'\\''s/a!b%5Ec?x=1&y=%20z'"));

  const links = build({ type: 'links' });
  ok('Linkliste: Kommentare mit #, jeder Link einmal', links.text.split('\r\n').filter((l) => l.startsWith('http')).length === 3 && links.filename.endsWith('-links.txt'));
  ok('linksOnly', linksOnly(items, null).split('\n').length === 3);

  const here = build({ type: 'bat' }, { env: { location: 'here' } });
  ok('.bat Ordner der Datei: %~dp0, kein mkdir', here.text.includes('set "SAVE_DIR=%~dp0"') && !here.text.includes('mkdir') && here.warnings.some((w) => w.code === 'hereFile'));
  const hereTxt = build({ type: 'txt', shell: 'cmd' }, { env: { location: 'here' } });
  ok('.txt Ordner der Datei: kein cd', !hereTxt.text.includes('cd /d'));
  const exe = build({ type: 'bat' }, { env: { exe: 'C:\\tools\\yt-dlp.exe' } });
  ok('.bat eigenes yt-dlp: gequotet und geprüft', exe.text.includes('if not exist "C:\\tools\\yt-dlp.exe" goto :noexe') && exe.text.includes('"C:\\tools\\yt-dlp.exe" -f '));
  const umlaut = build({ type: 'bat' }, { env: { path: 'D:\\Vidéos' } });
  ok('.bat mit Umlaut im Pfad: chcp 65001 + Hinweis', umlaut.text.split('\r\n')[1] === 'chcp 65001 >nul' && umlaut.warnings.some((w) => w.code === 'utf8'));
  const each = build({ type: 'bat', mode: 'each' });
  ok('.bat ein Aufruf pro Link', each.text.includes('call :dl_videos') && each.text.includes('set "U=https://example.com/it\'s/a!b%%5Ec?x=1&y=%%20z"') && each.text.includes('"%U%"'));
  const eachPs = build({ type: 'ps1', mode: 'each' });
  ok('.ps1 ein Aufruf pro Link', eachPs.text.includes('foreach ($u in $urls) {'));
  const opts = build({ type: 'bat' }, { bat: { update: true, openFolder: true, shutdown: true, pause: false } });
  ok('.bat Optionen: -U, Ordner öffnen, herunterfahren, keine Pause', opts.text.includes('yt-dlp -U') && opts.text.includes('start "" "%CD%"') && opts.text.includes('shutdown /s /t 120') && !opts.text.includes('\r\npause\r\nexit /b 0'));
  const extra = build({ type: 'bat' }, { extras: { extraArgs: '--parse-metadata "title:%(artist)s" --no-part' } });
  ok('Zusatzargumente je Ziel neu gequotet', extra.text.includes('--parse-metadata "title:%%(artist)s" --no-part'));
  ok('Zusatzargumente: " ist kein Grund zur Warnung', !extra.warnings.some((w) => w.code === 'sanitized'));

  // 400 Links: keine Zeile zu lang, jeder Link genau einmal, Reihenfolge bleibt
  const many = [];
  for (let i = 0; i < 400; i++) many.push('https://www.youtube.com/watch?v=' + ('v' + i).padEnd(11, 'x'));
  const bigItems = listOf(...many);
  for (const [type, shell, limit] of [['bat', 'cmd', 8191], ['txt', 'cmd', 8191], ['txt', 'ps', 32767], ['ps1', 'cmd', 32767]]) {
    const r = buildExport(bigItems, mergeExport({ output: { type, shell } }), { t: tEn, now: NOW });
    const lines = r.text.split(/\r?\n/);
    // nur Befehlszeilen - die Titelliste im Kopf nennt jeden Link ein zweites Mal
    const body = lines.filter((l) => !/^\s*(::|#|rem )/.test(l)).join('\n');
    const found = many.map((u) => body.indexOf(u));
    ok(type + (type === 'txt' ? '/' + shell : '') + ': 400 Links aufgeteilt, alle da, Reihenfolge stimmt',
      lines.every((l) => l.length <= limit) && found.every((pos, i) => pos > 0 && (i === 0 || pos > found[i - 1])) && many.every((u) => body.split(u).length === 2),
      Math.max(...lines.map((l) => l.length)));
  }
  ok('bat: mehrere Aufrufe bei 400 Links', (buildExport(bigItems, mergeExport(null), { t: tEn, now: NOW }).text.match(/^yt-dlp -f/gm) || []).length >= 3);
  ok('Grenzen', LIMITS.bat < 8191 && LIMITS.ps < 32767);
}

/* -------------------------------------------------------------------- */
head('Übersetzungen');

const enKeys = Object.keys(enMessages).sort();
const deKeys = Object.keys(deMessages).sort();
const onlyEn = enKeys.filter((k) => !(k in deMessages));
const onlyDe = deKeys.filter((k) => !(k in enMessages));
ok('beide Sprachen haben dieselben Schlüssel', onlyEn.length === 0 && onlyDe.length === 0, 'nur en: ' + onlyEn.join(', ') + ' | nur de: ' + onlyDe.join(', '));
ok('keine leeren Texte', [...enKeys, ...deKeys].every((k) => {
  const bundle = k in enMessages ? enMessages : deMessages;
  return typeof bundle[k] === 'string' && bundle[k].trim().length > 0;
}));
const holes = (text) => (String(text).match(/\{[a-zA-Z]+\}/g) || []).sort().join(',');
const mismatched = enKeys.filter((k) => k in deMessages && holes(enMessages[k]) !== holes(deMessages[k]));
ok('Platzhalter stimmen überein', mismatched.length === 0, mismatched.join(', '));

/* Jeder im Code benutzte Schlüssel muss es geben - auch im Service Worker (Menütitel). */
const used = new Set();
const KEY = /^[a-z][A-Za-z]*(?:\.[A-Za-z]+)+$/;
for (const dir of ['ui', 'src']) {
  for (const file of readdirSync(dir)) {
    if (!/[.](js|html)$/.test(file)) continue;
    const text = readFileSync(dir + '/' + file, 'utf8');
    for (const re of [/\bt\(\s*'([^']+)'/g, /\btr\(\s*'([^']+)'/g, /data-i18n(?:-ph|-title)?="([^"]+)"/g, /\bwarn\(\s*'([a-zA-Z]+)'/g]) {
      let m;
      while ((m = re.exec(text)) !== null) {
        const k = re.source.includes('warn') ? 'warn.' + m[1] : m[1];
        if (KEY.test(k)) used.add(k);
      }
    }
  }
}
// zusammengesetzte Schlüssel
for (const c of CATS) used.add('cat.' + c);
for (const p of BUILTIN_PRESETS) used.add('preset.' + p.id).add('preset.' + p.id + 'Desc');
for (const k of Object.keys(TEMPLATES).concat('custom')) used.add('template.' + k);
for (const k of ['pause', 'check', 'update', 'consoleTitle', 'openFolder', 'shutdown']) used.add('bat.' + k).add('bat.' + k + 'Desc');
for (const k of ['path', 'exe', 'saveTo', 'ffmpeg', 'template', 'extraArgs']) used.add('field.' + k);
for (const k of ['noItemsSkipped', 'noItems', 'hereTxt', 'hereFile']) used.add('warn.' + k);
const unknown = [...used].filter((k) => !(k in enMessages)).sort();
ok('alle benutzten Schlüssel sind übersetzt (' + used.size + ' Stück)', unknown.length === 0, unknown.join(', '));
const dynamic = [
  ...QUALITIES.map((q) => 'quality.' + q),
  ...['txt', 'bat', 'ps1', 'sh', 'links'].map((x) => 'output.hint_' + x)
].filter((k) => !(k in enMessages));
ok('zusammengesetzte Schlüssel mit Ziffern/Unterstrich', dynamic.length === 0, dynamic.join(', '));

/* -------------------------------------------------------------------- */
head('Manifest und Dateien');

const manifest = JSON.parse(readFileSync('manifest.json', 'utf8'));
ok('Manifest V3, default_locale en', manifest.manifest_version === 3 && manifest.default_locale === 'en');
ok('Berechtigungen genau diese (kein tabs, kein <all_urls>)',
  JSON.stringify(manifest.permissions.slice().sort()) === JSON.stringify(['activeTab', 'contextMenus', 'downloads', 'scripting', 'storage']) &&
  JSON.stringify(manifest.host_permissions) === JSON.stringify(['https://www.youtube.com/*']));
const referenced = [manifest.background.service_worker, manifest.action.default_popup, manifest.options_ui.page, ...Object.values(manifest.icons), ...Object.values(manifest.action.default_icon)];
ok('alle Dateien aus dem Manifest existieren', referenced.every((f) => existsSync(f)), referenced.filter((f) => !existsSync(f)).join(', '));
ok('Tastenkürzel Alt+Shift+D', manifest.commands['add-current-tab'].suggested_key.default === 'Alt+Shift+D');
const localeKeys = [];
for (const lang of ['en', 'de']) {
  const msgs = JSON.parse(readFileSync('_locales/' + lang + '/messages.json', 'utf8'));
  localeKeys.push(Object.keys(msgs).sort().join(','));
  ok('_locales/' + lang + ' hat alle Manifest-Texte', ['extName', 'extDescription', 'actionTitle', 'cmdAddTab'].every((k) => msgs[k] && msgs[k].message));
}
ok('_locales in beiden Sprachen deckungsgleich', localeKeys[0] === localeKeys[1]);
for (const [field, value] of [['name', manifest.name], ['description', manifest.description], ['command', manifest.commands['add-current-tab'].description]]) {
  ok('manifest.' + field + ' verweist auf _locales', /^__MSG_\w+__$/.test(value), value);
}

const sources = ['src', 'ui'].flatMap((d) => readdirSync(d).filter((f) => /\.(js|css)$/.test(f)).map((f) => d + '/' + f)).concat(['ui/i18n/de.js', 'ui/i18n/en.js', 'tools/icons.mjs', 'test/test.mjs']);
ok('jede JS/CSS-Datei beginnt mit dem Kopf', sources.every((f) => readFileSync(f, 'utf8').startsWith('/* dlplist - by herrlamatv')), sources.filter((f) => !readFileSync(f, 'utf8').startsWith('/* dlplist - by herrlamatv')).join(', '));

/* Named Imports prüfen: jeder importierte Name muss exportiert werden (fängt Tippfehler in der UI). */
{
  const problems = [];
  for (const file of sources.filter((f) => f.endsWith('.js'))) {
    const text = readFileSync(file, 'utf8');
    const re = /import\s*\{([^}]+)\}\s*from\s*'([^']+)'/g;
    let m;
    while ((m = re.exec(text)) !== null) {
      const target = resolve(dirname(file), m[2]);
      const mod = readFileSync(target, 'utf8');
      for (const raw of m[1].split(',')) {
        const name = raw.trim().split(/\s+as\s+/)[0];
        if (!name) continue;
        const exported = new RegExp('export\\s+(?:async\\s+)?(?:function|const|let|class)\\s+' + name + '\\b').test(mod);
        if (!exported) problems.push(file + ': ' + name + ' aus ' + m[2]);
      }
    }
  }
  ok('alle importierten Namen existieren', problems.length === 0, problems.join('; '));
}

/* Service Worker einmal laden (mit Attrappe für chrome.*) - prüft Syntax und Importe. */
{
  const noop = () => {};
  const listener = { addListener: noop };
  const fake = new Proxy(
    {},
    {
      get: (_t, prop) => (prop === 'then' ? undefined : new Proxy(noop, { get: (_f, p) => (p === 'addListener' ? noop : listener), apply: () => Promise.resolve() }))
    }
  );
  globalThis.chrome = fake;
  try {
    await import(pathToFileURL(join(ROOT, 'src/background.js')).href);
    ok('Service Worker lädt ohne Fehler', true);
  } catch (e) {
    ok('Service Worker lädt ohne Fehler', false, e.message);
  }
  try {
    await import(pathToFileURL(join(ROOT, 'ui/export-settings.js')).href);
    ok('Einstellungs-Panels laden ohne Fehler', true);
  } catch (e) {
    ok('Einstellungs-Panels laden ohne Fehler', false, e.message);
  }
  delete globalThis.chrome;
}

/* -------------------------------------------------------------------- */
/* Echter Lauf unter Windows: Fake-yt-dlp.exe protokolliert seine argv.  */
/* -------------------------------------------------------------------- */

const CSC = 'C:\\Windows\\Microsoft.NET\\Framework64\\v4.0.30319\\csc.exe';
if (process.platform === 'win32' && process.env.DLPLIST_E2E !== '0' && existsSync(CSC)) {
  head('Echter Lauf (cmd.exe, PowerShell, bash) mit Fake-yt-dlp.exe');

  const dir = join(tmpdir(), 'dlplist-e2e-' + process.pid);
  mkdirSync(dir, { recursive: true });
  const src = join(dir, 'fake.cs');
  writeFileSync(
    src,
    [
      'using System; using System.IO; using System.Text;',
      'class P { static int Main(string[] a) {',
      '  var sb = new StringBuilder("[");',
      '  for (int i = 0; i < a.Length; i++) {',
      '    if (i > 0) sb.Append(",");',
      '    sb.Append("\\"");',
      '    foreach (char c in a[i]) {',
      '      if (c == \'"\' || c == \'\\\\\') { sb.Append(\'\\\\\'); sb.Append(c); }',
      '      else if (c < 32) sb.AppendFormat("\\\\u{0:x4}", (int)c);',
      '      else sb.Append(c);',
      '    }',
      '    sb.Append("\\"");',
      '  }',
      '  sb.Append("]");',
      '  File.AppendAllText(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "argv.jsonl"), sb.ToString() + "\\n", new UTF8Encoding(false));',
      '  return 0; } }'
    ].join('\r\n')
  );
  let compiled = false;
  try {
    execFileSync(CSC, ['/nologo', '/out:' + join(dir, 'yt-dlp.exe'), src], { stdio: 'pipe' });
    compiled = existsSync(join(dir, 'yt-dlp.exe'));
  } catch (e) {
    console.log('  skip (csc fehlgeschlagen: ' + String(e.message).split('\n')[0] + ')');
  }

  if (compiled) {
    const items = listOf(W, "https://example.com/it's/a!b^c?x=1&y=%20z&q=%25USERPROFILE%25", S, 'https://vimeo.com/1?a=b');
    const cfg = mergeExport({
      env: { location: 'here', saveTo: '%USERPROFILE%\\dlplist test' },
      cats: { shorts: { quality: 'mp3', folder: 'Shorts & Co' } },
      extras: { extraArgs: '--parse-metadata "title:%(artist)s!" --no-part', subs: true },
      bat: { pause: false, check: true }
    });
    const plan = planExport(items, cfg);
    const raw = (tok, target) => {
      if (typeof tok === 'string') return tok;
      if (!tok.path) return tok.v;
      const p = tok.path;
      const envVal = p.base ? process.env[p.base === 'home' ? 'USERPROFILE' : p.base.slice(4)] : '';
      return envVal + p.rest;
    };
    const expected = (target, withTitle) =>
      plan.groups.map((g) => g.args.map((x) => raw(x, target)).concat(withTitle ? ['--console-title'] : [], g.urls));
    const readArgv = () => {
      const f = join(dir, 'argv.jsonl');
      if (!existsSync(f)) return [];
      const lines = readFileSync(f, 'utf8').trim().split('\n').filter(Boolean).map((l) => JSON.parse(l));
      rmSync(f);
      return lines;
    };
    const compare = (name, got, want) => {
      const same = JSON.stringify(got) === JSON.stringify(want);
      let detail = '';
      if (!same) {
        const gi = got.findIndex((g, i) => JSON.stringify(g) !== JSON.stringify(want[i]));
        detail = '\n       erwartet: ' + JSON.stringify(want[gi] || want) + '\n       bekommen: ' + JSON.stringify(got[gi] || got);
      }
      ok(name, same, detail);
    };

    // .bat
    const bat = buildExport(items, mergeExport(Object.assign({}, cfg, { output: { type: 'bat' } })), { t: tDe, now: NOW });
    writeFileSync(join(dir, 'run.bat'), bat.text);
    const rb = spawnSync('cmd.exe', ['/d', '/c', join(dir, 'run.bat')], { cwd: tmpdir(), encoding: 'utf8' });
    compare('.bat → cmd.exe übergibt jedes Argument exakt', readArgv(), expected('bat', true));
    ok('.bat meldet Fertig', /Fertig\./.test(rb.stdout), rb.stdout + rb.stderr);

    // .bat, ein Aufruf pro Link
    const each = buildExport(items, mergeExport(Object.assign({}, cfg, { output: { type: 'bat', mode: 'each' } })), { t: tEn, now: NOW });
    writeFileSync(join(dir, 'each.bat'), each.text);
    spawnSync('cmd.exe', ['/d', '/c', join(dir, 'each.bat')], { cwd: tmpdir(), encoding: 'utf8' });
    const perLink = plan.groups.flatMap((g) => g.urls.map((u) => g.args.map((x) => raw(x, 'bat')).concat(['--console-title', u])));
    compare('.bat ein Aufruf pro Link → exakt', readArgv(), perLink);

    // .txt für cmd: wie Einfügen in eine offene Eingabeaufforderung
    const txt = buildExport(items, mergeExport(Object.assign({}, cfg, { output: { type: 'txt', shell: 'cmd' } })), { t: tEn, now: NOW });
    // wie eine normale Eingabeaufforderung: ohne NoDefaultCurrentDirectoryInExePath
    const plainEnv = Object.assign({}, process.env);
    delete plainEnv.NoDefaultCurrentDirectoryInExePath;
    spawnSync('cmd.exe', ['/d', '/q', '/k'], { cwd: dir, input: txt.text + 'exit\r\n', encoding: 'utf8', env: plainEnv });
    compare('.txt (cmd, eingefügt) → exakt', readArgv(), expected('cmd', false));

    // .ps1 mit Windows PowerShell 5.1
    const ps1 = buildExport(items, mergeExport(Object.assign({}, cfg, { output: { type: 'ps1' } })), { t: tDe, now: NOW });
    writeFileSync(join(dir, 'run.ps1'), '\uFEFF' + ps1.text);
    spawnSync('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', join(dir, 'run.ps1')], { cwd: tmpdir(), encoding: 'utf8' });
    compare('.ps1 → PowerShell 5.1 übergibt jedes Argument exakt', readArgv(), expected('ps', true));

    // .sh mit Git Bash (ohne Speicherort, weil MSYS Pfade umschreibt)
    const bash = spawnSync('bash', ['--version'], { encoding: 'utf8' });
    if (bash.status === 0) {
      const shCfg = mergeExport(Object.assign({}, cfg, { env: { location: 'here', saveTo: '' }, output: { type: 'sh' } }));
      const sh = buildExport(items, shCfg, { t: tEn, now: NOW });
      writeFileSync(join(dir, 'run.sh'), sh.text);
      spawnSync('bash', [join(dir, 'run.sh').replace(/\\/g, '/')], { cwd: tmpdir(), encoding: 'utf8', env: Object.assign({}, process.env, { MSYS_NO_PATHCONV: '1', MSYS2_ARG_CONV_EXCL: '*' }) });
      const shPlan = planExport(items, shCfg);
      compare('.sh → bash übergibt jedes Argument exakt', readArgv(), shPlan.groups.map((g) => g.args.map((x) => raw(x, 'sh')).concat(g.urls)));
    } else {
      console.log('  skip .sh (kein bash)');
    }
  }
  rmSync(dir, { recursive: true, force: true });
}

console.log('\n' + (fails === 0 ? 'Alle Tests bestanden.' : fails + ' Test(s) fehlgeschlagen.'));
process.exit(fails === 0 ? 0 : 1);

/* QR-ext - by herrlamatv
 * Tests ohne Abhängigkeiten:
 *
 * Encoder gegen Referenzwerte der Norm, Rücklesen jeder Version aus der
 * Matrix, Reed-Solomon-Syndrome, Vorlagen, Renderer, Speicher, Übersetzungen,
 * Manifest. Wenn Python mit OpenCV da ist, zusätzlich ein echter Scan:
 * die Codes werden als PNG gerastert und von cv2.QRCodeDetector gelesen
 * (abschalten mit QREXT_E2E=0).
 */

import { readFileSync, existsSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { deflateSync } from 'node:zlib';

import {
  encode,
  buildData,
  addEcc,
  dataCodewords,
  rawDataModules,
  alignmentPositions,
  formatBits,
  versionBits,
  functionPatterns,
  dataPositions,
  pickMode,
  rsDivisor,
  gfMul,
  MASKS,
  ECC_LEVELS
} from '../src/qr.js';
import {
  TYPES,
  buildPayload,
  escapeWifi,
  escapeVcard,
  cleanPhone,
  summary,
  fileSlug,
  emptyFields
} from '../src/payload.js';
import { shapes, toSvg, drawToContext, colorWarning, contrast } from '../src/render.js';
import {
  mergeSettings,
  addHistory,
  removeHistory,
  cleanHistory,
  cleanFields,
  takePending,
  DEFAULT_SETTINGS,
  HISTORY_MAX
} from '../src/store.js';
import { MENUS, textFor } from '../src/menu.js';
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
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

/* -------------------------------------------------------------------- */
head('Encoder: Referenzwerte');

{
  // Beispiel aus der Norm / thonky.com: "HELLO WORLD", Version 1-Q
  const d = buildData('HELLO WORLD', 'Q');
  ok('HELLO WORLD -> Version 1, alphanumerisch', d.version === 1 && d.mode === 'alnum');
  ok('HELLO WORLD Datencodewörter', same(d.data, [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236]), d.data.join(' '));
  const all = addEcc(d.data, 1, 'Q');
  ok('HELLO WORLD Fehlerkorrektur', same(all.slice(13), [168, 72, 22, 82, 217, 54, 156, 0, 46, 15, 180, 122, 16]), all.slice(13).join(' '));
}

ok('Kapazität Version 1: L/M/Q/H = 19/16/13/9', same(ECC_LEVELS.map((e) => dataCodewords(1, e)), [19, 16, 13, 9]));
ok('Kapazität Version 40: L/M/Q/H = 2956/2334/1666/1276', same(ECC_LEVELS.map((e) => dataCodewords(40, e)), [2956, 2334, 1666, 1276]));
ok('Rohmodule Version 1 = 208, Version 40 = 29648', rawDataModules(1) === 208 && rawDataModules(40) === 29648);

ok('Ausrichtung Version 1: keine', same(alignmentPositions(1), []));
ok('Ausrichtung Version 2: 6, 18', same(alignmentPositions(2), [6, 18]));
ok('Ausrichtung Version 7: 6, 22, 38', same(alignmentPositions(7), [6, 22, 38]));
ok('Ausrichtung Version 32: 6 … 138', same(alignmentPositions(32), [6, 34, 60, 86, 112, 138]));
ok('Ausrichtung Version 40: 6 … 170', same(alignmentPositions(40), [6, 30, 58, 86, 114, 142, 170]));

ok('Formatbits L / Maske 4 = 110011000101111', formatBits('L', 4) === 0b110011000101111);
ok('Formatbits H / Maske 7 = 000100000111011', formatBits('H', 7) === 0b000100000111011);
ok('Formatbits M / Maske 0 = 101010000010010', formatBits('M', 0) === 0b101010000010010);
ok('Versionsbits Version 7 = 000111110010010100', versionBits(7) === 0b000111110010010100);
ok('Versionsbits Version 40 = 101000110001101001', versionBits(40) === 0b101000110001101001);

ok('Modus: nur Ziffern -> numeric', pickMode('0123456789') === 'numeric');
ok('Modus: GROSS + Zeichen -> alnum', pickMode('HTTPS://EXAMPLE.COM/A-B') === 'alnum');
ok('Modus: klein -> byte', pickMode('https://example.com') === 'byte');
ok('Modus: Umlaute -> byte', pickMode('Größe') === 'byte');

ok('17 Bytes passen in 1-L, 18 nicht', buildData('a'.repeat(17), 'L').version === 1 && buildData('a'.repeat(18), 'L').version === 2);
ok('41 Ziffern passen in 1-L, 42 nicht', buildData('1'.repeat(41), 'L').version === 1 && buildData('1'.repeat(42), 'L').version === 2);
ok('2953 Bytes passen in 40-L', buildData('x'.repeat(2953), 'L').version === 40);
let tooLong = '';
try {
  buildData('x'.repeat(2954), 'L');
} catch (e) {
  tooLong = e.message;
}
ok('2954 Bytes -> Fehler "tooLong"', tooLong === 'tooLong');
ok('7089 Ziffern passen in 40-L', buildData('7'.repeat(7089), 'L').version === 40);
{
  // Byte-Modus 0100, Zähler (8 Bit) = 2, dann 0xC3 0xA4
  const d = buildData('ä', 'M').data;
  ok('UTF-8: "ä" zählt 2 Bytes', d[0] === 0x40 && d[1] >> 4 === 2 && (((d[1] & 0x0f) << 4) | (d[2] >> 4)) === 0xc3);
}
ok('Leerer Text ergibt Version 1', encode('').version === 1);

/* -------------------------------------------------------------------- */
head('Encoder: Rücklesen aus der Matrix');

/** Codewörter aus einer fertigen Matrix zurücklesen (Maske entfernen, Zickzack). */
function readBack(qr) {
  const { isFunction } = functionPatterns(qr.version);
  const out = [];
  let cur = 0;
  let n = 0;
  for (const [x, y] of dataPositions(isFunction)) {
    const bit = qr.modules[y][x] !== MASKS[qr.mask](x, y);
    cur = (cur << 1) | (bit ? 1 : 0);
    if (++n === 8) {
      out.push(cur);
      cur = 0;
      n = 0;
    }
  }
  return out;
}

/** Formatinfo oben links lesen und gegen alle 32 Möglichkeiten prüfen. */
function readFormat(qr) {
  const m = qr.modules;
  let bits = 0;
  const seq = [];
  for (let i = 0; i <= 5; i++) seq.push(m[i][8]);
  seq.push(m[7][8], m[8][8], m[8][7]);
  for (let i = 9; i < 15; i++) seq.push(m[8][14 - i]);
  seq.forEach((b, i) => (bits |= (b ? 1 : 0) << i));
  for (const e of ECC_LEVELS) for (let k = 0; k < 8; k++) if (formatBits(e, k) === bits) return e + k;
  return null;
}

function finderOk(qr) {
  const n = qr.size;
  const check = (cx, cy) => {
    for (let dy = -3; dy <= 3; dy++) {
      for (let dx = -3; dx <= 3; dx++) {
        const d = Math.max(Math.abs(dx), Math.abs(dy));
        if (qr.modules[cy + dy][cx + dx] !== (d !== 2)) return false;
      }
    }
    return true;
  };
  return check(3, 3) && check(n - 4, 3) && check(3, n - 4);
}

{
  let versionsOk = 0;
  let firstBad = '';
  for (let v = 1; v <= 40; v++) {
    const ecc = ECC_LEVELS[v % 4];
    const cap = dataCodewords(v, ecc) - 3; // Byte-Modus: 4 Bit Modus + 8/16 Bit Zähler
    const text = Array.from({ length: cap }, (_, i) => String.fromCharCode(97 + ((i * 7) % 26))).join('');
    const qr = encode(text, { ecc, minVersion: v });
    const d = buildData(text, ecc, { minVersion: v });
    const good =
      qr.version === v &&
      qr.size === v * 4 + 17 &&
      finderOk(qr) &&
      readFormat(qr) === ecc + qr.mask &&
      same(readBack(qr).slice(0, addEcc(d.data, v, ecc).length), addEcc(d.data, v, ecc));
    if (good) versionsOk++;
    else if (!firstBad) firstBad = 'Version ' + v + ' ' + ecc;
  }
  ok('Alle 40 Versionen: Finder, Formatinfo und Codewörter stimmen', versionsOk === 40, firstBad);
}

{
  const masks = new Set();
  for (let k = 0; k < 8; k++) {
    const qr = encode('Maskentest 123', { ecc: 'M', mask: k });
    if (qr.mask === k && readFormat(qr) === 'M' + k) masks.add(k);
  }
  ok('Feste Maske 0-7 wird übernommen und steht in der Formatinfo', masks.size === 8);
  const auto = encode('https://github.com/herrlamatv/useful');
  ok('Automatische Maske liegt zwischen 0 und 7', auto.mask >= 0 && auto.mask <= 7);
  ok('Timing-Muster abwechselnd', auto.modules[6].slice(8, auto.size - 8).every((b, i) => b === (i % 2 === 0)));
  ok('Dunkles Modul bei (8, 4V+9)', auto.modules[auto.size - 8][8] === true);
}

{
  // Reed-Solomon: Nachricht * x^n ist durch das Generatorpolynom teilbar,
  // also muss das Polynom an allen Wurzeln α^0 … α^(n-1) null sein.
  const d = buildData('Reed-Solomon', 'H');
  const n = 28; // 12 Bytes passen nicht in 1-H; Version 2-H = ein Block mit 28 ECC-Codewörtern
  const block = d.data.concat(addEcc(d.data, d.version, 'H').slice(d.data.length));
  let alpha = 1;
  let zero = true;
  for (let i = 0; i < n; i++) {
    let acc = 0;
    for (const c of block) acc = gfMul(acc, alpha) ^ c;
    if (acc !== 0) zero = false;
    alpha = gfMul(alpha, 2);
  }
  ok('Reed-Solomon: alle Syndrome sind 0', d.version === 2 && zero);
  ok('Generatorpolynom Grad 7 (Version 1-L)', same(rsDivisor(7), [127, 122, 154, 164, 11, 68, 117]));
}

/* -------------------------------------------------------------------- */
head('Vorlagen');

ok('WLAN: Sonderzeichen geschützt', escapeWifi('a;b,c:d\\e"f') === 'a\\;b\\,c\\:d\\\\e\\"f');
ok('WLAN WPA', buildPayload('wifi', { ssid: 'Home', password: 'p;w', security: 'WPA' }) === 'WIFI:T:WPA;S:Home;P:p\\;w;;');
ok('WLAN offen: kein Passwort', buildPayload('wifi', { ssid: 'Cafe', password: 'x', security: 'nopass' }) === 'WIFI:T:nopass;S:Cafe;;');
ok('WLAN versteckt', buildPayload('wifi', { ssid: 'X', password: 'y', security: 'WEP', hidden: true }) === 'WIFI:T:WEP;S:X;P:y;H:true;;');
ok('WLAN ohne SSID -> leer', buildPayload('wifi', { password: 'y' }) === '');

ok('E-Mail mit Betreff und Text', buildPayload('email', { to: 'a@b.de', subject: 'Hallo Welt', body: 'Zeile 1\nZeile 2 & mehr' }) === 'mailto:a@b.de?subject=Hallo%20Welt&body=Zeile%201%0AZeile%202%20%26%20mehr');
ok('E-Mail nur Adresse', buildPayload('email', { to: ' a@b.de ' }) === 'mailto:a@b.de');
ok('E-Mail leer -> leer', buildPayload('email', {}) === '');

ok('Telefon säubern', cleanPhone(' +49 (170) 123-45 67 ') === '+491701234567');
ok('Telefon', buildPayload('tel', { number: '0170 / 12 34' }) === 'tel:01701234');
ok('Telefon nur "+" -> leer', buildPayload('tel', { number: '+' }) === '');

ok('vCard: Sonderzeichen', escapeVcard('a,b;c\\d\ne') === 'a\\,b\\;c\\\\d\\ne');
{
  const v = buildPayload('contact', { first: 'Max', last: 'Muster', org: 'A, B', phone: '+49 170 1', email: 'm@x.de', url: 'https://x.de' });
  ok(
    'vCard komplett',
    v === ['BEGIN:VCARD', 'VERSION:3.0', 'N:Muster;Max;;;', 'FN:Max Muster', 'ORG:A\\, B', 'TEL;TYPE=CELL:+491701', 'EMAIL:m@x.de', 'URL:https://x.de', 'END:VCARD'].join('\r\n'),
    JSON.stringify(v)
  );
  ok('vCard nur Firma -> FN = Firma', buildPayload('contact', { org: 'ACME' }).includes('FN:ACME'));
  ok('vCard leer -> leer', buildPayload('contact', {}) === '');
}
ok('Text bleibt unverändert', buildPayload('text', { text: '  a\nb ' }) === '  a\nb ');
ok('Unbekannter Typ -> Text', buildPayload('???', { text: 'x' }) === 'x');
ok('Zusammenfassung Kontakt', summary('contact', { first: 'Max', last: 'M' }) === 'Max M');
ok('Dateiname aus URL', fileSlug('text', { text: 'https://www.Example.com/a?b' }) === 'qr-www.example.com');
ok('Dateiname mit Umlauten', fileSlug('wifi', { ssid: 'Büro WLAN!' }) === 'qr-buro-wlan');
ok('Dateiname leer -> Typ', fileSlug('tel', { number: '' }) === 'qr-tel');
ok('Jede Vorlage hat leere Felder', TYPES.every((t) => typeof emptyFields(t) === 'object'));

/* -------------------------------------------------------------------- */
head('Renderer');

{
  const qr = encode('HELLO WORLD', { ecc: 'Q', mask: 6 });
  const sq = shapes(qr, { margin: true });
  const dark = qr.modules.flat().filter(Boolean).length;
  const area = sq.shapes.reduce((s, r) => s + r.w * r.h, 0);
  ok('Quadrate: Fläche = Anzahl dunkler Module', area === dark, area + ' / ' + dark);
  ok('Quadrate: Rand 4 Module', sq.total === qr.size + 8 && Math.min(...sq.shapes.map((s) => s.x)) === 4);
  ok('Ohne Rand: beginnt bei 0', shapes(qr, { margin: false }).total === qr.size);
  const rd = shapes(qr, { rounded: true });
  ok('Rund: 3 Finder-Ringe mit Loch', rd.shapes.filter((s) => s.hole).length === 3);
  ok('Rund: Punkte = dunkle Module außerhalb der Finder', rd.shapes.length === 6 + dark - 3 * 33, rd.shapes.length);

  const svg = toSvg(qr, { fg: '#112233', bg: '#ffeedd', px: 300 });
  ok('SVG: Kopf, Größe, Farben', svg.startsWith('<?xml') && svg.includes('width="300"') && svg.includes('fill="#112233"') && svg.includes('fill="#ffeedd"'));
  ok('SVG: viewBox in Modulen', svg.includes('viewBox="0 0 29 29"'));
  ok('SVG transparent: kein Hintergrund', !toSvg(qr, { bg: 'transparent' }).includes('<rect'));
  ok('SVG rund: Bögen', toSvg(qr, { rounded: true }).includes(' 0 0 1 '));
  ok('SVG: Farbe wird escaped', toSvg(qr, { fg: '"><x' }).includes('fill="&quot;>&lt;x"'));

  // Canvas-Aufrufe mit einem Fake-Kontext prüfen
  const calls = [];
  const ctx = new Proxy(
    {},
    {
      get: (_, k) => (k === 'roundRect' ? (...a) => calls.push([k, ...a]) : (...a) => calls.push([k, ...a])),
      set: (_, k, v) => (calls.push(['set', k, v]), true)
    }
  );
  drawToContext(ctx, qr, 290, { fg: '#000000', bg: '#ffffff' });
  const rects = calls.filter((c) => c[0] === 'rect');
  ok('Canvas: ganze Pixel bei Quadraten', rects.length > 0 && rects.every((c) => c.slice(1).every(Number.isInteger)));
  ok('Canvas: Hintergrund zuerst', calls.some((c) => c[0] === 'fillRect' && c[3] === 290));
}

ok('Kontrast schwarz/weiß = 21', Math.round(contrast('#000000', '#ffffff')) === 21);
ok('Farbwarnung: invertiert', colorWarning('#ffffff', '#000000') === 'inverted');
ok('Farbwarnung: zu wenig Kontrast', colorWarning('#999999', '#aaaaaa') === 'low');
ok('Farbwarnung: Standard ok', colorWarning(DEFAULT_SETTINGS.fg, DEFAULT_SETTINGS.bg) === null);

/* -------------------------------------------------------------------- */
head('Speicher');

ok('Einstellungen: Standard bei Müll', same(mergeSettings('x'), DEFAULT_SETTINGS));
{
  const s = mergeSettings({ ecc: 'H', size: '1024', margin: false, rounded: true, fg: '#ABCDEF', bg: 'red', lang: 'fr', history: false });
  ok('Einstellungen: gültige Werte übernommen', s.ecc === 'H' && s.size === 1024 && !s.margin && s.rounded && s.fg === '#abcdef' && !s.history);
  ok('Einstellungen: ungültige verworfen', s.bg === DEFAULT_SETTINGS.bg && s.lang === 'auto');
  ok('Einstellungen: ECC "X" verworfen', mergeSettings({ ecc: 'X' }).ecc === 'M');
}
{
  let h = [];
  h = addHistory(h, { type: 'text', fields: { text: 'a' }, payload: 'a', at: 1 });
  h = addHistory(h, { type: 'tel', fields: { number: '1' }, payload: 'tel:1', at: 2 });
  h = addHistory(h, { type: 'text', fields: { text: 'a' }, payload: 'a', at: 3 });
  ok('Verlauf: gleicher Inhalt wandert nach oben', h.length === 2 && h[0].payload === 'a' && h[0].at === 3);
  ok('Verlauf: leerer Inhalt wird ignoriert', addHistory(h, { type: 'text', fields: {}, payload: '' }).length === 2);
  ok('Verlauf: entfernen', removeHistory(h, h[0].id).length === 1);
  let big = [];
  for (let i = 0; i < 40; i++) big = addHistory(big, { type: 'text', fields: { text: 'x' + i }, payload: 'x' + i, at: i });
  ok('Verlauf: höchstens ' + HISTORY_MAX, big.length === HISTORY_MAX && big[0].payload === 'x39');
  ok('Verlauf säubern', cleanHistory([null, { type: 'nope', payload: 'x' }, { type: 'text', payload: 'y', fields: { text: 5, evil: 1 } }]).length === 1);
  ok('Felder säubern', same(cleanFields('wifi', { ssid: 1, hidden: 'yes', evil: 2 }), { ssid: '1', password: '', security: 'WPA', hidden: true }));
}
ok('Rechtsklick: frisch -> Text', takePending({ text: 'x', at: 1000 }, 2000) === 'x');
ok('Rechtsklick: alt -> nichts', takePending({ text: 'x', at: 0 }, 10 * 60 * 1000) === null);
ok('Rechtsklick: kaputt -> nichts', takePending({ text: 5 }) === null && takePending(null) === null);

ok('Menü: Link', textFor('qr-link', { linkUrl: 'https://a' }) === 'https://a');
ok('Menü: Auswahl getrimmt', textFor('qr-selection', { selectionText: '  hi  ' }) === 'hi');
ok('Menü: Bild', textFor('qr-image', { srcUrl: 'https://i' }) === 'https://i');
ok('Menü: Seite (Frame vor Seite vor Tab)', textFor('qr-page', { pageUrl: 'p' }, { url: 't' }) === 'p' && textFor('qr-page', {}, { url: 't' }) === 't');

/* -------------------------------------------------------------------- */
head('Übersetzungen & Manifest');

{
  const en = Object.keys(enMessages).sort();
  const de = Object.keys(deMessages).sort();
  ok('de und en haben dieselben Schlüssel', same(en, de), en.filter((k) => !de.includes(k)).concat(de.filter((k) => !en.includes(k))).join(', '));

  const used = new Set();
  const app = readFileSync('ui/app.js', 'utf8');
  for (const m of app.matchAll(/\bt\('([a-zA-Z.]+)'/g)) if (!m[1].endsWith('.')) used.add(m[1]); // 'type.' + … ist dynamisch
  for (const type of TYPES) used.add('type.' + type).add('type.' + type + 'Title').add('preview.empty.' + type);
  for (const k of ['preview.tooLong', 'preview.failed', 'style.warn.inverted', 'style.warn.low', 'style.fg', 'style.bg']) used.add(k);
  for (const file of ['ui/popup.html', 'ui/qr.html']) {
    for (const m of readFileSync(file, 'utf8').matchAll(/data-i18n(?:-ph|-title)?="([^"]+)"/g)) used.add(m[1]);
  }
  const missing = [...used].filter((k) => !(k in enMessages) || !(k in deMessages));
  ok('Alle benutzten Schlüssel existieren (' + used.size + ')', missing.length === 0, missing.join(', '));
  const unused = en.filter((k) => !used.has(k));
  ok('Keine toten Schlüssel', unused.length === 0, unused.join(', '));
  const placeholders = (s) => (s.match(/\{\w+\}/g) || []).sort().join();
  const badVars = en.filter((k) => placeholders(enMessages[k]) !== placeholders(deMessages[k]));
  ok('Platzhalter gleich in de/en', badVars.length === 0, badVars.join(', '));
}
{
  const manifest = JSON.parse(readFileSync('manifest.json', 'utf8'));
  const files = [
    manifest.background.service_worker,
    manifest.action.default_popup,
    manifest.options_ui.page,
    ...Object.values(manifest.icons),
    ...Object.values(manifest.action.default_icon)
  ];
  const gone = files.filter((f) => !existsSync(f));
  ok('Manifest: alle Dateien vorhanden', gone.length === 0, gone.join(', '));
  ok('Manifest: V3, keine Host-Rechte', manifest.manifest_version === 3 && !manifest.host_permissions);
  const msgKeys = ['extName', 'extDescription', 'actionTitle', ...MENUS.map((m) => m.title)];
  for (const lang of ['de', 'en']) {
    const msgs = JSON.parse(readFileSync('_locales/' + lang + '/messages.json', 'utf8'));
    const miss = msgKeys.filter((k) => !msgs[k] || !msgs[k].message);
    ok('_locales/' + lang + ': alle Texte (' + msgKeys.length + ')', miss.length === 0, miss.join(', '));
  }
}

/* -------------------------------------------------------------------- */
head('Echter Scan (OpenCV)');

/** Formen in ein Graustufen-PNG rastern - wie Canvas, nur in Node. */
function rasterize(qr, opts, px = 8) {
  const { total, shapes: list } = shapes(qr, opts);
  const size = total * px;
  const img = new Uint8Array(size * size).fill(255);
  const inside = (s, x, y) => {
    const r = s.r || 0;
    const cx = Math.max(s.x + r - x, 0, x - (s.x + s.w - r));
    const cy = Math.max(s.y + r - y, 0, y - (s.y + s.h - r));
    return x >= s.x && x <= s.x + s.w && y >= s.y && y <= s.y + s.h && Math.hypot(cx, cy) <= r + 1e-9;
  };
  for (const s of list) {
    for (let py = Math.floor(s.y * px); py < Math.ceil((s.y + s.h) * px); py++) {
      for (let pxl = Math.floor(s.x * px); pxl < Math.ceil((s.x + s.w) * px); pxl++) {
        const x = (pxl + 0.5) / px;
        const y = (py + 0.5) / px;
        if (inside(s, x, y) && !(s.hole && inside(s.hole, x, y))) img[py * size + pxl] = 0;
      }
    }
  }
  return pngGray(size, img);
}

function pngGray(size, pixels) {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf) => {
    let c = 0xffffffff;
    for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type), data]);
    const c = Buffer.alloc(4);
    c.writeUInt32BE(crc(body));
    return Buffer.concat([len, body, c]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 0; // Graustufen
  const raw = Buffer.alloc((size + 1) * size);
  for (let y = 0; y < size; y++) Buffer.from(pixels.subarray(y * size, (y + 1) * size)).copy(raw, y * (size + 1) + 1);
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

{
  const python = ['python', 'py'].find((cmd) => spawnSync(cmd, ['-c', 'import cv2'], { encoding: 'utf8' }).status === 0);
  if (process.env.QREXT_E2E === '0') console.log('  skip (QREXT_E2E=0)');
  else if (!python) console.log('  skip (kein Python mit OpenCV - pip install opencv-python)');
  else {
    const dir = join(tmpdir(), 'qrext-test-' + process.pid);
    mkdirSync(dir, { recursive: true });
    const cases = [
      ['URL', 'https://github.com/herrlamatv/useful', 'M', false],
      ['Ziffern', '0123456789012345678901234567890', 'L', false],
      ['Alphanumerisch', 'HELLO WORLD $%*+-./:', 'Q', false],
      ['Umlaute', 'Grüße aus Köln – Maß und Straße', 'M', false],
      ['WLAN', buildPayload('wifi', { ssid: 'Mein;WLAN', password: 'ge:heim"1', security: 'WPA' }), 'H', false],
      ['vCard', buildPayload('contact', { first: 'Max', last: 'Muster', org: 'A, B', phone: '+49 170 1', email: 'm@x.de' }), 'M', false],
      ['E-Mail', buildPayload('email', { to: 'a@b.de', subject: 'Hallo', body: 'Test & mehr' }), 'Q', false],
      ['lang (Version 10+)', 'https://example.com/?q=' + 'abcdefghij'.repeat(25), 'M', false],
      ['URL rund', 'https://github.com/herrlamatv/useful', 'M', true],
      ['WLAN rund', buildPayload('wifi', { ssid: 'Büro', password: 'x;y:z', security: 'WPA' }), 'Q', true],
      ['lang rund (Version 10+)', 'https://example.com/?q=' + 'abcdefghij'.repeat(25), 'M', true],
      ['vCard rund, H', buildPayload('contact', { first: 'Erika', last: 'Musterfrau', phone: '+49 30 1234', url: 'https://x.de' }), 'H', true]
    ];
    const expected = [];
    cases.forEach(([name, text, ecc, rounded], i) => {
      const qr = encode(text, { ecc });
      writeFileSync(join(dir, i + '.png'), rasterize(qr, { margin: true, rounded }));
      expected.push({ name: name + ' (V' + qr.version + '-' + ecc + ')', text });
    });
    const script = [
      'import cv2, json, sys',
      'd = cv2.QRCodeDetector()',
      'a = cv2.QRCodeDetectorAruco()  # robuster - liest, was der klassische manchmal nicht findet',
      'out = []',
      'for i in range(' + cases.length + '):',
      '    img = cv2.imread(sys.argv[1] + "/" + str(i) + ".png", cv2.IMREAD_GRAYSCALE)',
      '    txt = d.detectAndDecode(img)[0] or a.detectAndDecode(img)[0]',
      '    out.append(txt)',
      'sys.stdout.buffer.write(json.dumps(out).encode("utf-8"))'
    ].join('\n');
    const res = spawnSync(python, ['-c', script, dir], { encoding: 'utf8' });
    let decoded = [];
    try {
      decoded = JSON.parse(res.stdout);
    } catch (e) {
      console.log('  FAIL OpenCV-Aufruf  -> ' + (res.stderr || res.stdout).slice(0, 300));
      fails++;
    }
    expected.forEach((exp, i) => {
      if (!decoded.length) return;
      ok('gescannt: ' + exp.name, decoded[i] === exp.text, JSON.stringify(decoded[i]));
    });
    rmSync(dir, { recursive: true, force: true });
  }
}

console.log('\n' + (fails === 0 ? 'Alle Tests bestanden.' : fails + ' Test(s) fehlgeschlagen.'));
process.exit(fails === 0 ? 0 : 1);

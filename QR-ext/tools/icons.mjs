/* QR-ext - by herrlamatv
 * Formen als Abstandsfunktionen, 4×4-Supersampling, PNG von Hand.
 *
 * Creme-Kachel wie bei chroofer, darauf in Karamell: drei Finder-Muster
 * und ein paar Datenpunkte - ein stilisierter QR-Code.
 */

import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const OUT = join(dirname(fileURLToPath(import.meta.url)), '..', 'icons');

const EDGE = [229, 216, 189, 255];
const CREAM = [250, 245, 233, 255];
const CARAMEL = [185, 139, 82, 255];

/* Formen */

/** Abgerundetes Rechteck (Koordinaten im 128er-Raster). */
function roundRect(x0, y0, x1, y1, r) {
  return (x, y) => {
    const cx = Math.max(x0 + r - x, 0, x - (x1 - r));
    const cy = Math.max(y0 + r - y, 0, y - (y1 - r));
    return Math.hypot(cx, cy) <= r;
  };
}

function any(...shapes) {
  return (x, y) => shapes.some((s) => s(x, y));
}

/** Finder-Muster: Ring (7 Module) mit Kern (3 Module), Kantenlänge s. */
function finder(x, y, s, r) {
  const m = s / 7;
  const outer = roundRect(x, y, x + s, y + s, r);
  const hole = roundRect(x + m, y + m, x + s - m, y + s - m, r * 0.55);
  const core = roundRect(x + 2 * m, y + 2 * m, x + s - 2 * m, y + s - 2 * m, r * 0.45);
  return (px, py) => (outer(px, py) && !hole(px, py)) || core(px, py);
}

/* Große Variante: drei Finder und ein paar Datenpunkte unten rechts. */
function glyphLarge() {
  const d = 9;
  const dot = (x, y) => roundRect(x, y, x + d, y + d, 2.6);
  return any(
    finder(24, 24, 36, 8),
    finder(68, 24, 36, 8),
    finder(24, 68, 36, 8),
    dot(69, 69),
    dot(81, 69),
    dot(95, 69),
    dot(69, 83),
    dot(88, 83),
    dot(81, 95),
    dot(95, 95),
    dot(69, 95)
  );
}

/* Kleine Variante: dicker, drei Finder und ein Block. */
function glyphSmall() {
  const r = (x0, y0, x1, y1, rr) => roundRect(x0, y0, x1, y1, rr);
  const ring = (x, y) => {
    const outer = r(x, y, x + 42, y + 42, 9);
    const hole = r(x + 9, y + 9, x + 33, y + 33, 4);
    const core = r(x + 15, y + 15, x + 27, y + 27, 3);
    return (px, py) => (outer(px, py) && !hole(px, py)) || core(px, py);
  };
  return any(ring(18, 18), ring(68, 18), ring(18, 68), r(70, 70, 88, 88, 4), r(92, 92, 110, 110, 4), r(92, 70, 110, 84, 4));
}

/* 16 px: alles auf ganze Pixel (1 px = 8 Einheiten), sonst wird es Brei. */
function glyphTiny() {
  const box = (x0, y0, x1, y1) => roundRect(x0, y0, x1, y1, 0);
  const ring = (x, y) => {
    const outer = roundRect(x, y, x + 40, y + 40, 6);
    const hole = box(x + 8, y + 8, x + 32, y + 32);
    const core = box(x + 16, y + 16, x + 24, y + 24);
    return (px, py) => (outer(px, py) && !hole(px, py)) || core(px, py);
  };
  return any(ring(16, 16), ring(72, 16), ring(16, 72), box(72, 72, 80, 80), box(88, 88, 96, 96), box(104, 72, 112, 80), box(104, 104, 112, 112), box(72, 104, 80, 112));
}

function draw(size) {
  const small = size <= 32;
  const tileOuter = roundRect(4, 4, 124, 124, 28);
  const tileInner = roundRect(small ? 9 : 7, small ? 9 : 7, small ? 119 : 121, small ? 119 : 121, small ? 23 : 25);
  const glyph = size <= 16 ? glyphTiny() : small ? glyphSmall() : glyphLarge();

  const S = 4;
  const px = Buffer.alloc(size * size * 4);
  for (let py = 0; py < size; py++) {
    for (let pxl = 0; pxl < size; pxl++) {
      let r = 0,
        g = 0,
        b = 0,
        a = 0;
      for (let sy = 0; sy < S; sy++) {
        for (let sx = 0; sx < S; sx++) {
          const x = ((pxl + (sx + 0.5) / S) / size) * 128;
          const y = ((py + (sy + 0.5) / S) / size) * 128;
          let c = null;
          if (tileOuter(x, y)) c = EDGE;
          if (tileInner(x, y)) c = CREAM;
          if (c && glyph(x, y)) c = CARAMEL;
          if (!c) continue;
          // vormultipliziert mitteln
          r += c[0];
          g += c[1];
          b += c[2];
          a += 255;
        }
      }
      const n = S * S;
      const o = (py * size + pxl) * 4;
      if (a === 0) continue;
      px[o] = Math.round(r / (a / 255));
      px[o + 1] = Math.round(g / (a / 255));
      px[o + 2] = Math.round(b / (a / 255));
      px[o + 3] = Math.round(a / n);
    }
  }
  return png(size, size, px);
}

/* png  */

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

function png(w, h, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; // Bit-Tiefe
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0; // Filter: keiner
    rgba.copy(raw, y * (w * 4 + 1) + 1, y * w * 4, (y + 1) * w * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ]);
}

mkdirSync(OUT, { recursive: true });
for (const size of [16, 32, 48, 128]) {
  const file = join(OUT, 'icon' + size + '.png');
  writeFileSync(file, draw(size));
  console.log('geschrieben: icons/icon' + size + '.png');
}

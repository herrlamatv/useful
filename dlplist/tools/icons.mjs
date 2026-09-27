/* dlplist - by herrlamatv
 * Zeichnet die Symbole (16/32/48/128 px) ohne Abhängigkeiten:
 * Formen als Abstandsfunktionen, 4×4-Supersampling, PNG von Hand.
 *
 *   node tools/icons.mjs
 *
 * Creme-Kachel wie bei chroofer, darauf in Karamell: Listenbalken, ein
 * Pfeil nach unten und die Ablage darunter.
 */

import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const OUT = join(dirname(fileURLToPath(import.meta.url)), '..', 'icons');

const EDGE = [229, 216, 189, 255];
const CREAM = [250, 245, 233, 255];
const CARAMEL = [185, 139, 82, 255];

/* ------------------------------ Formen ------------------------------ */

/** Abgerundetes Rechteck (Koordinaten im 128er-Raster). */
function roundRect(x0, y0, x1, y1, r) {
  return (x, y) => {
    const cx = Math.max(x0 + r - x, 0, x - (x1 - r));
    const cy = Math.max(y0 + r - y, 0, y - (y1 - r));
    return Math.hypot(cx, cy) <= r;
  };
}

/** Strich mit runden Enden von (ax,ay) nach (bx,by), halbe Breite w. */
function capsule(ax, ay, bx, by, w) {
  const dx = bx - ax;
  const dy = by - ay;
  const len2 = dx * dx + dy * dy || 1;
  return (x, y) => {
    const t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / len2));
    return Math.hypot(x - (ax + t * dx), y - (ay + t * dy)) <= w;
  };
}

function any(...shapes) {
  return (x, y) => shapes.some((s) => s(x, y));
}

/* Große Variante: drei Listenbalken, Pfeil, Ablage. */
function glyphLarge() {
  const w = 5.5;
  return any(
    capsule(30, 36, 62, 36, w),
    capsule(30, 54, 56, 54, w),
    capsule(30, 72, 50, 72, w),
    capsule(88, 30, 88, 78, w),
    capsule(72, 63, 88, 79, w),
    capsule(104, 63, 88, 79, w),
    capsule(30, 97, 104, 97, w)
  );
}

/* Kleine Variante: dicker, nur Pfeil, Ablage und zwei Balken. */
function glyphSmall() {
  const w = 10;
  return any(
    capsule(34, 40, 52, 40, w),
    capsule(34, 66, 46, 66, w),
    capsule(86, 26, 86, 72, w),
    capsule(66, 54, 86, 74, w),
    capsule(106, 54, 86, 74, w),
    capsule(34, 100, 102, 100, w)
  );
}

function draw(size) {
  const small = size <= 32;
  const tileOuter = roundRect(4, 4, 124, 124, 28);
  const tileInner = roundRect(small ? 9 : 7, small ? 9 : 7, small ? 119 : 121, small ? 119 : 121, small ? 23 : 25);
  const glyph = small ? glyphSmall() : glyphLarge();

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

/* ------------------------------- PNG -------------------------------- */

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

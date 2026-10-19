/* ai chat export - by herrlamatv */

import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const OUT = join(dirname(fileURLToPath(import.meta.url)), '..', 'icons');

const EDGE = [229, 216, 189, 255];
const CREAM = [250, 245, 233, 255];
const CARAMEL = [185, 139, 82, 255];

/* Formen */

function roundRect(x0, y0, x1, y1, r) {
  return (x, y) => {
    const cx = Math.max(x0 + r - x, 0, x - (x1 - r));
    const cy = Math.max(y0 + r - y, 0, y - (y1 - r));
    return Math.hypot(cx, cy) <= r;
  };
}

/** Dicke Linie von (x0,y0) nach (x1,y1) mit runden Enden. */
function stroke(x0, y0, x1, y1, w) {
  const dx = x1 - x0;
  const dy = y1 - y0;
  const len2 = dx * dx + dy * dy;
  return (x, y) => {
    const t = Math.max(0, Math.min(1, ((x - x0) * dx + (y - y0) * dy) / len2));
    return Math.hypot(x - (x0 + t * dx), y - (y0 + t * dy)) <= w / 2;
  };
}

/** Dreieck über Vorzeichen der drei Kanten. */
function triangle(ax, ay, bx, by, cx, cy) {
  const side = (px, py, qx, qy, x, y) => (qx - px) * (y - py) - (qy - py) * (x - px);
  return (x, y) => {
    const d1 = side(ax, ay, bx, by, x, y);
    const d2 = side(bx, by, cx, cy, x, y);
    const d3 = side(cx, cy, ax, ay, x, y);
    const neg = d1 < 0 || d2 < 0 || d3 < 0;
    const pos = d1 > 0 || d2 > 0 || d3 > 0;
    return !(neg && pos);
  };
}

const any = (...shapes) => (x, y) => shapes.some((s) => s(x, y));

/* Groß: Blase mit Zipfel unten links, Pfeil ausgespart. */
function glyphLarge() {
  const bubble = any(roundRect(22, 24, 106, 88, 20), triangle(36, 80, 60, 86, 30, 106));
  const arrow = any(stroke(64, 38, 64, 70, 9), stroke(50, 57, 64, 71, 9), stroke(78, 57, 64, 71, 9));
  return (x, y) => (arrow(x, y) ? CREAM : bubble(x, y) ? CARAMEL : null);
}

/* Klein: weniger Rand, dickerer Pfeil - sonst verschwimmt er bei 16 px. */
function glyphSmall() {
  const bubble = any(roundRect(16, 20, 112, 92, 22), triangle(30, 82, 60, 90, 22, 114));
  const arrow = any(stroke(64, 36, 64, 72, 15), stroke(46, 56, 64, 74, 15), stroke(82, 56, 64, 74, 15));
  return (x, y) => (arrow(x, y) ? CREAM : bubble(x, y) ? CARAMEL : null);
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
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      for (let sy = 0; sy < S; sy++) {
        for (let sx = 0; sx < S; sx++) {
          const x = ((pxl + (sx + 0.5) / S) / size) * 128;
          const y = ((py + (sy + 0.5) / S) / size) * 128;
          let c = null;
          if (tileOuter(x, y)) c = EDGE;
          if (tileInner(x, y)) c = CREAM;
          if (c) c = glyph(x, y) || c;
          if (!c) continue;
          r += c[0];
          g += c[1];
          b += c[2];
          a += 255;
        }
      }
      if (a === 0) continue;
      const o = (py * size + pxl) * 4;
      px[o] = Math.round(r / (a / 255));
      px[o + 1] = Math.round(g / (a / 255));
      px[o + 2] = Math.round(b / (a / 255));
      px[o + 3] = Math.round(a / (S * S));
    }
  }
  return png(size, size, px);
}

/* png */

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
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
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
  writeFileSync(join(OUT, 'icon' + size + '.png'), draw(size));
  console.log('geschrieben: icons/icon' + size + '.png');
}

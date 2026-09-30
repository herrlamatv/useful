/* QR-ext - by herrlamatv
 * QR-Code-Encoder ohne Abhängigkeiten (ISO/IEC 18004, Modell 2).
 *
 *   encode('https://example.com', { ecc: 'M' })
 *     -> { version, size, ecc, mask, mode, modules: boolean[][] }
 *
 * Modi: Numeric, Alphanumeric, Byte (UTF-8) - der kleinste passende wird
 * genommen. Versionen 1-40, Fehlerkorrektur L/M/Q/H, alle 8 Masken werden
 * bewertet und die mit der geringsten Strafpunktzahl gewinnt.
 */

export const ECC_LEVELS = ['L', 'M', 'Q', 'H'];

/* Formatbits je Stufe (L=01, M=00, Q=11, H=10). */
const FORMAT_BITS = { L: 1, M: 0, Q: 3, H: 2 };
const ECC_INDEX = { L: 0, M: 1, Q: 2, H: 3 };

/* Fehlerkorrektur-Codewörter pro Block, Index = Version (0 unbenutzt). */
const ECC_PER_BLOCK = [
  [-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
  [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
  [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
  [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30]
];

/* Anzahl der Fehlerkorrektur-Blöcke, Index = Version. */
const NUM_BLOCKS = [
  [-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
  [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
  [-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
  [-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81]
];

const ALNUM = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:';

/* Modus: Kennung und Länge des Zeichenzählers je Versionsbereich 1-9 / 10-26 / 27-40. */
export const MODES = {
  numeric: { id: 1, cc: [10, 12, 14] },
  alnum: { id: 2, cc: [9, 11, 13] },
  byte: { id: 4, cc: [8, 16, 16] }
};

/* ---------------------------- Kapazitäten ---------------------------- */

/** Anzahl der Module, die Daten tragen (ohne Funktionsmuster und Format). */
export function rawDataModules(ver) {
  let result = (16 * ver + 128) * ver + 64;
  if (ver >= 2) {
    const numAlign = Math.floor(ver / 7) + 2;
    result -= (25 * numAlign - 10) * numAlign - 55;
    if (ver >= 7) result -= 36;
  }
  return result;
}

/** Datencodewörter (ohne Fehlerkorrektur) für Version + Stufe. */
export function dataCodewords(ver, ecc) {
  const e = ECC_INDEX[ecc];
  return Math.floor(rawDataModules(ver) / 8) - ECC_PER_BLOCK[e][ver] * NUM_BLOCKS[e][ver];
}

function ccBits(mode, ver) {
  return MODES[mode].cc[ver <= 9 ? 0 : ver <= 26 ? 1 : 2];
}

/* ---------------------------- Datenstrom ----------------------------- */

export function utf8(text) {
  return Array.from(new TextEncoder().encode(text));
}

/** Kleinster Modus, der den ganzen Text abdeckt. */
export function pickMode(text) {
  if (/^[0-9]*$/.test(text)) return 'numeric';
  if ([...text].every((c) => ALNUM.includes(c))) return 'alnum';
  return 'byte';
}

class Bits {
  constructor() {
    this.bits = [];
  }
  put(value, len) {
    for (let i = len - 1; i >= 0; i--) this.bits.push((value >>> i) & 1);
  }
  get length() {
    return this.bits.length;
  }
}

/** Nutzdaten des Segments (ohne Modus und Zähler). */
function segmentData(text, mode) {
  const bb = new Bits();
  if (mode === 'numeric') {
    for (let i = 0; i < text.length; i += 3) {
      const chunk = text.slice(i, i + 3);
      bb.put(parseInt(chunk, 10), chunk.length * 3 + 1);
    }
    return { bits: bb.bits, count: text.length };
  }
  if (mode === 'alnum') {
    for (let i = 0; i < text.length; i += 2) {
      if (i + 1 < text.length) bb.put(ALNUM.indexOf(text[i]) * 45 + ALNUM.indexOf(text[i + 1]), 11);
      else bb.put(ALNUM.indexOf(text[i]), 6);
    }
    return { bits: bb.bits, count: text.length };
  }
  const bytes = utf8(text);
  for (const b of bytes) bb.put(b, 8);
  return { bits: bb.bits, count: bytes.length };
}

/**
 * Kleinste passende Version suchen und den Datenstrom samt Füllbytes bauen.
 * @returns { version, mode, data: number[] } - data = Datencodewörter
 */
export function buildData(text, ecc = 'M', { minVersion = 1 } = {}) {
  if (!(ecc in ECC_INDEX)) throw new Error('badEcc');
  const mode = pickMode(text);
  const seg = segmentData(text, mode);

  let version = 0;
  for (let v = Math.max(1, minVersion); v <= 40; v++) {
    const cc = ccBits(mode, v);
    const used = 4 + cc + seg.bits.length;
    if (seg.count < 1 << cc && used <= dataCodewords(v, ecc) * 8) {
      version = v;
      break;
    }
  }
  if (!version) throw new Error('tooLong');

  const capacity = dataCodewords(version, ecc) * 8;
  const bb = new Bits();
  bb.put(MODES[mode].id, 4);
  bb.put(seg.count, ccBits(mode, version));
  bb.bits.push(...seg.bits);
  bb.put(0, Math.min(4, capacity - bb.length)); // Abschluss
  bb.put(0, (8 - (bb.length % 8)) % 8); // auf Byte auffüllen
  for (let pad = 0xec; bb.length < capacity; pad ^= 0xec ^ 0x11) bb.put(pad, 8);

  const data = [];
  for (let i = 0; i < bb.length; i += 8) {
    let byte = 0;
    for (let j = 0; j < 8; j++) byte = (byte << 1) | bb.bits[i + j];
    data.push(byte);
  }
  return { version, mode, data };
}

/* --------------------------- Reed-Solomon ---------------------------- */

export function gfMul(x, y) {
  let z = 0;
  for (let i = 7; i >= 0; i--) {
    z = (z << 1) ^ ((z >>> 7) * 0x11d);
    z ^= ((y >>> i) & 1) * x;
  }
  return z;
}

export function rsDivisor(degree) {
  const result = new Array(degree).fill(0);
  result[degree - 1] = 1;
  let root = 1;
  for (let i = 0; i < degree; i++) {
    for (let j = 0; j < degree; j++) {
      result[j] = gfMul(result[j], root);
      if (j + 1 < degree) result[j] ^= result[j + 1];
    }
    root = gfMul(root, 0x02);
  }
  return result;
}

export function rsRemainder(data, divisor) {
  const result = divisor.map(() => 0);
  for (const b of data) {
    const factor = b ^ result.shift();
    result.push(0);
    divisor.forEach((coef, i) => (result[i] ^= gfMul(coef, factor)));
  }
  return result;
}

/** Datencodewörter in Blöcke teilen, Fehlerkorrektur anhängen, verschränken. */
export function addEcc(data, version, ecc) {
  const e = ECC_INDEX[ecc];
  const numBlocks = NUM_BLOCKS[e][version];
  const eccLen = ECC_PER_BLOCK[e][version];
  const rawCodewords = Math.floor(rawDataModules(version) / 8);
  const numShort = numBlocks - (rawCodewords % numBlocks);
  const shortLen = Math.floor(rawCodewords / numBlocks);

  const divisor = rsDivisor(eccLen);
  const blocks = [];
  for (let i = 0, k = 0; i < numBlocks; i++) {
    const dat = data.slice(k, k + shortLen - eccLen + (i < numShort ? 0 : 1));
    k += dat.length;
    const block = dat.concat(rsRemainder(dat, divisor));
    if (i < numShort) block.splice(dat.length, 0, 0); // Platzhalter, wird übersprungen
    blocks.push(block);
  }

  const result = [];
  for (let i = 0; i < blocks[0].length; i++) {
    blocks.forEach((block, j) => {
      if (i !== shortLen - eccLen || j >= numShort) result.push(block[i]);
    });
  }
  return result;
}

/* ----------------------------- Matrix -------------------------------- */

export function alignmentPositions(ver) {
  if (ver === 1) return [];
  const numAlign = Math.floor(ver / 7) + 2;
  const step = Math.floor((ver * 8 + numAlign * 3 + 5) / (numAlign * 4 - 4)) * 2;
  const result = [6];
  for (let pos = ver * 4 + 10; result.length < numAlign; pos -= step) result.splice(1, 0, pos);
  return result;
}

export function formatBits(ecc, mask) {
  const data = (FORMAT_BITS[ecc] << 3) | mask;
  let rem = data;
  for (let i = 0; i < 10; i++) rem = (rem << 1) ^ ((rem >>> 9) * 0x537);
  return ((data << 10) | rem) ^ 0x5412;
}

export function versionBits(ver) {
  let rem = ver;
  for (let i = 0; i < 12; i++) rem = (rem << 1) ^ ((rem >>> 11) * 0x1f25);
  return (ver << 12) | rem;
}

const bit = (x, i) => ((x >>> i) & 1) !== 0;

function makeGrid(size) {
  return Array.from({ length: size }, () => new Array(size).fill(false));
}

function drawFormat(m, fn, ecc, mask) {
  const size = m.length;
  const bits = formatBits(ecc, mask);
  const set = (x, y, dark) => {
    m[y][x] = dark;
    fn[y][x] = true;
  };
  for (let i = 0; i <= 5; i++) set(8, i, bit(bits, i));
  set(8, 7, bit(bits, 6));
  set(8, 8, bit(bits, 7));
  set(7, 8, bit(bits, 8));
  for (let i = 9; i < 15; i++) set(14 - i, 8, bit(bits, i));
  for (let i = 0; i < 8; i++) set(size - 1 - i, 8, bit(bits, i));
  for (let i = 8; i < 15; i++) set(8, size - 15 + i, bit(bits, i));
  set(8, size - 8, true); // immer dunkles Modul
}

/** Alle Funktionsmuster zeichnen. Liefert { modules, isFunction }. */
export function functionPatterns(version) {
  const size = version * 4 + 17;
  const m = makeGrid(size);
  const fn = makeGrid(size);
  const set = (x, y, dark) => {
    m[y][x] = dark;
    fn[y][x] = true;
  };

  for (let i = 0; i < size; i++) {
    set(6, i, i % 2 === 0);
    set(i, 6, i % 2 === 0);
  }

  const finder = (cx, cy) => {
    for (let dy = -4; dy <= 4; dy++) {
      for (let dx = -4; dx <= 4; dx++) {
        const x = cx + dx;
        const y = cy + dy;
        if (x < 0 || y < 0 || x >= size || y >= size) continue;
        const dist = Math.max(Math.abs(dx), Math.abs(dy));
        set(x, y, dist !== 2 && dist !== 4);
      }
    }
  };
  finder(3, 3);
  finder(size - 4, 3);
  finder(3, size - 4);

  const pos = alignmentPositions(version);
  const n = pos.length;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      if ((i === 0 && j === 0) || (i === 0 && j === n - 1) || (i === n - 1 && j === 0)) continue;
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) {
          set(pos[i] + dx, pos[j] + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1);
        }
      }
    }
  }

  drawFormat(m, fn, 'L', 0); // Platz reservieren, echte Bits kommen später

  if (version >= 7) {
    const bits = versionBits(version);
    for (let i = 0; i < 18; i++) {
      const a = size - 11 + (i % 3);
      const b = Math.floor(i / 3);
      set(a, b, bit(bits, i));
      set(b, a, bit(bits, i));
    }
  }
  return { modules: m, isFunction: fn };
}

/** Reihenfolge der Datenmodule (Zickzack von rechts unten). */
export function dataPositions(isFunction) {
  const size = isFunction.length;
  const out = [];
  for (let right = size - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5;
    for (let vert = 0; vert < size; vert++) {
      for (let j = 0; j < 2; j++) {
        const x = right - j;
        const upward = ((right + 1) & 2) === 0;
        const y = upward ? size - 1 - vert : vert;
        if (!isFunction[y][x]) out.push([x, y]);
      }
    }
  }
  return out;
}

export const MASKS = [
  (x, y) => (x + y) % 2 === 0,
  (x, y) => y % 2 === 0,
  (x, y) => x % 3 === 0,
  (x, y) => (x + y) % 3 === 0,
  (x, y) => (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0,
  (x, y) => ((x * y) % 2) + ((x * y) % 3) === 0,
  (x, y) => (((x * y) % 2) + ((x * y) % 3)) % 2 === 0,
  (x, y) => (((x + y) % 2) + ((x * y) % 3)) % 2 === 0
];

function applyMask(m, fn, mask) {
  const test = MASKS[mask];
  for (let y = 0; y < m.length; y++) {
    for (let x = 0; x < m.length; x++) {
      if (!fn[y][x] && test(x, y)) m[y][x] = !m[y][x];
    }
  }
}

/** Strafpunkte nach den vier Regeln der Norm - kleiner ist besser. */
export function penalty(m) {
  const size = m.length;
  let score = 0;
  const lines = [];
  for (let y = 0; y < size; y++) lines.push(m[y]);
  for (let x = 0; x < size; x++) lines.push(m.map((row) => row[x]));

  for (const line of lines) {
    // Regel 1: fünf oder mehr gleiche in Folge
    let run = 1;
    for (let i = 1; i <= size; i++) {
      if (i < size && line[i] === line[i - 1]) run++;
      else {
        if (run >= 5) score += 3 + (run - 5);
        run = 1;
      }
    }
    // Regel 3: Finder-ähnliches Muster mit vier hellen Modulen daneben
    const s = line.map((d) => (d ? '1' : '0')).join('');
    for (const pat of ['10111010000', '00001011101']) {
      for (let i = s.indexOf(pat); i !== -1; i = s.indexOf(pat, i + 1)) score += 40;
    }
  }

  // Regel 2: 2×2-Blöcke gleicher Farbe
  for (let y = 0; y < size - 1; y++) {
    for (let x = 0; x < size - 1; x++) {
      const c = m[y][x];
      if (c === m[y][x + 1] && c === m[y + 1][x] && c === m[y + 1][x + 1]) score += 3;
    }
  }

  // Regel 4: Anteil dunkler Module weit weg von 50 %
  let dark = 0;
  for (const row of m) for (const d of row) if (d) dark++;
  const total = size * size;
  const k = Math.ceil(Math.abs(dark * 20 - total * 10) / total) - 1;
  score += Math.max(0, k) * 10;
  return score;
}

/**
 * Text als QR-Code kodieren.
 * @param {string} text
 * @param {{ ecc?: 'L'|'M'|'Q'|'H', mask?: number, minVersion?: number }} opts
 */
export function encode(text, { ecc = 'M', mask = -1, minVersion = 1 } = {}) {
  const { version, mode, data } = buildData(String(text), ecc, { minVersion });
  const codewords = addEcc(data, version, ecc);
  const { modules, isFunction } = functionPatterns(version);

  const positions = dataPositions(isFunction);
  positions.forEach(([x, y], i) => {
    const byte = codewords[i >>> 3];
    modules[y][x] = byte !== undefined && bit(byte, 7 - (i & 7)); // Restbits bleiben hell
  });

  let chosen = mask;
  if (chosen < 0 || chosen > 7) {
    let best = Infinity;
    for (let k = 0; k < 8; k++) {
      applyMask(modules, isFunction, k);
      drawFormat(modules, isFunction, ecc, k);
      const p = penalty(modules);
      if (p < best) {
        best = p;
        chosen = k;
      }
      applyMask(modules, isFunction, k); // XOR rückgängig
    }
  }
  applyMask(modules, isFunction, chosen);
  drawFormat(modules, isFunction, ecc, chosen);

  return { version, size: modules.length, ecc, mask: chosen, mode, modules };
}

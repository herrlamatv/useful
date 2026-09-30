/* QR-ext - by herrlamatv
 * Matrix -> Formen -> Canvas oder SVG.
 *
 * shapes() übersetzt die Module in eine Liste abgerundeter Rechtecke (in
 * Moduleinheiten, Rand schon eingerechnet). Canvas und SVG zeichnen beide
 * genau diese Liste - so sehen Vorschau, PNG und SVG immer gleich aus.
 */

import { alignmentPositions } from './qr.js';

export const QUIET_ZONE = 4;
export const DOT = 0.86;
export const FINDER_R = 0.3;

/** Liegt (x, y) in einem der drei Finder-Muster (7×7)? */
function inFinder(x, y, size) {
  return (x < 7 && y < 7) || (x >= size - 7 && y < 7) || (x < 7 && y >= size - 7);
}

/** Mittelpunkte der Ausrichtungsmuster (5×5), ohne die in den Finder-Ecken. */
function alignmentCenters(version) {
  const pos = alignmentPositions(version);
  const last = pos.length - 1;
  const out = [];
  pos.forEach((cy, i) =>
    pos.forEach((cx, j) => {
      if ((i === 0 && j === 0) || (i === 0 && j === last) || (i === last && j === 0)) return;
      out.push([cx, cy]);
    })
  );
  return out;
}

/**
 * @returns { total, shapes: { x, y, w, h, r, hole? }[] }
 *   total = Kantenlänge inkl. Rand in Modulen
 *   hole  = ausgesparte Fläche (für die Finder-Ringe)
 */
export function shapes(qr, opts = {}) {
  const { margin = true, rounded = false } = opts;
  const n = qr.size;
  const q = margin ? QUIET_ZONE : 0;
  const out = [];

  if (!rounded) {
    // Waagerechte Läufe zusammenfassen - weniger Formen, keine Haarrisse.
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; ) {
        if (!qr.modules[y][x]) {
          x++;
          continue;
        }
        let end = x;
        while (end < n && qr.modules[y][end]) end++;
        out.push({ x: x + q, y: y + q, w: end - x, h: 1, r: 0 });
        x = end;
      }
    }
    return { total: n + 2 * q, shapes: out };
  }

  // Runde Variante: Daten als Punkte. Finder und Ausrichtung bleiben fast
  // eckig - stärker gerundet erkennen viele Scanner sie nicht mehr
  // (gemessen: Radius 0.5 Modul lässt OpenCV schon scheitern, 0.3 nicht).
  for (const [fx, fy] of [
    [0, 0],
    [n - 7, 0],
    [0, n - 7]
  ]) {
    out.push({ x: fx + q, y: fy + q, w: 7, h: 7, r: FINDER_R, hole: { x: fx + q + 1, y: fy + q + 1, w: 5, h: 5, r: FINDER_R * 0.6 } });
    out.push({ x: fx + q + 2, y: fy + q + 2, w: 3, h: 3, r: FINDER_R * 0.5 });
  }
  const aligned = new Set();
  for (const [cx, cy] of alignmentCenters(qr.version)) {
    out.push({ x: cx - 2 + q, y: cy - 2 + q, w: 5, h: 5, r: FINDER_R, hole: { x: cx - 1 + q, y: cy - 1 + q, w: 3, h: 3, r: FINDER_R * 0.6 } });
    out.push({ x: cx + q + 0.1, y: cy + q + 0.1, w: 0.8, h: 0.8, r: 0.4 });
    for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) aligned.add((cy + dy) * n + cx + dx);
  }
  const d = DOT;
  const off = (1 - d) / 2;
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      if (!qr.modules[y][x] || inFinder(x, y, n) || aligned.has(y * n + x)) continue;
      out.push({ x: x + q + off, y: y + q + off, w: d, h: d, r: d / 2 });
    }
  }
  return { total: n + 2 * q, shapes: out };
}

/* ------------------------------- Canvas ------------------------------- */

/**
 * Auf einen 2D-Kontext zeichnen (px × px). Gerade Kanten werden auf ganze
 * Pixel gerundet, damit zwischen Modulen keine hellen Linien entstehen.
 */
export function drawToContext(ctx, qr, px, { margin = true, rounded = false, fg = '#000000', bg = '#ffffff' } = {}) {
  const { total, shapes: list } = shapes(qr, { margin, rounded });
  const scale = px / total;
  const snap = (v) => (rounded ? v * scale : Math.round(v * scale));

  ctx.clearRect(0, 0, px, px);
  if (bg && bg !== 'transparent') {
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, px, px);
  }
  ctx.fillStyle = fg;

  const addRect = (s) => {
    const x0 = snap(s.x);
    const y0 = snap(s.y);
    const w = snap(s.x + s.w) - x0;
    const h = snap(s.y + s.h) - y0;
    if (s.r > 0 && ctx.roundRect) ctx.roundRect(x0, y0, w, h, s.r * scale);
    else ctx.rect(x0, y0, w, h);
  };

  if (!rounded) {
    ctx.beginPath();
    list.forEach(addRect);
    ctx.fill();
    return;
  }
  for (const s of list) {
    ctx.beginPath();
    addRect(s);
    if (s.hole) addRect(s.hole);
    ctx.fill(s.hole ? 'evenodd' : 'nonzero');
  }
}

/* -------------------------------- SVG --------------------------------- */

const num = (v) => String(Math.round(v * 1000) / 1000);

function rectPath({ x, y, w, h, r }) {
  if (!r) return 'M' + num(x) + ' ' + num(y) + 'h' + num(w) + 'v' + num(h) + 'h' + num(-w) + 'z';
  const rr = Math.min(r, w / 2, h / 2);
  const a = (dx, dy) => 'a' + num(rr) + ' ' + num(rr) + ' 0 0 1 ' + num(dx) + ' ' + num(dy);
  return (
    'M' + num(x + rr) + ' ' + num(y) +
    'h' + num(w - 2 * rr) + a(rr, rr) +
    'v' + num(h - 2 * rr) + a(-rr, rr) +
    'h' + num(-(w - 2 * rr)) + a(-rr, -rr) +
    'v' + num(-(h - 2 * rr)) + a(rr, -rr) + 'z'
  );
}

const escAttr = (s) => String(s).replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');

/** Eigenständige SVG-Datei. `px` = angezeigte Größe (skaliert verlustfrei). */
export function toSvg(qr, { margin = true, rounded = false, fg = '#000000', bg = '#ffffff', px = 512 } = {}) {
  const { total, shapes: list } = shapes(qr, { margin, rounded });
  const d = list.map((s) => rectPath(s) + (s.hole ? rectPath(s.hole) : '')).join('');
  const parts = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<svg xmlns="http://www.w3.org/2000/svg" width="' + px + '" height="' + px + '" viewBox="0 0 ' + total + ' ' + total + '"' +
      (rounded ? '' : ' shape-rendering="crispEdges"') + '>'
  ];
  if (bg && bg !== 'transparent') parts.push('<rect width="100%" height="100%" fill="' + escAttr(bg) + '"/>');
  parts.push('<path fill="' + escAttr(fg) + '" fill-rule="evenodd" d="' + d + '"/>');
  parts.push('</svg>');
  return parts.join('\n') + '\n';
}

/* ------------------------------ Farben -------------------------------- */

function channel(c) {
  const v = c / 255;
  return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
}

export function luminance(hex) {
  const m = /^#?([0-9a-f]{6})$/i.exec(String(hex).trim());
  if (!m) return 1;
  const n = parseInt(m[1], 16);
  return 0.2126 * channel((n >> 16) & 255) + 0.7152 * channel((n >> 8) & 255) + 0.0722 * channel(n & 255);
}

/** Kontrastverhältnis nach WCAG (1 … 21). */
export function contrast(a, b) {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/**
 * Warnung für schlecht scanbare Farben:
 *   'inverted' = Vordergrund heller als Hintergrund (viele Scanner scheitern)
 *   'low'      = zu wenig Kontrast
 */
export function colorWarning(fg, bg) {
  if (bg === 'transparent') return null;
  if (luminance(fg) > luminance(bg)) return 'inverted';
  if (contrast(fg, bg) < 3) return 'low';
  return null;
}

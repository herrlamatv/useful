I18N.add({
  en: {
    title: 'Fractal Tree',
    back: '← Overview',
    heading: '🌳 Fractal Tree',
    angle: 'Angle',
    depth: 'Depth',
    factor: 'Length factor',
    thick: 'Trunk width',
    leaves: 'Leaves',
    green: 'green',
    pink: 'pink (blossom)',
    autumn: 'autumn',
    wind: 'Wind',
    random: 'Random',
    sym: 'Symmetric',
    hint: '“Random” = new variation per branch'
  },
  de: {
    title: 'Fraktal-Baum',
    back: '← Übersicht',
    heading: '🌳 Fraktal-Baum',
    angle: 'Winkel',
    depth: 'Tiefe',
    factor: 'Längenfaktor',
    thick: 'Stammdicke',
    leaves: 'Blätter',
    green: 'grün',
    pink: 'rosa (Blüte)',
    autumn: 'Herbst',
    wind: 'Wind',
    random: 'Zufällig',
    sym: 'Symmetrisch',
    hint: '„Zufällig“ = neue Variation pro Ast'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

let W, H, dpr;
let seed = 1, variation = 0;

const trunkColor = [90, 58, 34];
const leafColors = {
  green: [[88, 180, 70], [150, 220, 90]],
  pink: [[240, 150, 190], [255, 205, 225]],
  autumn: [[220, 90, 30], [250, 190, 50]]
};

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

// stable pseudo-random value per branch id (-1..1)
function rand(id, k) {
  let h = Math.imul(id ^ seed, 2654435761) ^ Math.imul(k + 1, 1597334677);
  h = Math.imul(h ^ h >>> 15, 2246822519);
  h ^= h >>> 13;
  return ((h >>> 0) / 4294967296) * 2 - 1;
}

const mix = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));

let segs, tips;
function build(x, y, len, ang, level, id, p, t) {
  // wind: sway grows towards the tips
  const lv = level / p.depth;
  if (p.wind) ang += (Math.sin(t * 1.3 + level * 0.45) * 0.6 + Math.sin(t * 2.9 + id * 0.01) * 0.25) * 0.035 * lv * lv * 3;
  const x2 = x + Math.cos(ang) * len, y2 = y + Math.sin(ang) * len;
  segs[level].push(x, y, x2, y2);
  if (level + 1 >= p.depth) { tips.push(x2, y2); return; }
  for (let s = -1; s <= 1; s += 2) {
    const cid = id * 2 + (s > 0 ? 1 : 0);
    const a = ang + s * p.angle * (1 + variation * rand(cid, 0) * 0.5);
    const l = len * p.factor * (1 + variation * rand(cid, 1) * 0.25);
    build(x2, y2, l, a, level + 1, cid, p, t);
  }
}

function frame(now) {
  const t = now / 1000;
  const p = {
    angle: +$('angle').value * Math.PI / 180,
    depth: +$('depth').value,
    factor: +$('factor').value,
    thick: +$('thick').value,
    wind: $('wind').checked
  };
  const [leafA, leafB] = leafColors[$('leaves').value];

  segs = Array.from({ length: p.depth }, () => []);
  tips = [];
  const len = Math.min(H * 0.3, H * 0.75 * (1 - p.factor));
  build(W / 2, H - 20, len, -Math.PI / 2, 0, 1, p, t);

  ctx.fillStyle = '#070b14';
  ctx.fillRect(0, 0, W, H);
  // ground glow
  const g = ctx.createRadialGradient(W / 2, H, 0, W / 2, H, Math.min(W, H) * 0.5);
  g.addColorStop(0, 'rgba(88,166,255,0.15)'); g.addColorStop(1, 'rgba(88,166,255,0)');
  ctx.fillStyle = g; ctx.fillRect(0, 0, W, H);

  // one stroke per depth level (same width & color)
  ctx.lineCap = 'round';
  for (let lv = 0; lv < p.depth; lv++) {
    const k = p.depth > 1 ? lv / (p.depth - 1) : 0;
    const col = k < 0.6 ? mix(trunkColor, [120, 90, 50], k / 0.6) : mix([120, 90, 50], leafA, (k - 0.6) / 0.4);
    ctx.strokeStyle = `rgb(${col})`;
    ctx.lineWidth = Math.max(0.6, p.thick * Math.pow(0.68, lv));
    const s = segs[lv];
    ctx.beginPath();
    for (let i = 0; i < s.length; i += 4) { ctx.moveTo(s[i], s[i + 1]); ctx.lineTo(s[i + 2], s[i + 3]); }
    ctx.stroke();
  }

  // leaves / blossoms at the tips
  if (p.depth >= 4) {
    const r = Math.max(1.5, 3.5 - p.depth * 0.15);
    for (let pass = 0; pass < 2; pass++) {
      ctx.fillStyle = `rgba(${pass ? leafB : leafA}, 0.8)`;
      ctx.beginPath();
      for (let i = pass; i < tips.length / 2; i += 2) {
        const x = tips[i * 2], y = tips[i * 2 + 1];
        ctx.moveTo(x + r, y); ctx.arc(x, y, r, 0, Math.PI * 2);
      }
      ctx.fill();
    }
  }

  requestAnimationFrame(frame);
}

addEventListener('resize', resize);
$('random').onclick = () => { seed = Math.random() * 1e9 | 0; variation = 1; };
$('sym').onclick = () => { variation = 0; };

resize();
requestAnimationFrame(frame);

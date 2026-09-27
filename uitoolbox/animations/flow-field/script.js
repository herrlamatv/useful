I18N.add({
  en: {
    title: 'Flow Field',
    back: '← Overview',
    heading: '🌊 Flow Field',
    palette: 'Palette',
    ocean: 'Ocean',
    sunset: 'Sunset',
    forest: 'Forest',
    scale: 'Noise scale',
    speed: 'Speed',
    count: 'Particles',
    new: 'New',
    hint: 'Lines build up over time'
  },
  de: {
    title: 'Flow Field',
    back: '← Übersicht',
    heading: '🌊 Flow Field',
    palette: 'Farbpalette',
    ocean: 'Ozean',
    sunset: 'Sonnenuntergang',
    forest: 'Wald',
    scale: 'Noise-Skalierung',
    speed: 'Tempo',
    count: 'Partikel',
    new: 'Neu',
    hint: 'Linien sammeln sich mit der Zeit an'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

let W, H, dpr;
let particles = [];
let z = 0;

const palettes = {
  ozean: ['#0b3d91', '#1f6feb', '#58a6ff', '#79c0ff', '#a5f3fc'],
  sonne: ['#ff6b6b', '#ff9f43', '#feca57', '#ff4d8d', '#c56cf0'],
  wald: ['#2d6a4f', '#40916c', '#74c69d', '#b7e4c7', '#d4a373'],
  neon: ['#ff00e5', '#00f0ff', '#39ff14', '#fffb00', '#8a2be2'],
  mono: ['#ffffff', '#c9d1d9', '#8b949e', '#6e7681', '#e6edf3']
};

// --- seeded 3D Perlin noise ---
function mulberry32(a) {
  return () => {
    a |= 0; a = a + 0x6D2B79F5 | 0;
    let t = Math.imul(a ^ a >>> 15, 1 | a);
    t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t;
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  };
}

const perm = new Uint8Array(512);
function seedNoise(seed) {
  const rnd = mulberry32(seed);
  const p = [...Array(256).keys()];
  for (let i = 255; i > 0; i--) { const j = rnd() * (i + 1) | 0; [p[i], p[j]] = [p[j], p[i]]; }
  for (let i = 0; i < 512; i++) perm[i] = p[i & 255];
}

const fade = t => t * t * t * (t * (t * 6 - 15) + 10);
const lerp = (a, b, t) => a + (b - a) * t;
function grad(h, x, y, z) {
  h &= 15;
  const u = h < 8 ? x : y, v = h < 4 ? y : (h === 12 || h === 14 ? x : z);
  return ((h & 1) ? -u : u) + ((h & 2) ? -v : v);
}
function noise(x, y, z) {
  const X = Math.floor(x) & 255, Y = Math.floor(y) & 255, Z = Math.floor(z) & 255;
  x -= Math.floor(x); y -= Math.floor(y); z -= Math.floor(z);
  const u = fade(x), v = fade(y), w = fade(z);
  const A = perm[X] + Y, AA = perm[A] + Z, AB = perm[A + 1] + Z;
  const B = perm[X + 1] + Y, BA = perm[B] + Z, BB = perm[B + 1] + Z;
  return lerp(
    lerp(lerp(grad(perm[AA], x, y, z), grad(perm[BA], x - 1, y, z), u),
         lerp(grad(perm[AB], x, y - 1, z), grad(perm[BB], x - 1, y - 1, z), u), v),
    lerp(lerp(grad(perm[AA + 1], x, y, z - 1), grad(perm[BA + 1], x - 1, y, z - 1), u),
         lerp(grad(perm[AB + 1], x, y - 1, z - 1), grad(perm[BB + 1], x - 1, y - 1, z - 1), u), v),
    w); // roughly -1..1
}

// --- particles ---
function makeParticle() {
  const x = Math.random() * W, y = Math.random() * H;
  return { x, y, px: x, py: y, c: Math.random() * 5 | 0, life: 100 + Math.random() * 300 };
}

function syncCount() {
  const n = +$('count').value;
  while (particles.length < n) particles.push(makeParticle());
  particles.length = n;
}

function clear() {
  ctx.globalAlpha = 1;
  ctx.fillStyle = '#070b14';
  ctx.fillRect(0, 0, W, H);
}

function reset() {
  seedNoise(Math.random() * 1e9 | 0);
  z = 0;
  particles = [];
  syncCount();
  clear();
}

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  clear();
  for (const p of particles) Object.assign(p, makeParticle());
}

let last = performance.now();
function frame(now) {
  const dt = Math.min((now - last) / 16.67, 3); last = now;
  const scale = +$('scale').value;
  const speed = +$('speed').value * dt;
  const colors = palettes[$('palette').value];
  z += 0.002 * dt;

  // move particles, collect segments per color (one stroke per color)
  const paths = colors.map(() => new Path2D());
  for (const p of particles) {
    const a = noise(p.x * scale, p.y * scale, z) * Math.PI * 4;
    p.px = p.x; p.py = p.y;
    p.x += Math.cos(a) * speed; p.y += Math.sin(a) * speed;
    p.life -= dt;
    if (p.life <= 0 || p.x < 0 || p.x > W || p.y < 0 || p.y > H) { Object.assign(p, makeParticle()); continue; }
    const path = paths[p.c];
    path.moveTo(p.px, p.py); path.lineTo(p.x, p.y);
  }

  ctx.globalAlpha = 0.08;
  ctx.lineWidth = 0.8;
  paths.forEach((path, i) => { ctx.strokeStyle = colors[i]; ctx.stroke(path); });
  ctx.globalAlpha = 1;

  requestAnimationFrame(frame);
}

addEventListener('resize', resize);
$('count').oninput = syncCount;
$('new').onclick = reset;
$('palette').onchange = clear;

resize();
reset();
requestAnimationFrame(frame);

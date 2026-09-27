I18N.add({
  en: {
    title: 'Fireworks',
    back: '← Overview',
    heading: '🎆 Fireworks',
    count: 'Particles per burst',
    gravity: 'Gravity',
    auto: 'Auto',
    hint: 'Click = launch rocket<br>Shapes: sphere, ring, heart'
  },
  de: {
    title: 'Feuerwerk',
    back: '← Übersicht',
    heading: '🎆 Feuerwerk',
    count: 'Partikel pro Explosion',
    gravity: 'Schwerkraft',
    auto: 'Automatik',
    hint: 'Klick = Rakete starten<br>Formen: Kugel, Ring, Herz'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

let W, H, dpr;
let rockets = [];
let sparks = [];
const MAX_SPARKS = 8000;

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = '#070b14'; ctx.fillRect(0, 0, W, H);
}

function launch(tx, ty) {
  const x = tx + (Math.random() - 0.5) * W * 0.2, y = H + 5;
  const dist = Math.hypot(tx - x, ty - y);
  rockets.push({ x, y, px: x, py: y, sx: x, sy: y, tx, ty, dist, speed: 2, hue: Math.random() * 360 });
}

function addSpark(x, y, vx, vy, hue, decay, size) {
  if (sparks.length < MAX_SPARKS) sparks.push({ x, y, px: x, py: y, vx, vy, hue, a: 1, decay, size });
}

// explosion shapes: return list of velocity vectors
const shapes = {
  kugel(n) {
    const out = [];
    for (let i = 0; i < n; i++) {
      const a = Math.random() * Math.PI * 2, s = Math.random() * 6 + 0.5;
      out.push([Math.cos(a) * s, Math.sin(a) * s]);
    }
    return out;
  },
  ring(n) {
    const out = [], s = 5 + Math.random() * 2, tilt = Math.random() * 0.6 + 0.4, rot = Math.random() * Math.PI;
    for (let i = 0; i < n; i++) {
      const a = i / n * Math.PI * 2;
      const x = Math.cos(a) * s, y = Math.sin(a) * s * tilt;
      out.push([x * Math.cos(rot) - y * Math.sin(rot), x * Math.sin(rot) + y * Math.cos(rot)]);
    }
    return out;
  },
  herz(n) {
    const out = [], k = 0.4 + Math.random() * 0.1;
    for (let i = 0; i < n; i++) {
      const t = i / n * Math.PI * 2;
      const x = 16 * Math.sin(t) ** 3;
      const y = -(13 * Math.cos(t) - 5 * Math.cos(2 * t) - 2 * Math.cos(3 * t) - Math.cos(4 * t));
      out.push([x * k, y * k]);
    }
    return out;
  }
};

function explode(r) {
  const names = Object.keys(shapes);
  const shape = names[Math.random() * names.length | 0];
  const n = +$('count').value;
  const twoTone = Math.random() < 0.3;
  for (const [vx, vy] of shapes[shape](n)) {
    const hue = twoTone && Math.random() < 0.5 ? r.hue + 150 : r.hue + (Math.random() - 0.5) * 30;
    addSpark(r.x, r.y, vx, vy, hue, 0.008 + Math.random() * 0.012, Math.random() * 1.5 + 1);
  }
}

let last = performance.now();
function frame(now) {
  const dt = Math.min((now - last) / 16.67, 3); last = now;
  const g = +$('gravity').value;
  const fric = Math.pow(0.97, dt);

  if ($('auto').checked && Math.random() < 0.025 * dt)
    launch(W * (0.15 + Math.random() * 0.7), H * (0.1 + Math.random() * 0.4));

  // fade previous frame -> glowing trails
  ctx.globalCompositeOperation = 'source-over';
  ctx.fillStyle = 'rgba(7, 11, 20, 0.2)';
  ctx.fillRect(0, 0, W, H);
  ctx.globalCompositeOperation = 'lighter';
  ctx.lineCap = 'round';

  // rockets
  for (let i = rockets.length - 1; i >= 0; i--) {
    const r = rockets[i];
    r.px = r.x; r.py = r.y;
    r.speed *= Math.pow(1.05, dt);
    const dx = r.tx - r.sx, dy = r.ty - r.sy;
    const traveled = Math.hypot(r.x - r.sx, r.y - r.sy) + r.speed * dt;
    const f = Math.min(1, traveled / r.dist);
    r.x = r.sx + dx * f; r.y = r.sy + dy * f;
    addSpark(r.x, r.y, (Math.random() - 0.5) * 0.6, Math.random() * 0.8, 40, 0.04, 1);
    ctx.strokeStyle = `hsl(${r.hue}, 100%, 75%)`;
    ctx.lineWidth = 2.5;
    ctx.beginPath(); ctx.moveTo(r.px, r.py); ctx.lineTo(r.x, r.y); ctx.stroke();
    if (f >= 1) { explode(r); rockets.splice(i, 1); }
  }

  // sparks
  for (let i = sparks.length - 1; i >= 0; i--) {
    const p = sparks[i];
    p.px = p.x; p.py = p.y;
    p.vx *= fric; p.vy *= fric; p.vy += g * dt;
    p.x += p.vx * dt; p.y += p.vy * dt;
    p.a -= p.decay * dt;
    if (p.a <= 0) { sparks[i] = sparks[sparks.length - 1]; sparks.pop(); continue; }
    ctx.strokeStyle = `hsla(${p.hue}, 100%, ${55 + p.a * 20}%, ${p.a})`;
    ctx.lineWidth = p.size;
    ctx.beginPath(); ctx.moveTo(p.px, p.py); ctx.lineTo(p.x, p.y); ctx.stroke();
  }

  requestAnimationFrame(frame);
}

addEventListener('resize', resize);
canvas.addEventListener('pointerdown', e => launch(e.clientX, e.clientY));

resize();
requestAnimationFrame(frame);

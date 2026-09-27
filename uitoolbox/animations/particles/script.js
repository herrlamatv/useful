I18N.add({
  en: {
    title: 'Particle Network',
    back: '← Overview',
    heading: '✨ Particles',
    count: 'Count',
    dist: 'Link distance',
    speed: 'Speed',
    color: 'Colour',
    rainbow: 'Rainbow',
    mouse: 'Mouse',
    attract: 'attract',
    repel: 'repel',
    off: 'off',
    hint: 'Click = add particles'
  },
  de: {
    title: 'Partikel-Netzwerk',
    back: '← Übersicht',
    heading: '✨ Partikel',
    count: 'Anzahl',
    dist: 'Verbindung',
    speed: 'Tempo',
    color: 'Farbe',
    rainbow: 'Regenbogen',
    mouse: 'Maus',
    attract: 'anziehen',
    repel: 'abstoßen',
    off: 'aus',
    hint: 'Klick = Partikel hinzufügen'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

let W, H, dpr;
let particles = [];
const mouse = { x: -9999, y: -9999 };

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function makeParticle(x = Math.random() * W, y = Math.random() * H) {
  const a = Math.random() * Math.PI * 2, s = Math.random() * 0.6 + 0.2;
  return { x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s, r: Math.random() * 2 + 1, hue: Math.random() * 360 };
}

function syncCount() {
  const n = +$('count').value;
  while (particles.length < n) particles.push(makeParticle());
  particles.length = n;
}

function hexToRgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [n >> 16, (n >> 8) & 255, n & 255];
}

let t = 0;
function frame() {
  t++;
  const speed = +$('speed').value;
  const maxD = +$('dist').value;
  const rainbow = $('rainbow').checked;
  const [cr, cg, cb] = hexToRgb($('color').value);
  const mode = $('mode').value;

  ctx.fillStyle = 'rgba(7, 11, 20, 0.35)';
  ctx.fillRect(0, 0, W, H);

  for (const p of particles) {
    // mouse interaction
    if (mode !== 'none') {
      const dx = mouse.x - p.x, dy = mouse.y - p.y, d = Math.hypot(dx, dy);
      if (d < 180 && d > 1) {
        const f = (mode === 'attract' ? 0.03 : -0.08) * (1 - d / 180);
        p.vx += dx / d * f; p.vy += dy / d * f;
      }
    }
    // keep velocity in a pleasant range
    const v = Math.hypot(p.vx, p.vy);
    if (v > 2.5) { p.vx *= 2.5 / v; p.vy *= 2.5 / v; }
    if (v < 0.15) { p.vx *= 1.05; p.vy *= 1.05; }

    p.x += p.vx * speed; p.y += p.vy * speed;
    if (p.x < 0 || p.x > W) { p.vx *= -1; p.x = Math.max(0, Math.min(W, p.x)); }
    if (p.y < 0 || p.y > H) { p.vy *= -1; p.y = Math.max(0, Math.min(H, p.y)); }
  }

  // connections
  ctx.lineWidth = 1;
  for (let i = 0; i < particles.length; i++) {
    const a = particles[i];
    for (let j = i + 1; j < particles.length; j++) {
      const b = particles[j];
      const dx = a.x - b.x, dy = a.y - b.y;
      if (Math.abs(dx) > maxD || Math.abs(dy) > maxD) continue;
      const d = Math.hypot(dx, dy);
      if (d < maxD) {
        const alpha = 1 - d / maxD;
        ctx.strokeStyle = rainbow
          ? `hsla(${(a.hue + t) % 360}, 90%, 60%, ${alpha})`
          : `rgba(${cr}, ${cg}, ${cb}, ${alpha})`;
        ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
      }
    }
    // line to mouse
    const md = Math.hypot(a.x - mouse.x, a.y - mouse.y);
    if (md < maxD * 1.3) {
      ctx.strokeStyle = `rgba(255,255,255,${(1 - md / (maxD * 1.3)) * 0.6})`;
      ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(mouse.x, mouse.y); ctx.stroke();
    }
  }

  // dots
  for (const p of particles) {
    ctx.fillStyle = rainbow ? `hsl(${(p.hue + t) % 360}, 90%, 65%)` : `rgb(${cr}, ${cg}, ${cb})`;
    ctx.beginPath(); ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2); ctx.fill();
  }

  requestAnimationFrame(frame);
}

addEventListener('resize', resize);
addEventListener('pointermove', e => { mouse.x = e.clientX; mouse.y = e.clientY; });
document.addEventListener('pointerleave', () => { mouse.x = mouse.y = -9999; });
canvas.addEventListener('click', e => {
  for (let i = 0; i < 8; i++) particles.push(makeParticle(e.clientX, e.clientY));
  $('count').value = particles.length;
});
$('count').oninput = syncCount;

resize();
syncCount();
frame();

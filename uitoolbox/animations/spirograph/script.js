I18N.add({
  en: {
    title: 'Spirograph',
    back: '← Overview',
    heading: '🌀 Spirograph',
    mode: 'Mode',
    R: 'R (fixed circle)',
    r: 'r (rolling circle)',
    d: 'd (pen offset)',
    speed: 'Speed',
    width: 'Line width',
    color: 'Colour',
    rainbow: 'Rainbow',
    gears: 'Show circles',
    redraw: 'Redraw',
    random: 'Random',
    infoLiss: 'Frequencies {a} : {b}',
    infoLoops: '{n} loops · R/r = {R}/{r}'
  },
  de: {
    title: 'Spirograph',
    back: '← Übersicht',
    heading: '🌀 Spirograph',
    mode: 'Modus',
    R: 'R (fester Kreis)',
    r: 'r (rollender Kreis)',
    d: 'd (Stiftabstand)',
    speed: 'Tempo',
    width: 'Linienbreite',
    color: 'Farbe',
    rainbow: 'Regenbogen',
    gears: 'Kreise anzeigen',
    redraw: 'Neu zeichnen',
    random: 'Zufall',
    infoLiss: 'Frequenzen {a} : {b}',
    infoLoops: '{n} Umläufe · R/r = {R}/{r}'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

// offscreen canvas holds the drawn curve; gears are drawn on top each frame
const trace = document.createElement('canvas');
const tctx = trace.getContext('2d');

let W, H, dpr;
let t = 0, tEnd = 0, scale = 1, cfg;
const DT = 0.01;

const gcd = (a, b) => b ? gcd(b, a % b) : a;

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = trace.width = W * dpr; canvas.height = trace.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  tctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  restart();
}

// pen position (unscaled, relative to center) + rolling circle center
function point(t) {
  const { mode, R, r, d } = cfg;
  if (mode === 'hypo') {
    const k = (R - r) / r;
    return { x: (R - r) * Math.cos(t) + d * Math.cos(k * t), y: (R - r) * Math.sin(t) - d * Math.sin(k * t),
             cx: (R - r) * Math.cos(t), cy: (R - r) * Math.sin(t) };
  }
  if (mode === 'epi') {
    const k = (R + r) / r;
    return { x: (R + r) * Math.cos(t) - d * Math.cos(k * t), y: (R + r) * Math.sin(t) - d * Math.sin(k * t),
             cx: (R + r) * Math.cos(t), cy: (R + r) * Math.sin(t) };
  }
  // Lissajous: frequencies a:b from R:r, phase from d
  return { x: Math.sin(cfg.a * t + cfg.phase), y: Math.sin(cfg.b * t) };
}

function restart() {
  const mode = $('mode').value, R = +$('R').value, r = +$('r').value, d = +$('d').value;
  const g = gcd(R, r);
  cfg = { mode, R, r, d, a: R / g, b: r / g, phase: d / 150 * Math.PI / 2 };
  const fit = Math.min(W, H) * (W < 600 ? 0.3 : 0.42);
  if (mode === 'liss') {
    // reduce huge frequency ratios to keep it readable
    while (cfg.a > 15 || cfg.b > 15) { cfg.a = Math.max(1, Math.round(cfg.a / 2)); cfg.b = Math.max(1, Math.round(cfg.b / 2)); }
    const g2 = gcd(cfg.a, cfg.b); cfg.a /= g2; cfg.b /= g2;
    tEnd = Math.PI * 2;
    scale = fit;
  } else {
    tEnd = Math.PI * 2 * r / g;
    const ext = mode === 'hypo' ? Math.max(R, Math.abs(R - r) + d) : Math.max(R + 2 * r, R + r + d);
    scale = fit / ext;
  }
  t = 0;
  tctx.clearRect(0, 0, W, H);
  updateInfo();
}

function updateInfo() {
  const { mode, R, r } = cfg;
  $('info').textContent = mode === 'liss'
    ? I18N.t('infoLiss', { a: cfg.a, b: cfg.b })
    : I18N.t('infoLoops', { n: r / gcd(R, r), R, r });
}

function frame() {
  const cx = W / 2, cy = H / 2;
  const steps = cfg.mode === 'liss' ? +$('speed').value * 2 : +$('speed').value * 20;
  const rainbow = $('rainbow').checked;

  // draw next chunk of the curve
  if (t < tEnd) {
    const t1 = Math.min(tEnd, t + steps * DT * (cfg.mode === 'liss' ? 0.1 : 1));
    const n = Math.ceil((t1 - t) / (DT * (cfg.mode === 'liss' ? 0.1 : 1)));
    tctx.lineWidth = +$('width').value;
    tctx.lineCap = tctx.lineJoin = 'round';
    tctx.strokeStyle = rainbow ? `hsl(${(t / tEnd * 720) % 360}, 90%, 65%)` : $('color').value;
    tctx.beginPath();
    let p = point(t);
    tctx.moveTo(cx + p.x * scale, cy + p.y * scale);
    for (let i = 1; i <= n; i++) {
      p = point(t + (t1 - t) * i / n);
      tctx.lineTo(cx + p.x * scale, cy + p.y * scale);
    }
    tctx.stroke();
    t = t1;
  }

  ctx.fillStyle = '#070b14';
  ctx.fillRect(0, 0, W, H);
  ctx.save(); ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.drawImage(trace, 0, 0); ctx.restore();

  // gears
  if ($('gears').checked && t < tEnd) {
    const p = point(t);
    const px = cx + p.x * scale, py = cy + p.y * scale;
    ctx.lineWidth = 1;
    ctx.strokeStyle = 'rgba(255,255,255,0.35)';
    if (cfg.mode === 'liss') {
      ctx.setLineDash([4, 6]);
      ctx.beginPath(); ctx.moveTo(px, cy - scale); ctx.lineTo(px, cy + scale); ctx.moveTo(cx - scale, py); ctx.lineTo(cx + scale, py); ctx.stroke();
      ctx.setLineDash([]);
      ctx.strokeRect(cx - scale, cy - scale, scale * 2, scale * 2);
    } else {
      const gx = cx + p.cx * scale, gy = cy + p.cy * scale;
      ctx.beginPath(); ctx.arc(cx, cy, cfg.R * scale, 0, Math.PI * 2); ctx.stroke();
      ctx.strokeStyle = 'rgba(88,166,255,0.7)';
      ctx.beginPath(); ctx.arc(gx, gy, cfg.r * scale, 0, Math.PI * 2); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(gx, gy); ctx.lineTo(px, py); ctx.stroke();
      ctx.fillStyle = 'rgba(88,166,255,0.9)';
      ctx.beginPath(); ctx.arc(gx, gy, 2.5, 0, Math.PI * 2); ctx.fill();
    }
    ctx.fillStyle = '#fff';
    ctx.beginPath(); ctx.arc(px, py, 4, 0, Math.PI * 2); ctx.fill();
  }

  requestAnimationFrame(frame);
}

function randomize() {
  const modes = ['hypo', 'hypo', 'epi', 'liss'];
  $('mode').value = modes[Math.random() * modes.length | 0];
  $('R').value = 60 + Math.random() * 140 | 0;
  $('r').value = 10 + Math.random() * 90 | 0;
  $('d').value = 10 + Math.random() * 130 | 0;
  restart();
}

addEventListener('resize', resize);
addEventListener('langchange', updateInfo);
for (const id of ['mode', 'R', 'r', 'd']) $(id).oninput = restart;
$('rainbow').onchange = restart;
$('color').oninput = restart;
$('redraw').onclick = restart;
$('random').onclick = randomize;

resize();
requestAnimationFrame(frame);

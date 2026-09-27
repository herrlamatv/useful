I18N.add({
  en: {
    title: 'Starfield',
    back: '← Overview',
    heading: '🌌 Starfield',
    speed: 'Speed',
    count: 'Number of stars',
    color: 'Colour',
    white: 'white',
    colorful: 'colourful',
    hint: 'Mouse = steer direction<br>Hold space = hyperspace'
  },
  de: {
    title: 'Sternenfeld',
    back: '← Übersicht',
    heading: '🌌 Sternenfeld',
    speed: 'Tempo',
    count: 'Anzahl Sterne',
    color: 'Farbe',
    white: 'weiß',
    colorful: 'bunt',
    hint: 'Maus = Richtung lenken<br>Leertaste halten = Hyperraum'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

let W, H, dpr;
let stars = [];
const mouse = { x: null, y: null };
const vp = { x: 0, y: 0 };   // current vanishing point
let warp = 1, warpTarget = 1;

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (!vp.x) { vp.x = W / 2; vp.y = H / 2; }
}

function makeStar(z = Math.random()) {
  return { x: (Math.random() * 2 - 1) * W, y: (Math.random() * 2 - 1) * H, z: Math.max(z, 0.02), hue: Math.random() * 360 };
}

function syncCount() {
  const n = +$('count').value;
  while (stars.length < n) stars.push(makeStar());
  stars.length = n;
}

// project a 3D point onto the screen around the vanishing point
const proj = (s, z) => [vp.x + s.x / z * 0.5, vp.y + s.y / z * 0.5];

let last = performance.now();
function frame(now) {
  const dt = Math.min((now - last) / 16.67, 3); last = now;
  const colorful = $('color').value === 'color';

  // ease warp factor towards target (hyperspace ramp)
  warp += (warpTarget - warp) * (warpTarget > warp ? 0.03 : 0.05) * dt;
  const v = +$('speed').value * 0.004 * warp * dt;

  // steer vanishing point towards mouse
  const tx = mouse.x ?? W / 2, ty = mouse.y ?? H / 2;
  vp.x += (tx - vp.x) * 0.05 * dt; vp.y += (ty - vp.y) * 0.05 * dt;

  // longer trails in hyperspace
  ctx.fillStyle = `rgba(7, 11, 20, ${1 / (1 + (warp - 1) * 0.25)})`;
  ctx.fillRect(0, 0, W, H);

  ctx.lineCap = 'round';
  const tailK = 4 + warp * 3;   // streak length in "speed units"
  for (let i = 0; i < stars.length; i++) {
    const s = stars[i];
    s.z -= v;
    const [x, y] = proj(s, s.z);
    if (s.z <= 0.02 || x < -50 || x > W + 50 || y < -50 || y > H + 50) { stars[i] = makeStar(1); continue; }
    const [px, py] = proj(s, Math.min(1, s.z + v * tailK));
    const k = 1 - s.z;
    ctx.lineWidth = 0.4 + k * 2.6;
    ctx.strokeStyle = colorful
      ? `hsla(${s.hue}, 90%, 70%, ${k})`
      : `rgba(220, 235, 255, ${k})`;
    ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(x + 0.1, y); ctx.stroke();
  }

  requestAnimationFrame(frame);
}

addEventListener('resize', resize);
addEventListener('pointermove', e => { mouse.x = e.clientX; mouse.y = e.clientY; });
document.addEventListener('pointerleave', () => { mouse.x = mouse.y = null; });
addEventListener('keydown', e => {
  if (e.code === 'Space') { e.preventDefault(); warpTarget = 14; }
});
addEventListener('keyup', e => { if (e.code === 'Space') warpTarget = 1; });
addEventListener('blur', () => { warpTarget = 1; });
$('count').oninput = syncCount;

resize();
syncCount();
requestAnimationFrame(frame);

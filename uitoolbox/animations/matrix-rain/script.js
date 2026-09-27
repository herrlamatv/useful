I18N.add({
  en: {
    title: 'Matrix Rain',
    back: '← Overview',
    settings: 'Settings',
    color: 'Colour',
    speed: 'Speed',
    fontSize: 'Font size',
    trail: 'Trail',
    chars: 'Characters',
    csMatrix: 'Katakana + digits',
    csBinary: 'Binary 0/1',
    rainbow: 'Rainbow',
    message: 'Custom text (optional)'
  },
  de: {
    title: 'Matrix Rain',
    back: '← Übersicht',
    settings: 'Einstellungen',
    color: 'Farbe',
    speed: 'Tempo',
    fontSize: 'Schriftgröße',
    trail: 'Spur',
    chars: 'Zeichen',
    csMatrix: 'Katakana + Zahlen',
    csBinary: 'Binär 0/1',
    rainbow: 'Regenbogen',
    message: 'Eigener Text (optional)'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

const CHARSETS = {
  matrix: 'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン0123456789',
  binary: '01',
  hex: '0123456789ABCDEF',
  latin: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
};

let W, H, fontSize, columns, drops;

function setup() {
  W = canvas.width = innerWidth;
  H = canvas.height = innerHeight;
  fontSize = +$('size').value;
  columns = Math.ceil(W / fontSize);
  drops = Array.from({ length: columns }, () => Math.random() * H / fontSize);
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, W, H);
}

let lastStep = 0, hueShift = 0;
function frame(now) {
  requestAnimationFrame(frame);
  // speed slider: 10 (slow) .. 100 (fast) => step interval in ms
  const interval = 110 - +$('speed').value;
  if (now - lastStep < interval) return;
  lastStep = now;
  hueShift = (hueShift + 2) % 360;

  const trail = +$('trail').value;
  ctx.fillStyle = `rgba(0, 0, 0, ${1 / trail})`;
  ctx.fillRect(0, 0, W, H);

  const chars = CHARSETS[$('charset').value];
  const msg = $('message').value.trim();
  const color = $('color').value;
  const rainbow = $('rainbow').checked;
  ctx.font = `${fontSize}px monospace`;

  for (let i = 0; i < columns; i++) {
    const y = drops[i] * fontSize;
    let ch = chars[Math.floor(Math.random() * chars.length)];
    if (msg) ch = msg[Math.floor(drops[i]) % msg.length] || ch;

    // bright head, coloured tail (the tail is produced by the fading overlay)
    ctx.fillStyle = rainbow ? `hsl(${(i * 6 + hueShift) % 360}, 100%, 60%)` : color;
    ctx.fillText(ch, i * fontSize, y - fontSize);
    ctx.fillStyle = '#e8ffe8';
    ctx.fillText(ch, i * fontSize, y);

    if (y > H && Math.random() > 0.975) drops[i] = 0;
    drops[i]++;
  }
}

addEventListener('resize', setup);
$('size').oninput = setup;
$('toggle').onclick = () => $('panel').classList.toggle('hidden');

setup();
requestAnimationFrame(frame);

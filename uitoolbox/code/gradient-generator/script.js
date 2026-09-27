I18N.add({
  en: {
    docTitle: 'Gradient Generator', back: '← Overview', heading: 'Gradient Generator',
    random: 'Random', reset: 'Reset', type: 'Type', angle: 'Angle', shape: 'Shape', position: 'Position',
    colourStop: 'Colour stop', hint: 'Click the bar = new stop, drag = move, Del = delete.',
    colour: 'Colour', alpha: 'Alpha', delStop: 'Delete stop', presets: 'Presets',
    generated: 'Generated CSS', copy: 'Copy', copied: 'Copied', error: 'Error'
  },
  de: {
    docTitle: 'Gradient Generator', back: '← Übersicht', heading: 'Gradient Generator',
    random: 'Zufall', reset: 'Reset', type: 'Typ', angle: 'Winkel', shape: 'Form', position: 'Position',
    colourStop: 'Farbstopp', hint: 'Klick auf die Leiste = neuer Stopp, ziehen = verschieben, Entf = löschen.',
    colour: 'Farbe', alpha: 'Alpha', delStop: 'Stopp löschen', presets: 'Vorlagen',
    generated: 'Generiertes CSS', copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler'
  }
});

// Preset gradients: [type, angle, stops as [hex, pos]]
const PRESETS = [
  ['linear', 135, [['#d9774b', 0], ['#e9b872', 100]]],
  ['linear', 90, [['#f83600', 0], ['#f9d423', 100]]],
  ['linear', 160, [['#0f2027', 0], ['#203a43', 50], ['#2c5364', 100]]],
  ['radial', 0, [['#fdfc47', 0], ['#24fe41', 100]]],
  ['conic', 0, [['#ff0000', 0], ['#ffff00', 17], ['#00ff00', 33], ['#00ffff', 50], ['#0000ff', 67], ['#ff00ff', 83], ['#ff0000', 100]]],
  ['linear', 45, [['#43e97b', 0], ['#38f9d7', 50], ['#4facfe', 100]]],
];

const DEFAULT = () => ({
  type: 'linear', angle: 90, shape: 'circle', pos: 'center',
  stops: [{ color: '#d9774b', alpha: 100, pos: 0 }, { color: '#e9b872', alpha: 100, pos: 100 }],
});

let state = DEFAULT();
let sel = state.stops[0];
let dragging = null;

const $ = id => document.getElementById(id);
const bar = $('bar'), handles = $('handles'), codeEl = $('code');

const hexToRgb = hex => { const n = parseInt(hex.slice(1), 16); return [n >> 16, (n >> 8) & 255, n & 255]; };
const rgbToHex = rgb => '#' + rgb.map(v => Math.round(v).toString(16).padStart(2, '0')).join('');
const colorStr = s => s.alpha >= 100 ? s.color : `rgba(${hexToRgb(s.color).join(', ')}, ${+(s.alpha / 100).toFixed(2)})`;
const sorted = () => [...state.stops].sort((a, b) => a.pos - b.pos);
const stopList = () => sorted().map(s => `${colorStr(s)} ${s.pos}%`).join(', ');

function gradientCSS() {
  const list = stopList();
  if (state.type === 'linear') return `linear-gradient(${state.angle}deg, ${list})`;
  if (state.type === 'radial') return `radial-gradient(${state.shape} at ${state.pos}, ${list})`;
  return `conic-gradient(from ${state.angle}deg at ${state.pos}, ${list})`;
}

// Interpolated colour at a position (used for new stops)
function colorAt(pos) {
  const s = sorted();
  if (pos <= s[0].pos) return { color: s[0].color, alpha: s[0].alpha };
  for (let i = 1; i < s.length; i++) {
    if (pos <= s[i].pos) {
      const a = s[i - 1], b = s[i], t = (pos - a.pos) / ((b.pos - a.pos) || 1);
      const ca = hexToRgb(a.color), cb = hexToRgb(b.color);
      return { color: rgbToHex(ca.map((v, k) => v + (cb[k] - v) * t)), alpha: Math.round(a.alpha + (b.alpha - a.alpha) * t) };
    }
  }
  const last = s[s.length - 1];
  return { color: last.color, alpha: last.alpha };
}

const posFromEvent = e => {
  const r = bar.getBoundingClientRect();
  return Math.max(0, Math.min(100, Math.round((e.clientX - r.left) / r.width * 100)));
};

function renderHandles() {
  handles.innerHTML = '';
  state.stops.forEach(s => {
    const h = document.createElement('div');
    h.className = 'handle' + (s === sel ? ' sel' : '');
    h.style.left = s.pos + '%';
    h.style.background = colorStr(s);
    h.stop = s;
    handles.appendChild(h);
  });
}

function render() {
  const g = gradientCSS();
  $('preview').style.background = g;
  $('barFill').style.background = `linear-gradient(90deg, ${stopList()})`;
  renderHandles();

  // Sync controls
  document.querySelectorAll('#typeSeg button').forEach(b => b.classList.toggle('on', b.dataset.type === state.type));
  $('angleRow').hidden = state.type === 'radial';
  $('shapeRow').hidden = state.type !== 'radial';
  $('posRow').hidden = state.type === 'linear';
  $('angle').value = state.angle; $('angleOut').textContent = state.angle + '°';
  $('shape').value = state.shape; $('pos').value = state.pos;
  $('stopColor').value = sel.color;
  $('stopAlpha').value = sel.alpha; $('alphaOut').textContent = sel.alpha + '%';
  $('stopPos').value = sel.pos; $('posOut').textContent = sel.pos + '%';
  $('delStop').disabled = state.stops.length <= 2;

  codeEl.innerHTML = `.gradient {\n  <span class="prop">background</span>: ${g};\n}`;
}

function deleteSel() {
  if (state.stops.length <= 2) return;
  state.stops.splice(state.stops.indexOf(sel), 1);
  sel = sorted()[0];
  render();
}

// Bar: click adds a stop, handles drag (pointer events → mouse + touch)
bar.addEventListener('pointerdown', e => {
  e.preventDefault();
  if (e.target.classList.contains('handle')) {
    sel = e.target.stop;
  } else {
    const pos = posFromEvent(e);
    sel = { ...colorAt(pos), pos };
    state.stops.push(sel);
  }
  dragging = sel;
  bar.setPointerCapture(e.pointerId);
  render();
});
bar.addEventListener('pointermove', e => {
  if (!dragging) return;
  dragging.pos = posFromEvent(e);
  render();
});
const endDrag = () => { dragging = null; };
bar.addEventListener('pointerup', endDrag);
bar.addEventListener('pointercancel', endDrag);

// Controls
document.querySelectorAll('#typeSeg button').forEach(b => b.onclick = () => { state.type = b.dataset.type; render(); });
$('angle').oninput = e => { state.angle = +e.target.value; render(); };
$('shape').onchange = e => { state.shape = e.target.value; render(); };
$('pos').onchange = e => { state.pos = e.target.value; render(); };
$('stopColor').oninput = e => { sel.color = e.target.value; render(); };
$('stopAlpha').oninput = e => { sel.alpha = +e.target.value; render(); };
$('stopPos').oninput = e => { sel.pos = +e.target.value; render(); };
$('delStop').onclick = deleteSel;

document.addEventListener('keydown', e => {
  if (e.key !== 'Delete' || /INPUT|SELECT|TEXTAREA/.test(document.activeElement.tagName)) return;
  deleteSel();
});

// Preset swatches
function loadPreset([type, angle, stops]) {
  state = { ...DEFAULT(), type, angle, stops: stops.map(([color, pos]) => ({ color, alpha: 100, pos })) };
  sel = state.stops[0];
  render();
}
PRESETS.forEach(p => {
  const b = document.createElement('button');
  const list = p[2].map(([c, pos]) => `${c} ${pos}%`).join(', ');
  b.style.background = p[0] === 'linear' ? `linear-gradient(${p[1]}deg, ${list})`
    : p[0] === 'radial' ? `radial-gradient(circle, ${list})` : `conic-gradient(${list})`;
  b.title = p[0];
  b.onclick = () => loadPreset(p);
  $('presets').appendChild(b);
});

$('random').onclick = () => {
  const rc = () => '#' + Math.floor(Math.random() * 0xffffff).toString(16).padStart(6, '0');
  const n = 2 + Math.floor(Math.random() * 3);
  state.type = ['linear', 'linear', 'radial', 'conic'][Math.floor(Math.random() * 4)];
  state.angle = Math.floor(Math.random() * 360);
  state.stops = Array.from({ length: n }, (_, i) => ({ color: rc(), alpha: 100, pos: Math.round(i / (n - 1) * 100) }));
  sel = state.stops[0];
  render();
};

$('resetAll').onclick = () => { state = DEFAULT(); sel = state.stops[0]; render(); };

$('copy').onclick = async e => {
  try {
    await navigator.clipboard.writeText(codeEl.textContent);
    e.target.textContent = I18N.t('copied');
  } catch {
    e.target.textContent = I18N.t('error');
  }
  setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
};

render();

// Static texts are relabelled by I18N.apply(); reset a pending copy feedback.
addEventListener('langchange', () => { $('copy').textContent = I18N.t('copy'); });

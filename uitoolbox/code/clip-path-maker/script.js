I18N.add({
  en: {
    docTitle: 'Clip-Path Maker', back: '← Overview', showGhost: 'Show cut-out', reset: 'Reset',
    mode: 'Mode', presets: 'Presets', params: 'Parameters', preview: 'Preview', background: 'Background',
    bgG1: 'Gradient', bgG2: 'Conic', bgG3: 'Stripes', bgG4: 'Checks', bgG5: 'Dots',
    hint: 'Drag points to move them.<br>Double-click adds a point.<br>Right-click, or select + <kbd>Del</kbd>, removes a point.',
    generated: 'Generated CSS', copy: 'Copy', copied: 'Copied', error: 'Error',
    p_triangle: 'Triangle', p_trapezoid: 'Trapezoid', p_parallelogram: 'Parallelogram', p_rhombus: 'Rhombus',
    p_pentagon: 'Pentagon', p_hexagon: 'Hexagon', p_star: 'Star', p_arrowRight: 'Arrow right', p_cross: 'Cross',
    p_speech: 'Speech bubble',
    s_radius: 'Radius', s_centerX: 'Centre X', s_centerY: 'Centre Y', s_radiusX: 'Radius X', s_radiusY: 'Radius Y',
    s_top: 'Top', s_right: 'Right', s_bottom: 'Bottom', s_left: 'Left', s_round: 'Rounding',
    pointCount: '{n} points', pointSelected: ' · point {n} selected', modeInfo: 'Mode: {mode}()',
    minPoints: 'At least 3 points needed'
  },
  de: {
    docTitle: 'Clip-Path Maker', back: '← Übersicht', showGhost: 'Ausschnitt zeigen', reset: 'Reset',
    mode: 'Modus', presets: 'Vorlagen', params: 'Parameter', preview: 'Vorschau', background: 'Hintergrund',
    bgG1: 'Verlauf', bgG2: 'Konisch', bgG3: 'Streifen', bgG4: 'Karos', bgG5: 'Punkte',
    hint: 'Punkte ziehen zum Verschieben.<br>Doppelklick fügt einen Punkt hinzu.<br>Rechtsklick oder auswählen + <kbd>Entf</kbd> löscht einen Punkt.',
    generated: 'Generiertes CSS', copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler',
    p_triangle: 'Dreieck', p_trapezoid: 'Trapez', p_parallelogram: 'Parallelogramm', p_rhombus: 'Raute',
    p_pentagon: 'Fünfeck', p_hexagon: 'Sechseck', p_star: 'Stern', p_arrowRight: 'Pfeil rechts', p_cross: 'Kreuz',
    p_speech: 'Sprechblase',
    s_radius: 'Radius', s_centerX: 'Mitte X', s_centerY: 'Mitte Y', s_radiusX: 'Radius X', s_radiusY: 'Radius Y',
    s_top: 'Oben', s_right: 'Rechts', s_bottom: 'Unten', s_left: 'Links', s_round: 'Rundung',
    pointCount: '{n} Punkte', pointSelected: ' · Punkt {n} ausgewählt', modeInfo: 'Modus: {mode}()',
    minPoints: 'Mindestens 3 Punkte nötig'
  }
});

// Polygon presets in % coordinates [x, y]; keys double as i18n ids (p_<key>)
const PRESETS = {
  triangle: [[50, 0], [100, 100], [0, 100]],
  trapezoid: [[20, 0], [80, 0], [100, 100], [0, 100]],
  parallelogram: [[25, 0], [100, 0], [75, 100], [0, 100]],
  rhombus: [[50, 0], [100, 50], [50, 100], [0, 50]],
  pentagon: [[50, 0], [100, 38], [82, 100], [18, 100], [0, 38]],
  hexagon: [[25, 0], [75, 0], [100, 50], [75, 100], [25, 100], [0, 50]],
  star: [[50, 0], [61, 35], [98, 35], [68, 57], [79, 91], [50, 70], [21, 91], [32, 57], [2, 35], [39, 35]],
  arrowRight: [[0, 20], [60, 20], [60, 0], [100, 50], [60, 100], [60, 80], [0, 80]],
  cross: [[35, 0], [65, 0], [65, 35], [100, 35], [100, 65], [65, 65], [65, 100], [35, 100], [35, 65], [0, 65], [0, 35], [35, 35]],
  speech: [[0, 0], [100, 0], [100, 75], [75, 75], [75, 100], [50, 75], [0, 75]],
};

// Slider definitions per mode: [key, label i18n key, min, max, unit]
const SLIDERS = {
  circle: [['r', 's_radius', 0, 75, '%'], ['cx', 's_centerX', 0, 100, '%'], ['cy', 's_centerY', 0, 100, '%']],
  ellipse: [['rx', 's_radiusX', 0, 100, '%'], ['ry', 's_radiusY', 0, 100, '%'], ['cx', 's_centerX', 0, 100, '%'], ['cy', 's_centerY', 0, 100, '%']],
  inset: [['t', 's_top', 0, 50, '%'], ['ri', 's_right', 0, 50, '%'], ['b', 's_bottom', 0, 50, '%'], ['l', 's_left', 0, 50, '%'], ['round', 's_round', 0, 100, 'px']],
};
const DEFAULTS = { r: 45, cx: 50, cy: 50, rx: 48, ry: 32, t: 10, ri: 10, b: 10, l: 10, round: 16 };

const stage = document.getElementById('stage');
const shape = document.getElementById('shape');
const ghost = document.getElementById('ghost');
const handlesEl = document.getElementById('handles');
const outlinePoly = document.getElementById('outlinePoly');
const codeEl = document.getElementById('code');
const infoEl = document.getElementById('pointsInfo');

let mode = 'polygon';
let points = PRESETS.hexagon.map(p => [...p]);
let params = { ...DEFAULTS };
let selected = -1;

const r1 = n => Math.round(n * 10) / 10;
const clamp = (n, a, b) => Math.min(b, Math.max(a, n));

// Preset buttons with small SVG icon
const presetsEl = document.getElementById('presets');
for (const [name, pts] of Object.entries(PRESETS)) {
  const b = document.createElement('button');
  b.innerHTML = `<svg viewBox="0 0 100 100"><polygon points="${pts.map(p => p.join(',')).join(' ')}"/></svg><span data-i18n="p_${name}">${I18N.t('p_' + name)}</span>`;
  b.onclick = () => { points = pts.map(p => [...p]); selected = -1; setMode('polygon'); };
  presetsEl.appendChild(b);
}

// Sliders (rebuilt on mode change)
const slidersEl = document.getElementById('sliders');
function buildSliders() {
  slidersEl.innerHTML = '';
  for (const [key, label, min, max, unit] of SLIDERS[mode] || []) {
    const row = document.createElement('div');
    row.className = 'row';
    row.innerHTML = `<span data-i18n="${label}">${I18N.t(label)}</span><input type="range" min="${min}" max="${max}" step="1"><output></output>`;
    const input = row.querySelector('input');
    const out = row.querySelector('output');
    input.value = params[key];
    out.textContent = params[key] + unit;
    input.dataset.key = key;
    input.addEventListener('input', () => {
      params[key] = +input.value;
      out.textContent = input.value + unit;
      render();
    });
    slidersEl.appendChild(row);
  }
}

function setMode(m) {
  mode = m;
  document.querySelectorAll('#modes button').forEach(b => b.classList.toggle('active', b.dataset.mode === m));
  document.getElementById('sliderBox').hidden = m === 'polygon';
  document.getElementById('hint').hidden = m !== 'polygon';
  buildSliders();
  render();
}
document.querySelectorAll('#modes button').forEach(b => b.onclick = () => setMode(b.dataset.mode));

// Build clip-path value from current state
function clipValue() {
  const p = params;
  if (mode === 'circle') return `circle(${p.r}% at ${p.cx}% ${p.cy}%)`;
  if (mode === 'ellipse') return `ellipse(${p.rx}% ${p.ry}% at ${p.cx}% ${p.cy}%)`;
  if (mode === 'inset') return `inset(${p.t}% ${p.ri}% ${p.b}% ${p.l}%${p.round ? ` round ${p.round}px` : ''})`;
  return `polygon(${points.map(([x, y]) => `${Math.round(x)}% ${Math.round(y)}%`).join(', ')})`;
}

// Handles: polygon points, or a single center handle for circle/ellipse
function currentHandles() {
  if (mode === 'polygon') return points;
  if (mode === 'circle' || mode === 'ellipse') return [[params.cx, params.cy]];
  return [];
}

function render() {
  const val = clipValue();
  shape.style.clipPath = val;

  outlinePoly.setAttribute('points', mode === 'polygon' ? points.map(p => p.join(',')).join(' ') : '');

  // Reuse handle elements so pointer targets stay stable while dragging
  const hs = currentHandles();
  while (handlesEl.children.length > hs.length) handlesEl.lastChild.remove();
  while (handlesEl.children.length < hs.length) {
    const h = document.createElement('div');
    h.appendChild(document.createElement('span'));
    handlesEl.appendChild(h);
  }
  hs.forEach(([x, y], i) => {
    const h = handlesEl.children[i];
    h.className = 'handle' + (i === selected && mode === 'polygon' ? ' sel' : '');
    h.style.left = x + '%';
    h.style.top = y + '%';
    h.dataset.i = i;
    const label = h.firstChild;
    label.hidden = !(i === selected || hs.length === 1);
    label.textContent = `${Math.round(x)}% ${Math.round(y)}%`;
  });

  codeEl.innerHTML = `<span class="prop">clip-path</span>: ${val};`;
  infoEl.textContent = mode === 'polygon'
    ? I18N.t('pointCount', { n: points.length }) + (selected >= 0 ? I18N.t('pointSelected', { n: selected + 1 }) : '')
    : I18N.t('modeInfo', { mode });
}

// Pointer position in % of the stage
function toPct(e) {
  const r = stage.getBoundingClientRect();
  return [clamp((e.clientX - r.left) / r.width * 100, 0, 100), clamp((e.clientY - r.top) / r.height * 100, 0, 100)];
}

// Dragging via pointer events (mouse + touch)
let dragIdx = -1;
let downOnHandle = false;
stage.addEventListener('pointerdown', e => {
  const h = e.target.closest('.handle');
  downOnHandle = !!h;
  if (!h) {
    // Click on empty area deselects
    if (selected >= 0 && e.button === 0) { selected = -1; render(); }
    return;
  }
  if (e.button === 2) return;
  e.preventDefault();
  dragIdx = +h.dataset.i;
  if (mode === 'polygon') selected = dragIdx;
  stage.setPointerCapture(e.pointerId);
  render();
});
stage.addEventListener('pointermove', e => {
  if (dragIdx < 0) return;
  const [x, y] = toPct(e).map(r1);
  if (mode === 'polygon') points[dragIdx] = [x, y];
  else { params.cx = Math.round(x); params.cy = Math.round(y); syncSliders(); }
  render();
});
const endDrag = e => {
  if (dragIdx < 0) return;
  dragIdx = -1;
  if (stage.hasPointerCapture(e.pointerId)) stage.releasePointerCapture(e.pointerId);
};
stage.addEventListener('pointerup', endDrag);
stage.addEventListener('pointercancel', endDrag);

function syncSliders() {
  slidersEl.querySelectorAll('input').forEach(inp => {
    inp.value = params[inp.dataset.key];
    inp.nextElementSibling.textContent = inp.value + '%';
  });
}

// Distance from point p to segment a-b
function segDist(p, a, b) {
  const dx = b[0] - a[0], dy = b[1] - a[1];
  const len = dx * dx + dy * dy;
  const t = len ? clamp(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len, 0, 1) : 0;
  return Math.hypot(p[0] - (a[0] + t * dx), p[1] - (a[1] + t * dy));
}

// Double-click adds a point on the nearest edge
stage.addEventListener('dblclick', e => {
  if (mode !== 'polygon' || downOnHandle) return;
  const p = toPct(e).map(r1);
  let best = 0, bestD = Infinity;
  points.forEach((a, i) => {
    const d = segDist(p, a, points[(i + 1) % points.length]);
    if (d < bestD) { bestD = d; best = i; }
  });
  points.splice(best + 1, 0, p);
  selected = best + 1;
  render();
});

// Right-click removes a point
stage.addEventListener('contextmenu', e => {
  const h = e.target.closest('.handle');
  if (!h || mode !== 'polygon') return;
  e.preventDefault();
  removePoint(+h.dataset.i);
});

function removePoint(i) {
  if (points.length <= 3) { infoEl.textContent = I18N.t('minPoints'); return; }
  points.splice(i, 1);
  selected = -1;
  render();
}

document.addEventListener('keydown', e => {
  if (mode !== 'polygon' || selected < 0) return;
  if (/INPUT|SELECT|TEXTAREA/.test(document.activeElement.tagName)) return;
  if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); removePoint(selected); return; }
  // Arrow keys nudge the selected point
  const step = e.shiftKey ? 5 : 1;
  const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[e.key];
  if (d) {
    e.preventDefault();
    const p = points[selected];
    points[selected] = [clamp(p[0] + d[0], 0, 100), clamp(p[1] + d[1], 0, 100)];
    render();
  }
});

document.getElementById('showGhost').onchange = e => ghost.classList.toggle('off', !e.target.checked);

document.getElementById('bgSelect').onchange = e => {
  for (const el of [shape, ghost]) el.className = el.className.replace(/\bg\d\b/, e.target.value);
};

document.getElementById('copy').onclick = async e => {
  try {
    await navigator.clipboard.writeText(codeEl.textContent);
    e.target.textContent = I18N.t('copied');
  } catch {
    e.target.textContent = I18N.t('error');
  }
  setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
};

document.getElementById('resetAll').onclick = () => {
  points = PRESETS.hexagon.map(p => [...p]);
  params = { ...DEFAULTS };
  selected = -1;
  setMode('polygon');
};

// Preset and slider labels carry data-i18n and are updated by I18N itself; the info line needs a re-render.
addEventListener('langchange', render);

setMode('polygon');

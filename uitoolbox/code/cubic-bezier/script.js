I18N.add({
  en: {
    docTitle: 'Cubic-Bezier Editor', back: '← Overview', heading: 'Cubic-Bezier Editor',
    play: 'Play', reset: 'Reset', controlPoints: 'Control points',
    rangeHint: 'x: 0 – 1, y: −0.8 – 1.8 (values outside 0 – 1 cause overshoot).',
    presets: 'Presets', compareTitle: 'Comparison & timing', compare: 'Compare with', duration: 'Duration',
    autoRepeat: 'Repeat automatically', curveLabel: 'Bezier curve', yourCurve: 'Your curve',
    generated: 'Generated CSS', copy: 'Copy', copied: 'Copied', error: 'Error',
    matches: 'matches: {name}', shorthand: 'Example as shorthand'
  },
  de: {
    docTitle: 'Cubic-Bezier Editor', back: '← Übersicht', heading: 'Cubic-Bezier Editor',
    play: 'Abspielen', reset: 'Reset', controlPoints: 'Kontrollpunkte',
    rangeHint: 'x: 0 – 1, y: −0.8 – 1.8 (Werte außerhalb 0 – 1 erzeugen Überschwingen).',
    presets: 'Vorlagen', compareTitle: 'Vergleich & Ablauf', compare: 'Vergleich', duration: 'Dauer',
    autoRepeat: 'Automatisch wiederholen', curveLabel: 'Bezier-Kurve', yourCurve: 'Deine Kurve',
    generated: 'Generiertes CSS', copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler',
    matches: 'entspricht: {name}', shorthand: 'Beispiel als Kurzschreibweise'
  }
});

// Presets: name → [x1, y1, x2, y2]
const PRESETS = {
  'ease': [0.25, 0.1, 0.25, 1],
  'linear': [0, 0, 1, 1],
  'ease-in': [0.42, 0, 1, 1],
  'ease-out': [0, 0, 0.58, 1],
  'ease-in-out': [0.42, 0, 0.58, 1],
  'easeInCubic': [0.32, 0, 0.67, 0],
  'easeOutCubic': [0.33, 1, 0.68, 1],
  'easeInOutCubic': [0.65, 0, 0.35, 1],
  'easeInExpo': [0.7, 0, 0.84, 0],
  'easeOutExpo': [0.16, 1, 0.3, 1],
  'easeInBack': [0.36, 0, 0.66, -0.56],
  'easeOutBack': [0.34, 1.56, 0.64, 1],
  'easeInOutBack': [0.68, -0.6, 0.32, 1.6],
  'easeOutCirc': [0, 0.55, 0.45, 1],
};
const COMPARE = ['linear', 'ease', 'ease-in', 'ease-out', 'ease-in-out'];
const Y_MIN = -0.8, Y_MAX = 1.8;

const $ = id => document.getElementById(id);
const svg = $('editor'), codeEl = $('code');
const runMine = $('runMine'), runCmp = $('runCmp');

let pts, dur, cmpName, dragIdx = -1, timer = null;

// Curve space → SVG space (unit square = 160px)
const X = x => 70 + x * 160;
const Y = y => 290 - y * 160;
const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
const r2 = v => Math.round(v * 100) / 100;
const cb = p => `cubic-bezier(${p.map(r2).join(', ')})`;
const pathD = (p, sx = X, sy = Y) => `M${sx(0)},${sy(0)} C${sx(p[0])},${sy(p[1])} ${sx(p[2])},${sy(p[3])} ${sx(1)},${sy(1)}`;

// Background grid lines (quarters)
let lines = '';
for (let i = 1; i < 4; i++) {
  const v = 70 + i * 40, h = 130 + i * 40;
  lines += `<line x1="${v}" y1="130" x2="${v}" y2="290"/><line x1="70" y1="${h}" x2="230" y2="${h}"/>`;
}
$('gridLines').innerHTML = lines;

function render() {
  $('curve').setAttribute('d', pathD(pts));
  $('cmpCurve').setAttribute('d', pathD(PRESETS[cmpName]));
  const set = (el, attrs) => { for (const k in attrs) el.setAttribute(k, attrs[k]); };
  set($('l1'), { x1: X(0), y1: Y(0), x2: X(pts[0]), y2: Y(pts[1]) });
  set($('l2'), { x1: X(1), y1: Y(1), x2: X(pts[2]), y2: Y(pts[3]) });
  set($('p1'), { cx: X(pts[0]), cy: Y(pts[1]) });
  set($('p2'), { cx: X(pts[2]), cy: Y(pts[3]) });

  // Sync numeric inputs (not the one being typed in)
  pts.forEach((v, i) => { const n = $('n' + i); if (document.activeElement !== n) n.value = r2(v); });
  $('dur').value = dur; $('durOut').textContent = dur + 's';
  $('compare').value = cmpName;
  $('cmpLbl').textContent = cmpName;

  // Mark matching preset
  const cur = pts.map(r2).join();
  let match = null;
  document.querySelectorAll('#presets button').forEach(b => {
    const on = PRESETS[b.dataset.name].join() === cur;
    b.classList.toggle('on', on);
    if (on) match = b.dataset.name;
  });

  const p = `<span class="prop">`;
  codeEl.innerHTML = `.element {\n  ${p}transition-timing-function</span>: ${cb(pts)};\n}`
    + (match ? `\n<span class="mute">/* ${I18N.t('matches', { name: match })} */</span>` : '')
    + `\n\n<span class="mute">/* ${I18N.t('shorthand')} */</span>\n.element {\n  ${p}transition</span>: transform ${dur}s ${cb(pts)};\n}`;
}

// Drag control points (pointer events → mouse + touch)
function toCurve(e) {
  const pt = svg.createSVGPoint();
  pt.x = e.clientX; pt.y = e.clientY;
  const s = pt.matrixTransform(svg.getScreenCTM().inverse());
  return [r2(clamp((s.x - 70) / 160, 0, 1)), r2(clamp((290 - s.y) / 160, Y_MIN, Y_MAX))];
}
svg.addEventListener('pointerdown', e => {
  if (e.target.id === 'p1') dragIdx = 0;
  else if (e.target.id === 'p2') dragIdx = 2;
  else {
    // Click elsewhere: move the nearest point there
    const [x, y] = toCurve(e);
    const d = i => (pts[i] - x) ** 2 + (pts[i + 1] - y) ** 2;
    dragIdx = d(0) <= d(2) ? 0 : 2;
  }
  e.preventDefault();
  svg.setPointerCapture(e.pointerId);
  moveDrag(e);
});
function moveDrag(e) {
  if (dragIdx < 0) return;
  [pts[dragIdx], pts[dragIdx + 1]] = toCurve(e);
  render();
}
svg.addEventListener('pointermove', moveDrag);
const endDrag = () => { if (dragIdx >= 0) { dragIdx = -1; if ($('auto').checked) play(); } };
svg.addEventListener('pointerup', endDrag);
svg.addEventListener('pointercancel', endDrag);

// Numeric inputs
for (let i = 0; i < 4; i++) {
  const n = $('n' + i);
  n.oninput = () => {
    const v = parseFloat(n.value);
    if (isNaN(v)) return;
    pts[i] = i % 2 ? clamp(v, Y_MIN, Y_MAX) : clamp(v, 0, 1);
    render();
  };
  n.onchange = () => { n.value = r2(pts[i]); if ($('auto').checked) play(); };
}

// Preset buttons with mini curve preview
Object.entries(PRESETS).forEach(([name, p]) => {
  const b = document.createElement('button');
  b.dataset.name = name;
  const d = pathD(p, x => 4 + x * 32, y => 30 - y * 20);
  b.innerHTML = `<svg viewBox="0 0 40 40"><path d="${d}"/></svg>${name}`;
  b.onclick = () => { pts = [...p]; render(); play(); };
  $('presets').appendChild(b);
});
COMPARE.forEach(c => $('compare').add(new Option(c, c)));

$('compare').onchange = e => { cmpName = e.target.value; render(); play(); };
$('dur').oninput = e => { dur = +e.target.value; render(); };
$('dur').onchange = () => play();
$('auto').onchange = e => e.target.checked ? play() : clearTimeout(timer);

// Race: reset both runners, then animate with their curves
function play() {
  clearTimeout(timer);
  [runMine, runCmp].forEach(r => { r.style.transition = 'none'; r.style.left = '6px'; });
  void runMine.offsetWidth; // force reflow so the transition restarts
  runMine.style.transition = `left ${dur}s ${cb(pts)}`;
  runCmp.style.transition = `left ${dur}s ${cb(PRESETS[cmpName])}`;
  runMine.style.left = runCmp.style.left = 'calc(100% - 40px)';
  if ($('auto').checked) timer = setTimeout(play, dur * 1000 + 800);
}
$('play').onclick = play;

$('resetAll').onclick = () => { init(); render(); play(); };

$('copy').onclick = async e => {
  try {
    await navigator.clipboard.writeText(codeEl.textContent);
    e.target.textContent = I18N.t('copied');
  } catch {
    e.target.textContent = I18N.t('error');
  }
  setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
};

function init() {
  pts = [...PRESETS.easeOutBack];
  dur = 1.2;
  cmpName = 'linear';
}

init();
render();
play();

// Static texts are relabelled by I18N.apply(); refresh the code comments (curve and race keep running).
addEventListener('langchange', () => { $('copy').textContent = I18N.t('copy'); render(); });

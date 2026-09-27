I18N.add({
  en: {
    docTitle: 'CSS Playground', back: '← Overview', heading: 'CSS Playground',
    random: 'Random', reset: 'Reset', generated: 'Generated CSS',
    copy: 'Copy', copied: 'Copied', error: 'Error', stageBg: 'Preview background',
    gSize: 'Size & shape', gBg: 'Background', gShadow: 'Shadow', gTransform: 'Transform', gFilter: 'Filter', gText: 'Text & animation',
    width: 'Width', height: 'Height', radius: 'Radius', borderW: 'Border', borderC: 'Border colour', borderS: 'Border style',
    gradType: 'Type', c1: 'Colour 1', c2: 'Colour 2', angle: 'Angle', opacity: 'Opacity',
    shBlur: 'Blur', shSpread: 'Spread', shColor: 'Colour', shAlpha: 'Alpha', shInset: 'Inset',
    rotate: 'Rotation', scale: 'Scale', skewX: 'Skew X', rotY: 'Rotate Y (3D)',
    blur: 'Blur', bright: 'Brightness', hue: 'Hue', sat: 'Saturation', gray: 'Greyscale',
    text: 'Text colour', fontSize: 'Font size', anim: 'Animation', animDur: 'Duration',
    optSolid: 'solid colour', optNo: 'no', optYes: 'yes', optNone: 'none'
  },
  de: {
    docTitle: 'CSS Playground', back: '← Übersicht', heading: 'CSS Playground',
    random: 'Zufall', reset: 'Reset', generated: 'Generiertes CSS',
    copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler', stageBg: 'Hintergrund Vorschau',
    gSize: 'Größe & Form', gBg: 'Hintergrund', gShadow: 'Schatten', gTransform: 'Transform', gFilter: 'Filter', gText: 'Text & Animation',
    width: 'Breite', height: 'Höhe', radius: 'Radius', borderW: 'Rahmen', borderC: 'Rahmenfarbe', borderS: 'Rahmenstil',
    gradType: 'Typ', c1: 'Farbe 1', c2: 'Farbe 2', angle: 'Winkel', opacity: 'Deckkraft',
    shBlur: 'Blur', shSpread: 'Spread', shColor: 'Farbe', shAlpha: 'Alpha', shInset: 'Inset',
    rotate: 'Rotation', scale: 'Skalierung', skewX: 'Skew X', rotY: 'Rotate Y (3D)',
    blur: 'Blur', bright: 'Helligkeit', hue: 'Farbton', sat: 'Sättigung', gray: 'Graustufen',
    text: 'Textfarbe', fontSize: 'Schriftgröße', anim: 'Animation', animDur: 'Dauer',
    optSolid: 'einfarbig', optNo: 'nein', optYes: 'ja', optNone: 'keine'
  }
});

// Each control: [id, labelKey, type, default, min, max, step, unit]
// Select options are either a plain value (shown as is) or [value, labelKey].
// A label key that is not in the dictionary (e.g. 'X') is shown literally.
const GROUPS = [
  ['gSize', [
    ['width', 'width', 'range', 220, 40, 500, 1, 'px'],
    ['height', 'height', 'range', 220, 40, 500, 1, 'px'],
    ['radius', 'radius', 'range', 24, 0, 250, 1, 'px'],
    ['borderW', 'borderW', 'range', 0, 0, 20, 1, 'px'],
    ['borderC', 'borderC', 'color', '#ffffff'],
    ['borderS', 'borderS', 'select', 'solid', ['solid', 'dashed', 'dotted', 'double', 'groove', 'ridge']],
  ]],
  ['gBg', [
    ['gradType', 'gradType', 'select', 'linear', ['linear', 'radial', 'conic', ['solid', 'optSolid']]],
    ['c1', 'c1', 'color', '#d9774b'],
    ['c2', 'c2', 'color', '#e9b872'],
    ['angle', 'angle', 'range', 135, 0, 360, 1, 'deg'],
    ['opacity', 'opacity', 'range', 100, 0, 100, 1, '%'],
  ]],
  ['gShadow', [
    ['shX', 'X', 'range', 0, -60, 60, 1, 'px'],
    ['shY', 'Y', 'range', 20, -60, 60, 1, 'px'],
    ['shBlur', 'shBlur', 'range', 40, 0, 120, 1, 'px'],
    ['shSpread', 'shSpread', 'range', -10, -50, 50, 1, 'px'],
    ['shColor', 'shColor', 'color', '#8a5a3c'],
    ['shAlpha', 'shAlpha', 'range', 60, 0, 100, 1, '%'],
    ['shInset', 'shInset', 'select', 'no', [['no', 'optNo'], ['yes', 'optYes']]],
  ]],
  ['gTransform', [
    ['rotate', 'rotate', 'range', 0, -180, 180, 1, 'deg'],
    ['scale', 'scale', 'range', 100, 20, 200, 1, '%'],
    ['skewX', 'skewX', 'range', 0, -45, 45, 1, 'deg'],
    ['rotY', 'rotY', 'range', 0, -80, 80, 1, 'deg'],
  ]],
  ['gFilter', [
    ['blur', 'blur', 'range', 0, 0, 20, 0.5, 'px'],
    ['bright', 'bright', 'range', 100, 0, 200, 1, '%'],
    ['hue', 'hue', 'range', 0, 0, 360, 1, 'deg'],
    ['sat', 'sat', 'range', 100, 0, 300, 1, '%'],
    ['gray', 'gray', 'range', 0, 0, 100, 1, '%'],
  ]],
  ['gText', [
    ['text', 'text', 'color', '#fffdf8'],
    ['fontSize', 'fontSize', 'range', 28, 8, 80, 1, 'px'],
    ['anim', 'anim', 'select', 'none', [['none', 'optNone'], 'pulse', 'spin', 'float', 'shake', 'glow']],
    ['animDur', 'animDur', 'range', 2, 0.2, 6, 0.1, 's'],
  ]],
];

const KEYFRAMES = {
  pulse: '@keyframes pulse {\n  50% { transform: scale(1.08); }\n}',
  spin: '@keyframes spin {\n  to { transform: rotate(360deg); }\n}',
  float: '@keyframes float {\n  50% { transform: translateY(-20px); }\n}',
  shake: '@keyframes shake {\n  25% { transform: translateX(-8px); }\n  75% { transform: translateX(8px); }\n}',
  glow: '@keyframes glow {\n  50% { filter: brightness(1.4) drop-shadow(0 0 20px currentColor); }\n}',
};

const values = {};
const inputs = {};
const aside = document.getElementById('controls');
const box = document.getElementById('box');
const codeEl = document.getElementById('code');

// Inject keyframes once so preview animations work
const kfStyle = document.createElement('style');
kfStyle.textContent = Object.values(KEYFRAMES).join('\n');
document.head.appendChild(kfStyle);

GROUPS.forEach(([title, ctrls], gi) => {
  const det = document.createElement('details');
  det.open = gi < 3;
  det.innerHTML = `<summary data-i18n="${title}">${I18N.t(title)}</summary><div class="rows"></div>`;
  const rows = det.querySelector('.rows');
  for (const [id, label, type, def, a, b, step, unit] of ctrls) {
    const row = document.createElement('div');
    row.className = 'row';
    let input;
    if (type === 'select') {
      input = document.createElement('select');
      a.forEach(o => {
        const [val, key] = Array.isArray(o) ? o : [o];
        const opt = new Option(key ? I18N.t(key) : val, val);
        if (key) opt.dataset.i18n = key;
        input.add(opt);
      });
      row.innerHTML = `<span data-i18n="${label}">${I18N.t(label)}</span>`;
    } else {
      input = document.createElement('input');
      input.type = type;
      if (type === 'range') Object.assign(input, { min: a, max: b, step });
      row.innerHTML = `<span data-i18n="${label}">${I18N.t(label)}</span>`;
    }
    input.value = def;
    input.dataset.def = def;
    row.appendChild(input);
    if (type === 'range') {
      const out = document.createElement('output');
      row.appendChild(out);
      input.addEventListener('input', () => out.textContent = input.value + unit);
      out.textContent = def + unit;
    } else if (type === 'color') {
      row.appendChild(document.createElement('span'));
    }
    input.addEventListener('input', update);
    inputs[id] = input;
    rows.appendChild(row);
  }
  aside.appendChild(det);
});

const hexA = (hex, pct) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${n >> 16}, ${(n >> 8) & 255}, ${n & 255}, ${+(pct / 100).toFixed(2)})`;
};

function update() {
  for (const id in inputs) values[id] = inputs[id].value;
  const v = values;

  const css = {};
  css.width = v.width + 'px';
  css.height = v.height + 'px';
  css['border-radius'] = v.radius + 'px';
  if (+v.borderW) css.border = `${v.borderW}px ${v.borderS} ${v.borderC}`;
  css.background = v.gradType === 'solid' ? v.c1
    : v.gradType === 'linear' ? `linear-gradient(${v.angle}deg, ${v.c1}, ${v.c2})`
    : v.gradType === 'radial' ? `radial-gradient(circle, ${v.c1}, ${v.c2})`
    : `conic-gradient(from ${v.angle}deg, ${v.c1}, ${v.c2}, ${v.c1})`;
  if (+v.opacity !== 100) css.opacity = v.opacity / 100;
  if (+v.shAlpha) css['box-shadow'] = `${v.shInset === 'yes' ? 'inset ' : ''}${v.shX}px ${v.shY}px ${v.shBlur}px ${v.shSpread}px ${hexA(v.shColor, v.shAlpha)}`;

  const tf = [];
  if (+v.rotY) tf.push(`perspective(600px) rotateY(${v.rotY}deg)`);
  if (+v.rotate) tf.push(`rotate(${v.rotate}deg)`);
  if (+v.scale !== 100) tf.push(`scale(${v.scale / 100})`);
  if (+v.skewX) tf.push(`skewX(${v.skewX}deg)`);
  if (tf.length) css.transform = tf.join(' ');

  const fl = [];
  if (+v.blur) fl.push(`blur(${v.blur}px)`);
  if (+v.bright !== 100) fl.push(`brightness(${v.bright}%)`);
  if (+v.hue) fl.push(`hue-rotate(${v.hue}deg)`);
  if (+v.sat !== 100) fl.push(`saturate(${v.sat}%)`);
  if (+v.gray) fl.push(`grayscale(${v.gray}%)`);
  if (fl.length) css.filter = fl.join(' ');

  css.color = v.text;
  css['font-size'] = v.fontSize + 'px';
  if (v.anim !== 'none') css.animation = `${v.anim} ${v.animDur}s ease-in-out infinite`;

  // Apply to preview
  box.removeAttribute('style');
  for (const k in css) box.style.setProperty(k, css[k]);

  // Render code
  const body = Object.entries(css).map(([k, val]) => `  <span class="prop">${k}</span>: ${val};`).join('\n');
  let text = `.box {\n${body}\n}`;
  if (v.anim !== 'none') text += '\n\n' + KEYFRAMES[v.anim];
  codeEl.innerHTML = text;
}

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
  for (const id in inputs) {
    inputs[id].value = inputs[id].dataset.def;
    inputs[id].dispatchEvent(new Event('input'));
  }
};

document.getElementById('random').onclick = () => {
  const rc = () => '#' + Math.floor(Math.random() * 0xffffff).toString(16).padStart(6, '0');
  const set = (id, val) => { inputs[id].value = val; inputs[id].dispatchEvent(new Event('input')); };
  set('c1', rc()); set('c2', rc()); set('shColor', rc());
  set('angle', Math.floor(Math.random() * 360));
  set('radius', Math.floor(Math.random() * 120));
  set('gradType', ['linear', 'radial', 'conic'][Math.floor(Math.random() * 3)]);
  set('shY', Math.floor(Math.random() * 40));
  set('shBlur', Math.floor(Math.random() * 80));
};

document.getElementById('stageBg').oninput = e => {
  document.getElementById('previewArea').style.background = e.target.value;
};

// Generated labels carry data-i18n, so I18N.apply() relabels them on a language switch;
// only the copy button may still show a stale feedback text.
addEventListener('langchange', () => { document.getElementById('copy').textContent = I18N.t('copy'); });

update();

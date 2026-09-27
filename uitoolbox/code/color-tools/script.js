I18N.add({
  en: {
    title: 'Colour Tools', back: '← Overview', random: 'Random', reset: 'Reset',
    converter: 'Converter', copy: 'Copy', copied: 'Copied', error: 'Error', copyError: 'Copying failed',
    convHint: 'Values are editable, e.g. <code>#f0a</code>, <code>rgb(255 0 128)</code>, <code>hsl(210, 80%, 60%)</code>',
    invalidValue: 'Invalid {type} value',
    palettes: 'Palettes', palettesHint: '· Click a colour to copy its HEX',
    pal_complementary: 'Complementary', pal_splitComplementary: 'Split complementary', pal_analogous: 'Analogous',
    pal_triadic: 'Triadic', pal_tetradic: 'Tetradic', pal_monochrome: 'Monochrome',
    swatchTitle: '{hex} · {rgb} — click to copy', hexCopied: '{hex} copied',
    contrastHead: 'Contrast Checker (WCAG)', text: 'Text', background: 'Background',
    swap: 'Swap', useBase: 'Base colour as text',
    largeSample: 'Large text (24px)', normalSample: 'Normal text: The quick brown fox jumps over the lazy dog.',
    ratio: 'Contrast ratio', normalText: 'Normal text', largeText: 'Large text',
    pass: 'Pass', fail: 'Fail', cssVars: 'CSS Variables'
  },
  de: {
    title: 'Farb-Tools', back: '← Übersicht', random: 'Zufall', reset: 'Reset',
    converter: 'Konverter', copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler', copyError: 'Fehler beim Kopieren',
    convHint: 'Werte sind editierbar, z. B. <code>#f0a</code>, <code>rgb(255 0 128)</code>, <code>hsl(210, 80%, 60%)</code>',
    invalidValue: 'Ungültiger {type}-Wert',
    palettes: 'Paletten', palettesHint: '· Klick auf Farbe kopiert HEX',
    pal_complementary: 'Komplementär', pal_splitComplementary: 'Split-Komplementär', pal_analogous: 'Analog',
    pal_triadic: 'Triadisch', pal_tetradic: 'Tetradisch', pal_monochrome: 'Monochrom',
    swatchTitle: '{hex} · {rgb} — Klicken zum Kopieren', hexCopied: '{hex} kopiert',
    contrastHead: 'Kontrast-Prüfer (WCAG)', text: 'Text', background: 'Hintergrund',
    swap: 'Tauschen', useBase: 'Basisfarbe als Text',
    largeSample: 'Großer Text (24px)', normalSample: 'Normaler Text: Franz jagt im komplett verwahrlosten Taxi quer durch Bayern.',
    ratio: 'Kontrastverhältnis', normalText: 'Normaler Text', largeText: 'Großer Text',
    pass: 'Bestanden', fail: 'Nicht bestanden', cssVars: 'CSS Variablen'
  }
});

const $ = id => document.getElementById(id);
const DEF_BASE = '#0071e3', DEF_FG = '#1f1d18', DEF_BG = '#fffdf8';

// ---------- Conversions ----------
const clamp = (n, a, b) => Math.min(b, Math.max(a, n));
const hex2 = n => Math.round(n).toString(16).padStart(2, '0');
const toHex = ({ r, g, b }) => '#' + hex2(r) + hex2(g) + hex2(b);

function rgbToHsl({ r, g, b }) {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0, s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > .5 ? d / (2 - max - min) : d / (max + min);
    h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    h *= 60;
  }
  return { h, s: s * 100, l: l * 100 };
}

function hslToRgb({ h, s, l }) {
  h = ((h % 360) + 360) % 360; s /= 100; l /= 100;
  const k = n => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = n => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return { r: Math.round(f(0) * 255), g: Math.round(f(8) * 255), b: Math.round(f(4) * 255) };
}

const fmtRgb = c => `rgb(${c.r}, ${c.g}, ${c.b})`;
const fmtHsl = c => { const h = rgbToHsl(c); return `hsl(${Math.round(h.h)}, ${Math.round(h.s)}%, ${Math.round(h.l)}%)`; };

// ---------- Parsing (returns {r,g,b} or null) ----------
function parseHex(s) {
  const m = s.trim().replace(/^#/, '').match(/^([0-9a-f]{3}|[0-9a-f]{6})$/i);
  if (!m) return null;
  let h = m[1];
  if (h.length === 3) h = [...h].map(c => c + c).join('');
  const n = parseInt(h, 16);
  return { r: n >> 16, g: (n >> 8) & 255, b: n & 255 };
}

function parseRgb(s) {
  const m = s.trim().match(/^(?:rgba?\()?\s*(\d{1,3})\s*[,\s]\s*(\d{1,3})\s*[,\s]\s*(\d{1,3})\s*(?:[,/]\s*[\d.]+%?\s*)?\)?$/i);
  if (!m) return null;
  const [r, g, b] = m.slice(1).map(Number);
  return r > 255 || g > 255 || b > 255 ? null : { r, g, b };
}

function parseHsl(s) {
  const m = s.trim().match(/^(?:hsla?\()?\s*(-?[\d.]+)(?:deg)?\s*[,\s]\s*([\d.]+)%?\s*[,\s]\s*([\d.]+)%?\s*(?:[,/]\s*[\d.]+%?\s*)?\)?$/i);
  if (!m) return null;
  const [h, sat, l] = m.slice(1).map(Number);
  if ([h, sat, l].some(isNaN) || sat > 100 || l > 100) return null;
  return hslToRgb({ h, s: sat, l });
}

const parseAny = s => parseHex(s) || parseRgb(s) || parseHsl(s);

// Relative luminance + contrast (WCAG 2.x)
function luminance({ r, g, b }) {
  const ch = v => { v /= 255; return v <= .03928 ? v / 12.92 : Math.pow((v + .055) / 1.055, 2.4); };
  return .2126 * ch(r) + .7152 * ch(g) + .0722 * ch(b);
}
function contrast(a, b) {
  const [l1, l2] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (l1 + .05) / (l2 + .05);
}
const textOn = c => luminance(c) > .35 ? '#000' : '#fff';
const mix = (a, b, t) => ({ r: Math.round(a.r + (b.r - a.r) * t), g: Math.round(a.g + (b.g - a.g) * t), b: Math.round(a.b + (b.b - a.b) * t) });

// ---------- State ----------
let base = parseHex(DEF_BASE);
let palettes = {};
let convErr = null; // field type of the current invalid-value message, e.g. 'HEX'

// ---------- (a) Converter ----------
const fields = { hexIn: [parseHex, toHex], rgbIn: [parseRgb, fmtRgb], hslIn: [parseHsl, fmtHsl] };

function setBase(c, skip) {
  base = { r: Math.round(c.r), g: Math.round(c.g), b: Math.round(c.b) };
  $('picker').value = toHex(base);
  $('bigSwatch').style.background = toHex(base);
  for (const id in fields) {
    $(id).classList.remove('invalid');
    if (id !== skip) $(id).value = fields[id][1](base);
  }
  $('convMsg').classList.remove('err');
  renderPalettes();
}

$('picker').addEventListener('input', e => setBase(parseHex(e.target.value)));

for (const id in fields) {
  $(id).addEventListener('input', e => {
    const c = fields[id][0](e.target.value);
    if (c) setBase(c, id);
    else {
      e.target.classList.add('invalid');
      convErr = id.slice(0, 3).toUpperCase();
      $('convMsg').textContent = I18N.t('invalidValue', { type: convErr });
      $('convMsg').classList.add('err');
    }
  });
  // Normalize on blur
  $(id).addEventListener('blur', e => { e.target.value = fields[id][1](base); e.target.classList.remove('invalid'); resetMsg(); });
}
function resetMsg() {
  convErr = null;
  $('convMsg').classList.remove('err');
  $('convMsg').innerHTML = I18N.t('convHint');
}

// ---------- (b) Palettes ----------
// Palette ids are i18n keys (pal_<id>); CSS variable names stay fixed regardless of language.
const PAL_SLUG = { complementary: 'complementary', splitComplementary: 'split-complementary', analogous: 'analog',
  triadic: 'triadic', tetradic: 'tetradic', monochrome: 'monochrome' };
function buildPalettes() {
  const hsl = rgbToHsl(base);
  const rot = d => hslToRgb({ ...hsl, h: hsl.h + d });
  const black = { r: 0, g: 0, b: 0 }, white = { r: 255, g: 255, b: 255 };
  return {
    complementary: [base, rot(180)],
    splitComplementary: [base, rot(150), rot(210)],
    analogous: [rot(-60), rot(-30), base, rot(30), rot(60)],
    triadic: [base, rot(120), rot(240)],
    tetradic: [base, rot(90), rot(180), rot(270)],
    monochrome: [mix(base, black, .75), mix(base, black, .5), mix(base, black, .25), base,
                  mix(base, white, .25), mix(base, white, .5), mix(base, white, .75)],
  };
}

function renderPalettes() {
  palettes = buildPalettes();
  const wrap = $('palettes');
  wrap.innerHTML = '';
  const baseHex = toHex(base);
  for (const [name, cols] of Object.entries(palettes)) {
    const div = document.createElement('div');
    div.className = 'pal';
    div.innerHTML = `<h3>${I18N.t('pal_' + name)}</h3><div class="swatches"></div>`;
    const sw = div.querySelector('.swatches');
    for (const c of cols) {
      const hex = toHex(c);
      const s = document.createElement('div');
      s.className = 'swatch' + (hex === baseHex ? ' base' : '');
      s.style.background = hex;
      s.style.color = textOn(c);
      s.textContent = hex;
      s.title = I18N.t('swatchTitle', { hex, rgb: fmtRgb(c) });
      s.onclick = () => copyText(hex, I18N.t('hexCopied', { hex }));
      sw.appendChild(s);
    }
    wrap.appendChild(div);
  }
  renderVars();
}

// ---------- (c) Contrast ----------
let fg = parseHex(DEF_FG), bg = parseHex(DEF_BG);

function renderContrast() {
  $('fgPick').value = toHex(fg); $('bgPick').value = toHex(bg);
  const pv = $('cPreview');
  pv.style.color = toHex(fg);
  pv.style.background = toHex(bg);
  const r = contrast(fg, bg);
  $('ratio').textContent = (Math.floor(r * 100) / 100).toFixed(2) + ' : 1';
  const badge = ok => `<span class="badge ${ok ? 'pass' : 'fail'}">${I18N.t(ok ? 'pass' : 'fail')}</span>`;
  $('aaN').innerHTML = badge(r >= 4.5);
  $('aaaN').innerHTML = badge(r >= 7);
  $('aaL').innerHTML = badge(r >= 3);
  $('aaaL').innerHTML = badge(r >= 4.5);
}

function bindContrast(pickId, hexId, set) {
  $(pickId).addEventListener('input', e => { set(parseHex(e.target.value)); $(hexId).value = e.target.value; $(hexId).classList.remove('invalid'); renderContrast(); });
  $(hexId).addEventListener('input', e => {
    const c = parseAny(e.target.value);
    e.target.classList.toggle('invalid', !c);
    if (c) { set(c); renderContrast(); }
  });
  $(hexId).addEventListener('blur', () => syncContrastText());
}
bindContrast('fgPick', 'fgHex', c => fg = c);
bindContrast('bgPick', 'bgHex', c => bg = c);

function syncContrastText() {
  $('fgHex').value = toHex(fg); $('bgHex').value = toHex(bg);
  $('fgHex').classList.remove('invalid'); $('bgHex').classList.remove('invalid');
}

$('swap').onclick = () => { [fg, bg] = [bg, fg]; syncContrastText(); renderContrast(); };
$('useBase').onclick = () => { fg = { ...base }; syncContrastText(); renderContrast(); };

// ---------- (d) CSS variables ----------

function renderVars() {
  const lines = [':root {'];
  lines.push(`  <span class="prop">--color-base</span>: <span class="sw" style="background:${toHex(base)}"></span>${toHex(base)};`);
  for (const [name, cols] of Object.entries(palettes)) {
    const others = name === 'monochrome' ? cols : cols.filter(c => toHex(c) !== toHex(base));
    others.forEach((c, i) => {
      const key = name === 'monochrome' ? `--${PAL_SLUG[name]}-${(i + 1) * 100}` : `--${PAL_SLUG[name]}-${i + 1}`;
      lines.push(`  <span class="prop">${key}</span>: <span class="sw" style="background:${toHex(c)}"></span>${toHex(c)};`);
    });
  }
  lines.push('}');
  $('code').innerHTML = lines.join('\n');
}

// ---------- Clipboard ----------
let toastTimer;
function toast(msg) {
  const t = $('toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove('show'), 1400);
}
async function copyText(text, okMsg) {
  try { await navigator.clipboard.writeText(text); toast(okMsg); }
  catch { toast(I18N.t('copyError')); }
}

document.querySelectorAll('[data-copy]').forEach(b => b.addEventListener('click', e => {
  e.preventDefault();
  copyText($(b.dataset.copy).value, I18N.t('copied'));
}));

$('copy').onclick = async e => {
  try {
    await navigator.clipboard.writeText($('code').textContent);
    e.target.textContent = I18N.t('copied');
  } catch {
    e.target.textContent = I18N.t('error');
  }
  setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
};

$('random').onclick = () => setBase(hslToRgb({ h: Math.random() * 360, s: 50 + Math.random() * 40, l: 40 + Math.random() * 25 }));
$('resetAll').onclick = () => {
  resetMsg();
  setBase(parseHex(DEF_BASE));
  fg = parseHex(DEF_FG); bg = parseHex(DEF_BG);
  syncContrastText(); renderContrast();
};

// Live language switch: re-render generated labels, keep all colours.
addEventListener('langchange', () => {
  if (convErr) $('convMsg').textContent = I18N.t('invalidValue', { type: convErr });
  else $('convMsg').innerHTML = I18N.t('convHint');
  renderPalettes();
  renderContrast();
});

resetMsg();
setBase(base);
syncContrastText();
renderContrast();

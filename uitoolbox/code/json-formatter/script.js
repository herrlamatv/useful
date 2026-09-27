I18N.add({
  en: {
    docTitle: 'JSON Formatter', back: '← Overview', indent: 'Indent', spaces2: '2 spaces', spaces4: '4 spaces', tab: 'Tab',
    format: 'Format', formatHint: 'Ctrl+Enter', minify: 'Minify', validate: 'Validate', sort: 'Sort',
    sample: 'Load example', clear: 'Clear', input: 'Input', inputPh: 'Paste JSON here …',
    tabCode: 'Code', tabTree: 'Tree', expandAll: 'Expand all', collapseAll: 'Collapse all',
    copy: 'Copy', copied: 'Copied', error: 'Error',
    cursor: 'Ln {line}, Col {col}',
    ready: 'Ready.', noInput: 'No input.', errAt: 'Line {line}, column {col}: {msg}',
    formatted: 'Formatted.', minified: 'Minified.', valid: 'Valid JSON ({type}, {keys} keys, depth {depth}).',
    sorted: 'Keys sorted recursively.', cleared: 'Cleared.', restored: 'Last input restored.',
    expandCap: 'Only the first 5000 nodes were expanded.',
    statIn: 'Input', statOut: 'Output', statKeys: 'Keys', statDepth: 'Depth', statType: 'Type',
    treeItem: '{n} item', treeItems: '{n} items', treeKey: '{n} key', treeKeys: '{n} keys',
    treeEmpty: 'No valid JSON loaded.',
    e_eof: 'Unexpected end of input',
    e_single: 'Single quotes are not allowed in JSON – use "',
    e_char: 'Unexpected character “{c}”',
    e_objTrail: 'Trailing comma after the last entry is not allowed',
    e_keyQuote: 'Expected key in double quotes',
    e_colon: 'Expected colon “:” after the key',
    e_objOpen: 'Object not closed – “}” missing',
    e_objSep: 'Expected “,” or “}”',
    e_arrTrail: 'Trailing comma after the last element is not allowed',
    e_arrOpen: 'Array not closed – “]” missing',
    e_arrSep: 'Expected “,” or “]”',
    e_uEsc: 'Invalid \\u escape sequence',
    e_esc: 'Invalid escape sequence',
    e_ctrl: 'Control characters (e.g. line breaks) are not allowed in strings',
    e_strOpen: 'String not closed – “"” missing',
    e_num: 'Invalid number',
    e_extra: 'Extra characters after the end of the JSON'
  },
  de: {
    docTitle: 'JSON Formatter', back: '← Übersicht', indent: 'Einrückung', spaces2: '2 Leerzeichen', spaces4: '4 Leerzeichen', tab: 'Tab',
    format: 'Formatieren', formatHint: 'Strg+Enter', minify: 'Minifizieren', validate: 'Validieren', sort: 'Sortieren',
    sample: 'Beispiel laden', clear: 'Leeren', input: 'Eingabe', inputPh: 'JSON hier einfügen …',
    tabCode: 'Code', tabTree: 'Baum', expandAll: 'Alle aufklappen', collapseAll: 'Alle zuklappen',
    copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler',
    cursor: 'Z {line}, S {col}',
    ready: 'Bereit.', noInput: 'Keine Eingabe.', errAt: 'Zeile {line}, Spalte {col}: {msg}',
    formatted: 'Formatiert.', minified: 'Minifiziert.', valid: 'Gültiges JSON ({type}, {keys} Schlüssel, Tiefe {depth}).',
    sorted: 'Schlüssel rekursiv sortiert.', cleared: 'Geleert.', restored: 'Letzte Eingabe wiederhergestellt.',
    expandCap: 'Nur die ersten 5000 Knoten aufgeklappt.',
    statIn: 'Eingabe', statOut: 'Ausgabe', statKeys: 'Schlüssel', statDepth: 'Tiefe', statType: 'Typ',
    treeItem: '{n} Element', treeItems: '{n} Elemente', treeKey: '{n} Schlüssel', treeKeys: '{n} Schlüssel',
    treeEmpty: 'Kein gültiges JSON geladen.',
    e_eof: 'Unerwartetes Ende der Eingabe',
    e_single: 'Einfache Anführungszeichen sind in JSON nicht erlaubt – nutze "',
    e_char: 'Unerwartetes Zeichen „{c}“',
    e_objTrail: 'Komma nach dem letzten Eintrag ist nicht erlaubt',
    e_keyQuote: 'Schlüssel in doppelten Anführungszeichen erwartet',
    e_colon: 'Doppelpunkt „:“ nach dem Schlüssel erwartet',
    e_objOpen: 'Objekt nicht geschlossen – „}“ fehlt',
    e_objSep: '„,“ oder „}“ erwartet',
    e_arrTrail: 'Komma nach dem letzten Element ist nicht erlaubt',
    e_arrOpen: 'Array nicht geschlossen – „]“ fehlt',
    e_arrSep: '„,“ oder „]“ erwartet',
    e_uEsc: 'Ungültige \\u-Escape-Sequenz',
    e_esc: 'Ungültige Escape-Sequenz',
    e_ctrl: 'Steuerzeichen (z. B. Zeilenumbruch) im String nicht erlaubt',
    e_strOpen: 'String nicht geschlossen – „"“ fehlt',
    e_num: 'Ungültige Zahl',
    e_extra: 'Zusätzliche Zeichen nach dem Ende des JSON'
  }
});

const $ = id => document.getElementById(id);
const STORE_KEY = 'json-formatter-input';

const input = $('input'), output = $('output'), tree = $('tree'), gutter = $('gutter'), statusEl = $('status');

// Example data per language ("Load example" / fresh start); user input is never replaced on a language switch.
const SAMPLE_EN = {
  project: 'uitoolbox',
  version: 1.4,
  active: true,
  description: 'Small web tools – usable offline',
  author: { name: 'Jane Doe', email: 'jane@example.com', roles: ['admin', 'dev'] },
  tools: [
    { id: 1, name: 'CSS Playground', tags: ['css', 'design'], stars: 42 },
    { id: 2, name: 'Regex Tester', tags: ['regex'], stars: 17.5 },
    { id: 3, name: 'JSON Formatter', tags: [], stars: null },
  ],
  settings: { theme: 'light', language: 'en', limits: { max: 1e6, min: -3 } },
};
const SAMPLE_DE = {
  projekt: 'uitoolbox',
  version: 1.4,
  aktiv: true,
  beschreibung: 'Kleine Web-Tools – offline nutzbar',
  autor: { name: 'Max Mustermann', email: 'max@example.de', rollen: ['admin', 'dev'] },
  tools: [
    { id: 1, name: 'CSS Playground', tags: ['css', 'design'], sterne: 42 },
    { id: 2, name: 'Regex Tester', tags: ['regex'], sterne: 17.5 },
    { id: 3, name: 'JSON Formatter', tags: [], sterne: null },
  ],
  einstellungen: { theme: 'hell', sprache: 'de', limits: { max: 1e6, min: -3 } },
};

let parsed;          // last successfully parsed value
let hasParsed = false;
let analysis = { keys: 0, depth: 0 }; // cached stats of `parsed`
let outText = '';    // plain text of output (for copy)
let tab = 'code';
let statusFn = () => I18N.t('ready'); // re-evaluated on language switch
const T = (key, vars) => () => I18N.t(key, vars);

// ---------- Helpers ----------
const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const bytes = s => new TextEncoder().encode(s).length;
const fmtBytes = n => n < 1024 ? n + ' B' : n < 1048576 ? (n / 1024).toFixed(1) + ' KB' : (n / 1048576).toFixed(2) + ' MB';
const indentVal = () => $('indent').value === 'tab' ? '\t' : +$('indent').value;
const isObj = v => v !== null && typeof v === 'object';

function lineCol(text, pos) {
  const before = text.slice(0, pos).split('\n');
  return { line: before.length, col: before[before.length - 1].length + 1 };
}

// Small validator to find the exact error position (browser messages vary)
function findError(t) {
  let i = 0;
  const fail = (key, vars) => { throw { pos: i, key, vars }; };
  const ws = () => { while (i < t.length && ' \t\n\r'.includes(t[i])) i++; };
  const expect = (c, msg) => { if (t[i] !== c) fail(msg); i++; };

  function value() {
    ws();
    const c = t[i];
    if (c === undefined) fail('e_eof');
    if (c === '{') return obj();
    if (c === '[') return arr();
    if (c === '"') return str();
    if (c === '-' || (c >= '0' && c <= '9')) return num();
    for (const lit of ['true', 'false', 'null']) if (t.startsWith(lit, i)) { i += lit.length; return; }
    if (c === "'") fail('e_single');
    fail('e_char', { c });
  }
  function obj() {
    i++; ws();
    if (t[i] === '}') { i++; return; }
    for (;;) {
      ws();
      if (t[i] === '}') fail('e_objTrail');
      if (t[i] !== '"') fail('e_keyQuote');
      str(); ws();
      expect(':', 'e_colon');
      value(); ws();
      if (t[i] === ',') { i++; continue; }
      if (t[i] === '}') { i++; return; }
      fail(t[i] === undefined ? 'e_objOpen' : 'e_objSep');
    }
  }
  function arr() {
    i++; ws();
    if (t[i] === ']') { i++; return; }
    for (;;) {
      ws();
      if (t[i] === ']') fail('e_arrTrail');
      value(); ws();
      if (t[i] === ',') { i++; continue; }
      if (t[i] === ']') { i++; return; }
      fail(t[i] === undefined ? 'e_arrOpen' : 'e_arrSep');
    }
  }
  function str() {
    i++;
    while (i < t.length) {
      const c = t[i];
      if (c === '"') { i++; return; }
      if (c === '\\') {
        const e = t[i + 1];
        if (e === 'u') {
          if (!/^[0-9a-fA-F]{4}$/.test(t.substr(i + 2, 4))) fail('e_uEsc');
          i += 6; continue;
        }
        if (!'"\\/bfnrt'.includes(e) || e === undefined) fail('e_esc');
        i += 2; continue;
      }
      if (c < ' ') fail('e_ctrl');
      i++;
    }
    fail('e_strOpen');
  }
  function num() {
    const re = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
    re.lastIndex = i;
    const m = re.exec(t);
    if (!m) fail('e_num');
    i += m[0].length;
  }

  try {
    value(); ws();
    if (i < t.length) fail('e_extra');
    return null;
  } catch (e) {
    return e && typeof e.pos === 'number' ? e : { pos: 0, msg: String(e) };
  }
}

// Parse input; on error show message + jump to position
function parseInput() {
  const text = input.value;
  markGutter(0);
  if (!text.trim()) { setStatus(T('noInput'), ''); return false; }
  try {
    parsed = JSON.parse(text);
    hasParsed = true;
    analysis = analyze(parsed);
    return true;
  } catch (err) {
    hasParsed = false;
    const e = findError(text) || { pos: 0, msg: err.message };
    const { line, col } = lineCol(text, e.pos);
    setStatus(() => I18N.t('errAt', { line, col, msg: e.key ? I18N.t(e.key, e.vars) : e.msg }), 'err');
    showError(e.pos, line);
    return false;
  }
}

function showError(pos, line) {
  markGutter(line);
  input.focus();
  input.setSelectionRange(pos, Math.min(pos + 1, input.value.length));
  const lh = parseFloat(getComputedStyle(input).lineHeight) || 20;
  input.scrollTop = Math.max(0, (line - 4) * lh);
  syncGutter();
}

// msg is a function returning the (translated) text, so it can be re-rendered on language switch
function setStatus(msg, cls) {
  statusFn = msg;
  statusEl.textContent = msg();
  statusEl.className = 'status ' + cls;
}

// ---------- Stats ----------
function analyze(v, depth = 0) {
  if (!isObj(v)) return { keys: 0, depth };
  let keys = Array.isArray(v) ? 0 : Object.keys(v).length, max = depth + 1;
  for (const k in v) {
    const r = analyze(v[k], depth + 1);
    keys += r.keys;
    max = Math.max(max, r.depth);
  }
  return { keys, depth: max };
}

function renderStats() {
  const inB = bytes(input.value);
  const parts = [`${I18N.t('statIn')}: <b>${fmtBytes(inB)}</b>`];
  if (outText) {
    const outB = bytes(outText);
    const diff = inB ? Math.round((outB - inB) / inB * 100) : 0;
    parts.push(`${I18N.t('statOut')}: <b>${fmtBytes(outB)}</b> (${diff > 0 ? '+' : ''}${diff}%)`);
  }
  if (hasParsed) {
    const { keys, depth } = analysis;
    parts.push(`${I18N.t('statKeys')}: <b>${keys}</b>`, `${I18N.t('statDepth')}: <b>${depth}</b>`, `${I18N.t('statType')}: <b>${typeName(parsed)}</b>`);
  }
  $('stats').innerHTML = parts.map(p => `<span>${p}</span>`).join('');
}

const typeName = v => v === null ? 'null' : Array.isArray(v) ? 'array' : typeof v;

// ---------- Output ----------
function highlight(json) {
  return esc(json).replace(
    /("(?:\\u[0-9a-fA-F]{4}|\\[^u]|[^\\"])*")(\s*:)?|\b(true|false)\b|\bnull\b|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|[{}[\],]/g,
    (m, str, colon, bool) => {
      if (str) return colon ? `<span class="prop">${str}</span><span class="mute">${colon}</span>` : `<span class="str">${str}</span>`;
      if (bool) return `<span class="key">${m}</span>`;
      if (m === 'null') return `<span class="key">${m}</span>`;
      if (/[{}[\],]/.test(m)) return `<span class="mute">${m}</span>`;
      return `<span class="num">${m}</span>`;
    });
}

function setOutput(text, wrap) {
  outText = text;
  output.classList.toggle('wrap', !!wrap);
  output.innerHTML = highlight(text);
  if (tab === 'tree') renderTree();
  renderStats();
}

function sortKeys(v) {
  if (Array.isArray(v)) return v.map(sortKeys);
  if (!isObj(v)) return v;
  return Object.keys(v).sort((a, b) => a.localeCompare(b)).reduce((o, k) => (o[k] = sortKeys(v[k]), o), {});
}

// ---------- Tree view (children built lazily on first open) ----------
function leafHtml(v) {
  const t = typeName(v);
  const cls = { string: 'str', number: 'num', boolean: 'key', null: 'key' }[t] || 'str';
  const txt = t === 'string' ? JSON.stringify(v) : String(v);
  return `<span class="${cls}">${esc(txt)}</span><span class="type">${t}</span>`;
}

const metaText = (n, isArr) => I18N.t((isArr ? 'treeItem' : 'treeKey') + (n === 1 ? '' : 's'), { n });

function makeNode(key, v, open) {
  const keyHtml = key === null ? '' : `<span class="prop">${esc(String(key))}</span><span class="mute">: </span>`;
  if (!isObj(v)) {
    const div = document.createElement('div');
    div.className = 'leaf';
    div.innerHTML = keyHtml + leafHtml(v);
    return div;
  }
  const isArr = Array.isArray(v);
  const n = isArr ? v.length : Object.keys(v).length;
  const det = document.createElement('details');
  det.innerHTML = `<summary>${keyHtml}<span class="mute">${isArr ? '[…]' : '{…}'}</span><span class="meta" data-n="${n}" data-arr="${isArr ? 1 : ''}">${metaText(n, isArr)}</span></summary>`;
  det._value = v;
  det.addEventListener('toggle', () => { if (det.open) buildChildren(det); });
  if (open) det.open = true;
  return det;
}

function buildChildren(det) {
  if (det._built) return;
  det._built = true;
  const v = det._value;
  const frag = document.createDocumentFragment();
  for (const k of Object.keys(v)) frag.appendChild(makeNode(Array.isArray(v) ? +k : k, v[k], false));
  det.appendChild(frag);
}

function renderTree() {
  tree.innerHTML = '';
  if (!hasParsed) { tree.innerHTML = `<span class="muted">${I18N.t('treeEmpty')}</span>`; return; }
  const root = makeNode(null, parsed, true);
  tree.appendChild(root);
  if (root.tagName === 'DETAILS') buildChildren(root);
}

// Expand all (capped so huge files don't freeze the page)
$('expandAll').onclick = () => {
  let budget = 5000;
  const walk = det => {
    if (budget-- <= 0) return;
    buildChildren(det);
    det.open = true;
    for (const c of det.children) if (c.tagName === 'DETAILS') walk(c);
  };
  const root = tree.querySelector('details');
  if (root) walk(root);
  if (budget <= 0) setStatus(T('expandCap'), '');
};
$('collapseAll').onclick = () => {
  tree.querySelectorAll('details').forEach(d => d.open = false);
  const root = tree.querySelector('details');
  if (root) root.open = true;
};

document.querySelectorAll('.tab').forEach(b => b.onclick = () => {
  tab = b.dataset.tab;
  document.querySelectorAll('.tab').forEach(x => x.classList.toggle('active', x === b));
  output.hidden = tab !== 'code';
  tree.hidden = tab !== 'tree';
  $('treeActions').hidden = tab !== 'tree';
  if (tab === 'tree') renderTree();
});

// ---------- Actions ----------
function doFormat(value) {
  const txt = JSON.stringify(value, null, indentVal());
  setOutput(txt ?? '');
  return txt;
}

$('format').onclick = () => {
  if (!parseInput()) return renderStats();
  doFormat(parsed);
  setStatus(T('formatted'), 'ok');
};

$('minify').onclick = () => {
  if (!parseInput()) return renderStats();
  setOutput(JSON.stringify(parsed), true);
  setStatus(T('minified'), 'ok');
};

$('validate').onclick = () => {
  if (!parseInput()) return renderStats();
  const { keys, depth } = analysis;
  const type = typeName(parsed);
  setStatus(T('valid', { type, keys, depth }), 'ok');
  if (tab === 'tree') renderTree();
  renderStats();
};

$('sort').onclick = () => {
  if (!parseInput()) return renderStats();
  parsed = sortKeys(parsed);
  doFormat(parsed);
  setStatus(T('sorted'), 'ok');
};

$('sample').onclick = () => {
  input.value = JSON.stringify(I18N.lang === 'de' ? SAMPLE_DE : SAMPLE_EN);
  onInput();
  $('format').click();
};

$('clear').onclick = () => {
  input.value = '';
  hasParsed = false;
  outText = '';
  output.innerHTML = '';
  tree.innerHTML = '';
  onInput();
  setStatus(T('cleared'), '');
  input.focus();
};

$('indent').onchange = () => { if (hasParsed && outText && !output.classList.contains('wrap')) doFormat(parsed); };

$('copy').onclick = async e => {
  try {
    await navigator.clipboard.writeText(outText || input.value);
    e.target.textContent = I18N.t('copied');
  } catch {
    e.target.textContent = I18N.t('error');
  }
  setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
};

// ---------- Input: gutter, cursor, storage ----------
let errLine = 0;
function renderGutter() {
  const n = input.value.split('\n').length;
  let html = '';
  for (let i = 1; i <= n; i++) html += i === errLine ? `<span class="err">${i}</span>\n` : i + '\n';
  gutter.innerHTML = html;
  syncGutter();
}
function markGutter(line) { errLine = line; renderGutter(); }
function syncGutter() { gutter.scrollTop = input.scrollTop; }

function updateCursor() {
  const { line, col } = lineCol(input.value, input.selectionStart);
  $('cursorPos').textContent = I18N.t('cursor', { line, col });
}

let saveTimer;
function onInput() {
  if (errLine) errLine = 0;
  renderGutter();
  updateCursor();
  renderStats();
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try { localStorage.setItem(STORE_KEY, input.value); } catch { /* storage unavailable */ }
  }, 300);
}

input.addEventListener('input', onInput);
input.addEventListener('scroll', syncGutter);
input.addEventListener('click', updateCursor);
input.addEventListener('keyup', updateCursor);
input.addEventListener('keydown', e => {
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); $('format').click(); }
  // Tab inserts indentation instead of leaving the field
  if (e.key === 'Tab' && !e.shiftKey) {
    e.preventDefault();
    const ind = indentVal();
    input.setRangeText(ind === '\t' ? '\t' : ' '.repeat(ind), input.selectionStart, input.selectionEnd, 'end');
    onInput();
  }
});

// ---------- Language switch: re-render texts in place, keep input, output and tree state ----------
addEventListener('langchange', () => {
  statusEl.textContent = statusFn();
  updateCursor();
  renderStats();
  tree.querySelectorAll('.meta').forEach(m => { m.textContent = metaText(+m.dataset.n, !!m.dataset.arr); });
  if (!hasParsed && tab === 'tree') renderTree();
});

// ---------- Init ----------
let saved = null;
try { saved = localStorage.getItem(STORE_KEY); } catch { /* storage unavailable */ }
if (saved) {
  input.value = saved;
  onInput();
  if (parseInput()) { doFormat(parsed); setStatus(T('restored'), ''); }
} else {
  $('sample').click();
}

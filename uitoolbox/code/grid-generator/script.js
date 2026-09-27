I18N.add({
  en: {
    docTitle: 'Grid Generator', back: '← Overview', heading: 'Grid Generator',
    clearAreas: 'Clear areas', reset: 'Reset', gridTitle: 'Grid', cols: 'Columns', rows: 'Rows',
    colSizes: 'Column sizes', rowSizes: 'Row sizes', colN: 'Column {n}', rowN: 'Row {n}',
    areas: 'Areas', areaHint: 'Drag across empty cells in the preview to create an area.',
    noAreas: 'No areas yet.', deleteArea: 'Delete area',
    cellTaken: 'Cell already belongs to “{name}”.', overlaps: 'Overlaps an existing area.',
    notCreated: 'Area overlaps – not created.',
    copy: 'Copy', copied: 'Copied', error: 'Error'
  },
  de: {
    docTitle: 'Grid Generator', back: '← Übersicht', heading: 'Grid Generator',
    clearAreas: 'Bereiche leeren', reset: 'Reset', gridTitle: 'Raster', cols: 'Spalten', rows: 'Zeilen',
    colSizes: 'Spaltengrößen', rowSizes: 'Zeilengrößen', colN: 'Spalte {n}', rowN: 'Zeile {n}',
    areas: 'Bereiche', areaHint: 'Ziehe in der Vorschau über leere Zellen, um einen Bereich anzulegen.',
    noAreas: 'Noch keine Bereiche.', deleteArea: 'Bereich löschen',
    cellTaken: 'Zelle gehört bereits zu „{name}“.', overlaps: 'Überschneidet sich mit einem vorhandenen Bereich.',
    notCreated: 'Bereich überschneidet sich – nicht angelegt.',
    copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler'
  }
});

const $ = id => document.getElementById(id);
const grid = $('grid'), cssCode = $('cssCode'), htmlCode = $('htmlCode');

let st, counter, cells = [];
let drag = null; // { r0, c0, r, c, box }

function reset() {
  st = { cols: 3, rows: 3, colGap: 10, rowGap: 10, colSizes: [], rowSizes: [], areas: [] };
  counter = 0;
  fitSizes();
}

// Keep size arrays in sync with the count, clip areas to the grid
function fitSizes() {
  while (st.colSizes.length < st.cols) st.colSizes.push('1fr');
  while (st.rowSizes.length < st.rows) st.rowSizes.push('1fr');
  st.colSizes.length = st.cols;
  st.rowSizes.length = st.rows;
  st.areas = st.areas.filter(a => a.r1 <= st.rows && a.c1 <= st.cols);
  st.areas.forEach(a => { a.r2 = Math.min(a.r2, st.rows); a.c2 = Math.min(a.c2, st.cols); });
}

const validSize = v => CSS.supports('grid-template-columns', v);
const colorOf = i => `hsl(${(i * 67 + 190) % 360} 34% 58% / .9)`;
const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const overlaps = (a, b) => a.r1 <= b.r2 && b.r1 <= a.r2 && a.c1 <= b.c2 && b.c1 <= a.c2;
const rect = (r0, c0, r, c) => ({ r1: Math.min(r0, r), r2: Math.max(r0, r), c1: Math.min(c0, c), c2: Math.max(c0, c) });
const areaPos = a => `${a.r1} / ${a.c1} / ${a.r2 + 1} / ${a.c2 + 1}`;

// Sanitize to a valid CSS ident for grid-template-areas
const sanitize = v => {
  let n = v.trim().replace(/[^a-zA-Z0-9_-]/g, '-');
  if (/^(\d|-\d|--)/.test(n)) n = 'a' + n;
  return n;
};

// Compress "1fr 1fr 1fr" → "repeat(3, 1fr)"
const track = arr => arr.length > 1 && arr.every(v => v === arr[0]) ? `repeat(${arr.length}, ${arr[0]})` : arr.join(' ');

function sizeInputs(boxId, arr, label) {
  const wrap = $(boxId);
  wrap.innerHTML = '';
  arr.forEach((v, i) => {
    const row = document.createElement('div');
    row.className = 'row';
    row.innerHTML = `<span>${I18N.t(label, { n: i + 1 })}</span>`;
    const inp = document.createElement('input');
    inp.type = 'text';
    inp.value = v;
    inp.oninput = () => {
      const val = inp.value.trim();
      const ok = val && validSize(val);
      inp.classList.toggle('invalid', !ok);
      if (ok) { arr[i] = val; renderGrid(); renderCode(); }
    };
    row.appendChild(inp);
    wrap.appendChild(row);
  });
}

function renderList() {
  const list = $('areaList');
  list.innerHTML = '';
  if (!st.areas.length) list.innerHTML = `<p class="hint" data-i18n="noAreas">${I18N.t('noAreas')}</p>`;
  st.areas.forEach((a, i) => {
    const row = document.createElement('div');
    row.className = 'area-item';
    row.innerHTML = `<span class="dot" style="background:${colorOf(i)}"></span>`;
    const inp = document.createElement('input');
    inp.value = a.name;
    inp.title = `.item${i + 1}`;
    inp.oninput = () => {
      const n = sanitize(inp.value);
      const ok = n && !st.areas.some(o => o !== a && o.name === n);
      inp.classList.toggle('invalid', !ok);
      if (ok) { a.name = n; renderGrid(); renderCode(); }
    };
    inp.onblur = () => { inp.value = a.name; inp.classList.remove('invalid'); };
    const del = document.createElement('button');
    del.textContent = '×';
    del.title = I18N.t('deleteArea');
    del.dataset.i18nAttr = 'title:deleteArea';
    del.onclick = () => { st.areas.splice(i, 1); renderAll(); };
    row.append(inp, del);
    list.appendChild(row);
  });
}

function renderGrid() {
  grid.style.gridTemplateColumns = st.colSizes.map(v => validSize(v) ? v : '1fr').join(' ');
  grid.style.gridTemplateRows = st.rowSizes.map(v => validSize(v) ? v : '1fr').join(' ');
  grid.style.columnGap = st.colGap + 'px';
  grid.style.rowGap = st.rowGap + 'px';
  grid.innerHTML = '';
  cells = [];
  for (let r = 1; r <= st.rows; r++) {
    cells[r] = [];
    for (let c = 1; c <= st.cols; c++) {
      const el = document.createElement('div');
      el.className = 'cell';
      el.style.gridArea = `${r} / ${c}`;
      grid.appendChild(el);
      cells[r][c] = el;
    }
  }
  st.areas.forEach((a, i) => {
    const el = document.createElement('div');
    el.className = 'area';
    el.style.gridArea = areaPos(a);
    el.style.background = colorOf(i);
    el.textContent = a.name;
    grid.appendChild(el);
  });
}

function renderCode() {
  const line = (k, v) => `  <span class="prop">${k}</span>: ${v};`;
  const out = [line('display', 'grid'), line('grid-template-columns', track(st.colSizes)), line('grid-template-rows', track(st.rowSizes))];
  out.push(line('gap', st.rowGap === st.colGap ? st.rowGap + 'px' : `${st.rowGap}px ${st.colGap}px`));
  if (st.areas.length) {
    // Build the name matrix, pad columns for readability
    const m = Array.from({ length: st.rows }, () => Array(st.cols).fill('.'));
    st.areas.forEach(a => {
      for (let r = a.r1; r <= a.r2; r++) for (let c = a.c1; c <= a.c2; c++) m[r - 1][c - 1] = a.name;
    });
    const w = Array.from({ length: st.cols }, (_, c) => Math.max(...m.map(row => row[c].length)));
    const rows = m.map(row => `    "${row.map((n, c) => n.padEnd(w[c])).join(' ').trimEnd()}"`);
    out.push(`  <span class="prop">grid-template-areas</span>:\n${rows.join('\n')};`);
  }
  let text = `.container {\n${out.join('\n')}\n}`;
  st.areas.forEach((a, i) => { text += `\n\n.item${i + 1} { <span class="prop">grid-area</span>: ${a.name}; }`; });
  cssCode.innerHTML = text;

  const html = ['<div class="container">', ...st.areas.map((a, i) => `  <div class="item${i + 1}">${a.name}</div>`), '</div>'];
  htmlCode.innerHTML = esc(html.join('\n'));
}

function renderControls() {
  for (const k of ['cols', 'rows', 'colGap', 'rowGap']) {
    $(k).value = st[k];
    $(k + 'Out').textContent = st[k] + (k.includes('Gap') ? 'px' : '');
  }
  sizeInputs('colSizes', st.colSizes, 'colN');
  sizeInputs('rowSizes', st.rowSizes, 'rowN');
}

function renderAll() {
  renderControls();
  renderList();
  renderGrid();
  renderCode();
}

// Slider controls
['cols', 'rows'].forEach(k => $(k).oninput = e => { st[k] = +e.target.value; fitSizes(); renderAll(); });
['colGap', 'rowGap'].forEach(k => $(k).oninput = e => {
  st[k] = +e.target.value;
  $(k + 'Out').textContent = st[k] + 'px';
  renderGrid(); renderCode();
});

// Cell under a point (clamped to the grid)
function cellAt(x, y) {
  let c = 1, r = 1;
  for (let i = 1; i <= st.cols; i++) if (x >= cells[1][i].getBoundingClientRect().left) c = i;
  for (let i = 1; i <= st.rows; i++) if (y >= cells[i][1].getBoundingClientRect().top) r = i;
  return [r, c];
}

// Status is kept as key + vars so it can be re-translated on a language switch.
let statusMsg = null;
const setStatus = (key, vars) => { statusMsg = key ? [key, vars] : null; $('status').textContent = key ? I18N.t(key, vars) : ''; };

// Drag to create areas (pointer events → mouse + touch)
grid.addEventListener('pointerdown', e => {
  if (!e.target.classList.contains('cell')) return;
  const [r, c] = cellAt(e.clientX, e.clientY);
  const hit = st.areas.find(a => overlaps(a, { r1: r, r2: r, c1: c, c2: c }));
  if (hit) { setStatus('cellTaken', { name: hit.name }); return; }
  e.preventDefault();
  grid.setPointerCapture(e.pointerId);
  const box = document.createElement('div');
  box.className = 'sel-box';
  grid.appendChild(box);
  drag = { r0: r, c0: c, r, c, box };
  updateDrag();
});

function dragRect() { return rect(drag.r0, drag.c0, drag.r, drag.c); }
function updateDrag() {
  const a = dragRect();
  drag.box.style.gridArea = areaPos(a);
  const bad = st.areas.some(o => overlaps(o, a));
  drag.box.classList.toggle('bad', bad);
  drag.box.textContent = `${a.r2 - a.r1 + 1}×${a.c2 - a.c1 + 1}`;
  setStatus(bad ? 'overlaps' : null);
}

grid.addEventListener('pointermove', e => {
  if (!drag) return;
  const [r, c] = cellAt(e.clientX, e.clientY);
  if (r !== drag.r || c !== drag.c) { drag.r = r; drag.c = c; updateDrag(); }
});

function endDrag(commit) {
  if (!drag) return;
  const a = dragRect();
  drag.box.remove();
  drag = null;
  if (!commit) return;
  if (st.areas.some(o => overlaps(o, a))) { setStatus('notCreated'); return; }
  let name;
  do { name = 'item' + ++counter; } while (st.areas.some(o => o.name === name));
  st.areas.push({ name, ...a });
  setStatus(null);
  renderList(); renderGrid(); renderCode();
}
grid.addEventListener('pointerup', () => endDrag(true));
grid.addEventListener('pointercancel', () => endDrag(false));

$('clearAreas').onclick = () => { st.areas = []; counter = 0; setStatus(null); renderAll(); };
$('resetAll').onclick = () => { reset(); setStatus(null); renderAll(); };

// Copy buttons
function copyBtn(btnId, pre) {
  $(btnId).onclick = async e => {
    try {
      await navigator.clipboard.writeText(pre.textContent);
      e.target.textContent = I18N.t('copied');
    } catch {
      e.target.textContent = I18N.t('error');
    }
    setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
  };
}
copyBtn('copyCss', cssCode);
copyBtn('copyHtml', htmlCode);

reset();
renderAll();

// Language switch: static texts, the empty-list hint and delete titles are handled by I18N.apply();
// relabel the size rows in place (keeps half-typed values) and re-translate the status line.
addEventListener('langchange', () => {
  [['colSizes', 'colN'], ['rowSizes', 'rowN']].forEach(([id, key]) =>
    $(id).querySelectorAll('.row > span').forEach((s, i) => { s.textContent = I18N.t(key, { n: i + 1 }); }));
  if (statusMsg) setStatus(...statusMsg);
  ['copyCss', 'copyHtml'].forEach(id => { $(id).textContent = I18N.t('copy'); });
});

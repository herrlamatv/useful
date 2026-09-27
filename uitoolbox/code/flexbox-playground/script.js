I18N.add({
  en: {
    docTitle: 'Flexbox Playground', back: '← Overview', heading: 'Flexbox Playground',
    addItem: 'Add item', removeItem: 'Remove item', reset: 'Reset',
    container: 'Container', item: 'Item', itemN: 'Item {n}',
    itemHint: 'Click an item in the preview to edit it.',
    customised: 'customised', defaultsOnly: 'Item {n}: defaults only',
    generated: 'Generated CSS', copy: 'Copy', copied: 'Copied', error: 'Error'
  },
  de: {
    docTitle: 'Flexbox Playground', back: '← Übersicht', heading: 'Flexbox Playground',
    addItem: 'Item hinzufügen', removeItem: 'Item entfernen', reset: 'Reset',
    container: 'Container', item: 'Item', itemN: 'Item {n}',
    itemHint: 'Klicke auf ein Item in der Vorschau, um es zu bearbeiten.',
    customised: 'angepasst', defaultsOnly: 'Item {n}: nur Standardwerte',
    generated: 'Generiertes CSS', copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler'
  }
});

// Container button groups: [prop, options] – first option = default
const GROUPS = [
  ['flex-direction', ['row', 'row-reverse', 'column', 'column-reverse']],
  ['flex-wrap', ['nowrap', 'wrap', 'wrap-reverse']],
  ['justify-content', ['flex-start', 'flex-end', 'center', 'space-between', 'space-around', 'space-evenly']],
  ['align-items', ['stretch', 'flex-start', 'flex-end', 'center', 'baseline']],
  ['align-content', ['normal', 'flex-start', 'flex-end', 'center', 'space-between', 'space-around', 'space-evenly', 'stretch']],
];
const ALIGN_SELF = ['auto', 'flex-start', 'flex-end', 'center', 'baseline', 'stretch'];
const ITEM_DEFAULT = { grow: 0, shrink: 1, basis: 'auto', alignSelf: 'auto', order: 0 };
const MAX_ITEMS = 20;

const $ = id => document.getElementById(id);
const container = $('container'), codeEl = $('code');

let box, items, sel;
function reset() {
  box = { gap: 10 };
  GROUPS.forEach(([p, opts]) => box[p] = opts[0]);
  items = Array.from({ length: 5 }, () => ({ ...ITEM_DEFAULT }));
  sel = null;
}
reset();

// Build container controls
const segs = {};
GROUPS.forEach(([prop, opts]) => {
  const g = document.createElement('div');
  g.className = 'group';
  g.innerHTML = `<span class="label">${prop}</span>`;
  const seg = document.createElement('div');
  seg.className = 'seg';
  opts.forEach(o => {
    const b = document.createElement('button');
    b.textContent = o;
    b.onclick = () => { box[prop] = o; render(); };
    seg.appendChild(b);
  });
  segs[prop] = seg;
  g.appendChild(seg);
  $('containerCtrls').appendChild(g);
});
const gapRow = document.createElement('div');
gapRow.className = 'row';
gapRow.innerHTML = '<span>gap</span><input type="range" id="gap" min="0" max="60" step="1"><output id="gapOut"></output>';
$('containerCtrls').appendChild(gapRow);
$('gap').oninput = e => { box.gap = +e.target.value; render(); };

// Item align-self buttons
ALIGN_SELF.forEach(o => {
  const b = document.createElement('button');
  b.textContent = o;
  b.onclick = () => { sel.alignSelf = o; render(); };
  $('alignSelf').appendChild(b);
});

const markActive = (seg, val) => seg.querySelectorAll('button').forEach(b => b.classList.toggle('on', b.textContent === val));

function itemCSS(it) {
  const css = {};
  if (it.grow !== 0) css['flex-grow'] = it.grow;
  if (it.shrink !== 1) css['flex-shrink'] = it.shrink;
  if (it.basis !== 'auto') css['flex-basis'] = it.basis;
  if (it.alignSelf !== 'auto') css['align-self'] = it.alignSelf;
  if (it.order !== 0) css.order = it.order;
  return css;
}

function renderItems() {
  container.innerHTML = '';
  items.forEach((it, i) => {
    const el = document.createElement('div');
    el.className = 'item' + (it === sel ? ' sel' : '');
    el.style.background = `hsl(${(i * 47 + 200) % 360} 34% 60%)`;
    el.style.padding = `${12 + (i % 3) * 10}px ${16 + (i % 4) * 8}px`;
    el.style.fontSize = `${14 + (i % 3) * 5}px`;
    const css = itemCSS(it);
    for (const k in css) el.style.setProperty(k, css[k]);
    el.innerHTML = `${i + 1}${Object.keys(css).length ? `<small>${I18N.t('customised')}</small>` : ''}`;
    el.onclick = e => { e.stopPropagation(); sel = it === sel ? null : it; render(); };
    container.appendChild(el);
  });
}

function render() {
  // Container styles
  GROUPS.forEach(([p]) => { container.style.setProperty(p, box[p]); markActive(segs[p], box[p]); });
  container.style.gap = box.gap + 'px';
  $('gap').value = box.gap; $('gapOut').textContent = box.gap + 'px';
  renderItems();

  // Item panel
  $('itemCtrls').hidden = !sel;
  $('itemHint').hidden = !!sel;
  $('itemTitle').textContent = sel ? I18N.t('itemN', { n: items.indexOf(sel) + 1 }) : I18N.t('item');
  if (sel) {
    $('grow').value = sel.grow; $('growOut').textContent = sel.grow;
    $('shrink').value = sel.shrink; $('shrinkOut').textContent = sel.shrink;
    if (document.activeElement !== $('basis')) { $('basis').value = sel.basis; $('basis').classList.remove('invalid'); }
    if (document.activeElement !== $('order')) $('order').value = sel.order;
    markActive($('alignSelf'), sel.alignSelf);
  }
  $('addItem').disabled = items.length >= MAX_ITEMS;
  $('removeItem').disabled = items.length <= 1;

  // Code
  const line = (k, v) => `  <span class="prop">${k}</span>: ${v};`;
  const out = [line('display', 'flex')];
  GROUPS.forEach(([p, opts]) => { if (box[p] !== opts[0]) out.push(line(p, box[p])); });
  if (box.gap) out.push(line('gap', box.gap + 'px'));
  let text = `.container {\n${out.join('\n')}\n}`;
  if (sel) {
    const n = items.indexOf(sel) + 1;
    const css = itemCSS(sel);
    const body = Object.entries(css).map(([k, v]) => line(k, v)).join('\n');
    text += body ? `\n\n.item:nth-child(${n}) {\n${body}\n}` : `\n\n<span class="mute">/* ${I18N.t('defaultsOnly', { n })} */</span>`;
  }
  codeEl.innerHTML = text;
}

// Item controls
$('grow').oninput = e => { sel.grow = +e.target.value; render(); };
$('shrink').oninput = e => { sel.shrink = +e.target.value; render(); };
$('basis').oninput = e => {
  const v = e.target.value.trim() || 'auto';
  const ok = CSS.supports('flex-basis', v);
  e.target.classList.toggle('invalid', !ok);
  if (ok) { sel.basis = v; render(); }
};
$('order').oninput = e => { sel.order = parseInt(e.target.value, 10) || 0; render(); };

container.onclick = () => { if (sel) { sel = null; render(); } };

$('addItem').onclick = () => { if (items.length < MAX_ITEMS) { items.push({ ...ITEM_DEFAULT }); render(); } };
$('removeItem').onclick = () => {
  if (items.length <= 1) return;
  items.splice(sel ? items.indexOf(sel) : items.length - 1, 1);
  sel = null;
  render();
};
$('resetAll').onclick = () => { reset(); $('basis').classList.remove('invalid'); render(); };

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

// Static texts are relabelled by I18N.apply(); re-render the generated ones (state is kept).
addEventListener('langchange', () => { $('copy').textContent = I18N.t('copy'); render(); });

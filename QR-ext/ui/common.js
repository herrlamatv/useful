/* QR-ext - by herrlamatv
 * Gemeinsame Bausteine für Popup und große Ansicht (aus chroofer übernommen).
 */

export function el(tag, attrs = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') node.className = v;
    else if (k === 'text') node.textContent = v;
    else if (k.startsWith('on') && typeof v === 'function') node.addEventListener(k.slice(2), v);
    else if (v !== null && v !== undefined && v !== false) node.setAttribute(k, v === true ? '' : v);
  }
  for (const child of [].concat(children)) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child.nodeType ? child : document.createTextNode(String(child)));
  }
  return node;
}

/* --------------------------- iOS-Bausteine --------------------------- */

/** Schalter im iOS-Stil. */
export function toggle(checked, onChange, { disabled = false, title = '' } = {}) {
  const input = el('input', { type: 'checkbox', checked: !!checked, disabled, title });
  input.addEventListener('change', () => onChange(input.checked));
  return el('label', { class: 'toggle' }, [input, el('span', { class: 'track' })]);
}

/** Zeile mit Titel, Beschreibung und Bedienelement rechts. */
export function row(title, description, control) {
  return el('div', { class: 'item' }, [
    el('div', { class: 'txt' }, [
      el('div', { class: 't' }, title),
      description ? el('div', { class: 'd' }, description) : null
    ]),
    control
  ]);
}

/** Zeile mit Schalter. */
export function toggleRow(title, description, checked, onChange, opts) {
  return row(title, description, toggle(checked, onChange, opts));
}

/** Segmentierte Auswahl. `items` = [[wert, beschriftung], …] */
export function segmented(items, current, onPick, disabled = false) {
  const wrap = el('div', { class: 'seg' });
  for (const [value, label, title] of items) {
    wrap.append(
      el(
        'button',
        {
          type: 'button',
          disabled,
          title: title || null,
          'aria-pressed': String(value === current),
          onclick: () => {
            if (value !== current) onPick(value);
          }
        },
        label
      )
    );
  }
  return wrap;
}

/** Knopftext kurz austauschen ("Kopiert ✓") und zurücksetzen. */
export function flash(button, text, ms = 1300) {
  if (!button.dataset.label) button.dataset.label = button.textContent;
  button.textContent = text;
  clearTimeout(button._flash);
  button._flash = setTimeout(() => {
    button.textContent = button.dataset.label;
    delete button.dataset.label;
  }, ms);
}

/** Datei im Browser speichern (Blob oder Text). */
export function download(data, filename, mime) {
  const blob = data instanceof Blob ? data : new Blob([data], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = el('a', { href: url, download: filename });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

/** Wartet `ms` nach dem letzten Aufruf, dann einmal ausführen. */
export function debounce(fn, ms) {
  let timer = 0;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

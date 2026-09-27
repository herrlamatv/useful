I18N.add({
  en: {
    docTitle: 'Regex Tester', back: '← Overview', loadExample: 'Load example …', clear: 'Clear',
    pattern: 'Pattern', patternPh: 'Regular expression …',
    flagG: 'global – all matches', flagI: 'ignoreCase – case-insensitive', flagM: 'multiline – ^ and $ per line',
    flagS: 'dotAll – . also matches line breaks', flagU: 'unicode', flagY: 'sticky – only from lastIndex',
    testText: 'Test text', liveHint: 'Matches are highlighted live', replace: 'Replace',
    replacePh: 'Replacement, e.g. [$1] or $<name> or $&', preview: 'Preview',
    copy: 'Copy', copied: 'Copied', error: 'Error', errorPrefix: 'Error: ',
    matches: 'Matches', thIndex: 'Index', thMatch: 'Match', thGroups: 'Groups',
    count: '{n} matches', noMatches: 'No matches', empty: '(empty)', more: '… and {n} more',
    insertTok: 'Insert “{tok}”',
    pr_email: 'Email', pr_url: 'URL', pr_date: 'Date', pr_phone: 'Phone number', pr_ipv4: 'IPv4', pr_hex: 'Hex colour',
    cg_classes: 'Character classes', cg_anchors: 'Anchors', cg_quant: 'Quantifiers', cg_groups: 'Groups',
    cg_look: 'Lookaround', cg_special: 'Special characters', cg_replace: 'Replace',
    c_any: 'any character', c_digit: 'digit', c_nondigit: 'non-digit', c_word: 'word character', c_nonword: 'non-word character',
    c_space: 'whitespace', c_nonspace: 'non-whitespace', c_set: 'one of a, b, c', c_nset: 'not a, b, c', c_range: 'range a–z',
    c_start: 'start (of line with m)', c_end: 'end (of line with m)', c_wordb: 'word boundary', c_nwordb: 'no word boundary',
    c_star: '0 or more', c_plus: '1 or more', c_opt: '0 or 1', c_exact: 'exactly 3', c_min: '2 or more', c_between: '2 to 5',
    c_lazy: 'lazy (as few as possible)',
    c_group: 'group (capturing)', c_ncgroup: 'non-capturing', c_named: 'named group', c_backref: 'backreference group 1',
    c_namedref: 'named backreference', c_or: 'a or b',
    c_ahead: 'followed by', c_nahead: 'not followed by', c_behind: 'preceded by', c_nbehind: 'not preceded by',
    c_newline: 'line break', c_tab: 'tab', c_dot: 'dot (escaped)', c_unicode: 'Unicode character', c_letter: 'letter (flag u)',
    c_whole: 'whole match', c_g1: 'group 1', c_gnamed: 'named group', c_before: 'text before', c_after: 'text after', c_dollar: 'dollar sign'
  },
  de: {
    docTitle: 'Regex Tester', back: '← Übersicht', loadExample: 'Beispiel laden …', clear: 'Leeren',
    pattern: 'Muster', patternPh: 'Regulärer Ausdruck …',
    flagG: 'global – alle Treffer', flagI: 'ignoreCase – Groß/Klein egal', flagM: 'multiline – ^ und $ pro Zeile',
    flagS: 'dotAll – . passt auch auf Zeilenumbruch', flagU: 'unicode', flagY: 'sticky – nur ab lastIndex',
    testText: 'Testtext', liveHint: 'Treffer werden live markiert', replace: 'Ersetzen',
    replacePh: 'Ersetzung, z. B. [$1] oder $<name> oder $&', preview: 'Vorschau',
    copy: 'Kopieren', copied: 'Kopiert', error: 'Fehler', errorPrefix: 'Fehler: ',
    matches: 'Treffer', thIndex: 'Index', thMatch: 'Treffer', thGroups: 'Gruppen',
    count: '{n} Treffer', noMatches: 'Keine Treffer', empty: '(leer)', more: '… und {n} weitere',
    insertTok: '„{tok}“ einfügen',
    pr_email: 'E-Mail', pr_url: 'URL', pr_date: 'Datum', pr_phone: 'Telefonnummer', pr_ipv4: 'IPv4', pr_hex: 'Hex-Farbe',
    cg_classes: 'Zeichenklassen', cg_anchors: 'Anker', cg_quant: 'Quantoren', cg_groups: 'Gruppen',
    cg_look: 'Lookaround', cg_special: 'Sonderzeichen', cg_replace: 'Ersetzen',
    c_any: 'beliebiges Zeichen', c_digit: 'Ziffer', c_nondigit: 'keine Ziffer', c_word: 'Wortzeichen', c_nonword: 'kein Wortzeichen',
    c_space: 'Leerraum', c_nonspace: 'kein Leerraum', c_set: 'eines von a, b, c', c_nset: 'nicht a, b, c', c_range: 'Bereich a–z',
    c_start: 'Anfang (Zeile mit m)', c_end: 'Ende (Zeile mit m)', c_wordb: 'Wortgrenze', c_nwordb: 'keine Wortgrenze',
    c_star: '0 oder mehr', c_plus: '1 oder mehr', c_opt: '0 oder 1', c_exact: 'genau 3', c_min: '2 oder mehr', c_between: '2 bis 5',
    c_lazy: 'lazy (so wenig wie möglich)',
    c_group: 'Gruppe (erfassend)', c_ncgroup: 'nicht erfassend', c_named: 'benannte Gruppe', c_backref: 'Rückverweis Gruppe 1',
    c_namedref: 'Rückverweis benannt', c_or: 'a oder b',
    c_ahead: 'gefolgt von', c_nahead: 'nicht gefolgt von', c_behind: 'vorausgegangen von', c_nbehind: 'nicht vorausgegangen von',
    c_newline: 'Zeilenumbruch', c_tab: 'Tabulator', c_dot: 'Punkt (escaped)', c_unicode: 'Unicode-Zeichen', c_letter: 'Buchstabe (Flag u)',
    c_whole: 'ganzer Treffer', c_g1: 'Gruppe 1', c_gnamed: 'benannte Gruppe', c_before: 'Text davor', c_after: 'Text danach', c_dollar: 'Dollarzeichen'
  }
});

const $ = id => document.getElementById(id);
const MAX_MATCHES = 10000; // safety cap for highlighting
const MAX_ROWS = 500;      // rows shown in table

// Example presets; keys are i18n ids (pr_<key>). Values may be { en, de } to pick a sample per language.
const PRESETS = {
  email: {
    pattern: String.raw`(?<user>[\w.+-]+)@(?<domain>[\w-]+(?:\.[\w-]+)*\.[a-z]{2,})`, flags: 'gi', replace: '<$<user> at $<domain>>',
    text: {
      en: 'Contact: john.smith@example.com or support+web@company-xyz.co.uk\nInvalid: foo@bar, @example.com, test@.com',
      de: 'Kontakt: max.mustermann@example.de oder support+web@firma-xyz.co.uk\nUngültig: foo@bar, @example.com, test@.de',
    },
  },
  url: {
    pattern: String.raw`https?:\/\/(?:www\.)?([\w-]+(?:\.[\w-]+)+)(\/[^\s]*)?`, flags: 'gi', replace: '[$1]',
    text: {
      en: 'See https://developer.mozilla.org/en-US/docs/Web and http://www.example.com.\nNot a link: ftp://server.local or www.no-protocol.com',
      de: 'Siehe https://developer.mozilla.org/de/docs/Web und http://www.example.com.\nKein Link: ftp://server.local oder www.ohne-protokoll.de',
    },
  },
  date: {
    pattern: {
      en: String.raw`\b(?<day>0?[1-9]|[12]\d|3[01])\.(?<month>0?[1-9]|1[0-2])\.(?<year>\d{4})\b`,
      de: String.raw`\b(?<tag>0?[1-9]|[12]\d|3[01])\.(?<monat>0?[1-9]|1[0-2])\.(?<jahr>\d{4})\b`,
    },
    flags: 'g', replace: { en: '$<year>-$<month>-$<day>', de: '$<jahr>-$<monat>-$<tag>' },
    text: {
      en: 'Dates: 24.12.2026, 1.4.2027 and 31.10.2026\nInvalid: 32.01.2026, 15.13.2026',
      de: 'Termine: 24.12.2026, 1.4.2027 und 31.10.2026\nUngültig: 32.01.2026, 15.13.2026',
    },
  },
  phone: {
    pattern: String.raw`(?:\+49|0049|0)[\s/-]?(\d{2,5})[\s/-]?(\d{3,}(?:[\s-]?\d+)*)`, flags: 'g', replace: '+49 $1 $2',
    text: {
      en: 'Office: 030 12345678\nMobile: +49 171 2345678\nFax: 0049-89-987654\nHotline 0800/1234567',
      de: 'Büro: 030 12345678\nMobil: +49 171 2345678\nFax: 0049-89-987654\nHotline 0800/1234567',
    },
  },
  ipv4: {
    pattern: String.raw`\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b`, flags: 'g', replace: '<IP>',
    text: {
      en: 'Router 192.168.0.1, DNS 8.8.8.8, localhost 127.0.0.1\nInvalid: 256.1.1.1, 10.0.0',
      de: 'Router 192.168.0.1, DNS 8.8.8.8, localhost 127.0.0.1\nUngültig: 256.1.1.1, 10.0.0',
    },
  },
  hex: {
    pattern: String.raw`#(?:[0-9a-f]{3,4}){1,2}\b`, flags: 'gi', replace: { en: 'var(--colour)', de: 'var(--farbe)' },
    text: {
      en: 'color: #58A6FF; background: #0d1117; border: 1px solid #fff;\nWith alpha: #ff000080 · Invalid: #12, #ggg',
      de: 'color: #58A6FF; background: #0d1117; border: 1px solid #fff;\nMit Alpha: #ff000080 · Ungültig: #12, #ggg',
    },
  },
};
const loc = v => (v && typeof v === 'object') ? (v[I18N.lang] ?? v.en) : v;

// Cheatsheet: [group key, [[token, description key, target], ...]]
const CHEATS = [
  ['cg_classes', [['.', 'c_any'], ['\\d', 'c_digit'], ['\\D', 'c_nondigit'], ['\\w', 'c_word'], ['\\W', 'c_nonword'], ['\\s', 'c_space'], ['\\S', 'c_nonspace'], ['[abc]', 'c_set'], ['[^abc]', 'c_nset'], ['[a-z]', 'c_range']]],
  ['cg_anchors', [['^', 'c_start'], ['$', 'c_end'], ['\\b', 'c_wordb'], ['\\B', 'c_nwordb']]],
  ['cg_quant', [['*', 'c_star'], ['+', 'c_plus'], ['?', 'c_opt'], ['{3}', 'c_exact'], ['{2,}', 'c_min'], ['{2,5}', 'c_between'], ['*?', 'c_lazy']]],
  ['cg_groups', [['(…)', 'c_group'], ['(?:…)', 'c_ncgroup'], ['(?<name>…)', 'c_named'], ['\\1', 'c_backref'], ['\\k<name>', 'c_namedref'], ['a|b', 'c_or']]],
  ['cg_look', [['(?=…)', 'c_ahead'], ['(?!…)', 'c_nahead'], ['(?<=…)', 'c_behind'], ['(?<!…)', 'c_nbehind']]],
  ['cg_special', [['\\n', 'c_newline'], ['\\t', 'c_tab'], ['\\.', 'c_dot'], ['\\u00e4', 'c_unicode'], ['\\p{L}', 'c_letter']]],
  ['cg_replace', [['$&', 'c_whole', 'r'], ['$1', 'c_g1', 'r'], ['$<name>', 'c_gnamed', 'r'], ['$`', 'c_before', 'r'], ["$'", 'c_after', 'r'], ['$$', 'c_dollar', 'r']]],
];

const patternEl = $('pattern'), textEl = $('text'), backdrop = $('backdrop'), replaceEl = $('replace');
const flags = new Set(['g']);

const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

// ---------- Build UI ----------
for (const name in PRESETS) $('presets').add(new Option(I18N.t('pr_' + name), name));
function renderPresetNames() {
  [...$('presets').options].forEach(o => { if (o.value) o.text = I18N.t('pr_' + o.value); });
}
$('presets').onchange = e => {
  const p = PRESETS[e.target.value];
  if (!p) return;
  patternEl.value = loc(p.pattern);
  textEl.value = loc(p.text);
  replaceEl.value = loc(p.replace);
  flags.clear();
  [...p.flags].forEach(f => flags.add(f));
  syncFlags();
  run();
  e.target.value = '';
};

function renderCheats() {
  // Keep collapsed/expanded state when re-rendering after a language switch
  const openState = [...$('cheats').querySelectorAll('details')].map(d => d.open);
  $('cheats').innerHTML = '';
  CHEATS.forEach(([title, toks], gi) => {
  const det = document.createElement('details');
  det.open = openState[gi] ?? true;
  det.innerHTML = `<summary>${I18N.t(title)}</summary><div class="tokens"></div>`;
  for (const [tok, desc, target] of toks) {
    const b = document.createElement('button');
    b.className = 'tok';
    b.innerHTML = `<code>${esc(tok)}</code><span>${I18N.t(desc)}</span>`;
    b.title = I18N.t('insertTok', { tok });
    // Remove the … placeholder so the cursor lands inside the group
    b.onclick = () => insertToken(target === 'r' ? replaceEl : patternEl, tok.replace('…', ''), tok.indexOf('…'));
    det.querySelector('.tokens').appendChild(b);
  }
  $('cheats').appendChild(det);
  });
}
renderCheats();

function insertToken(input, tok, caretOffset) {
  const s = input.selectionStart ?? input.value.length, e = input.selectionEnd ?? s;
  input.setRangeText(tok, s, e, 'end');
  if (caretOffset >= 0) input.setSelectionRange(s + caretOffset, s + caretOffset);
  input.focus();
  run();
}

$('flags').addEventListener('click', e => {
  const f = e.target.dataset.flag;
  if (!f) return;
  flags.has(f) ? flags.delete(f) : flags.add(f);
  syncFlags();
  run();
});
function syncFlags() {
  document.querySelectorAll('#flags button').forEach(b => b.classList.toggle('on', flags.has(b.dataset.flag)));
}

// ---------- Matching ----------
function collectMatches(re, text) {
  const out = [];
  if (!re.global) {
    re.lastIndex = 0;
    const m = re.exec(text);
    if (m) out.push(m);
    return { list: out, capped: false };
  }
  re.lastIndex = 0;
  let m;
  while ((m = re.exec(text)) !== null) {
    out.push(m);
    // Guard against zero-length matches looping forever
    if (m[0] === '') {
      const cp = re.unicode ? text.codePointAt(re.lastIndex) : 0;
      re.lastIndex += cp > 0xffff ? 2 : 1;
      if (re.lastIndex > text.length) break;
    }
    if (out.length >= MAX_MATCHES) return { list: out, capped: true };
  }
  return { list: out, capped: false };
}

function run() {
  const src = patternEl.value, text = textEl.value;
  const flagStr = [...'gimsuy'].filter(f => flags.has(f)).join('');
  let re = null;
  $('error').hidden = true;
  patternEl.classList.remove('invalid');

  if (src) {
    try { re = new RegExp(src, flagStr); }
    catch (err) {
      $('error').textContent = err.message;
      $('error').hidden = false;
      patternEl.classList.add('invalid');
    }
  }

  const { list, capped } = re ? collectMatches(re, text) : { list: [], capped: false };
  $('count').textContent = I18N.t('count', { n: `${list.length}${capped ? '+' : ''}` });

  renderHighlight(text, list);
  renderTable(list);

  // Replace preview (fresh regex so lastIndex state doesn't interfere)
  let replaced = text;
  if (re) {
    try { replaced = text.replace(new RegExp(src, flagStr), replaceEl.value); }
    catch (err) { replaced = I18N.t('errorPrefix') + err.message; }
  }
  $('replaced').textContent = replaced;
}

function renderHighlight(text, list) {
  let html = '', pos = 0;
  list.forEach((m, i) => {
    if (m.index < pos) return;
    html += esc(text.slice(pos, m.index));
    if (m[0] === '') html += '<mark class="zero">⁠</mark>'; // zero-width marker
    else html += `<mark${i % 2 ? ' class="alt"' : ''}>${esc(m[0])}</mark>`;
    pos = m.index + m[0].length;
  });
  html += esc(text.slice(pos));
  // Trailing space keeps the last empty line in sync with the textarea
  backdrop.innerHTML = html + ' ';
  backdrop.scrollTop = textEl.scrollTop;
}

function renderTable(list) {
  const tbody = $('matches').tBodies[0];
  if (!list.length) {
    tbody.innerHTML = `<tr><td colspan="4" class="empty">${I18N.t('noMatches')}</td></tr>`;
    return;
  }
  const rows = list.slice(0, MAX_ROWS).map((m, i) => {
    const show = v => v === undefined ? '<i>undefined</i>' : `"${esc(v)}"`;
    let groups = m.slice(1).map((g, gi) => `<span class="grp"><b>$${gi + 1}</b> ${show(g)}</span>`).join('');
    if (m.groups) groups += Object.entries(m.groups).map(([k, v]) => `<span class="grp"><b>&lt;${esc(k)}&gt;</b> ${show(v)}</span>`).join('');
    return `<tr><td>${i + 1}</td><td>${m.index}</td><td><code>${m[0] === '' ? `<i>${I18N.t('empty')}</i>` : esc(m[0])}</code></td><td>${groups || '<span class="grp">–</span>'}</td></tr>`;
  });
  if (list.length > MAX_ROWS) rows.push(`<tr><td colspan="4" class="empty">${I18N.t('more', { n: list.length - MAX_ROWS })}</td></tr>`);
  tbody.innerHTML = rows.join('');
}

// ---------- Events ----------
patternEl.addEventListener('input', run);
textEl.addEventListener('input', run);
replaceEl.addEventListener('input', run);
textEl.addEventListener('scroll', () => { backdrop.scrollTop = textEl.scrollTop; backdrop.scrollLeft = textEl.scrollLeft; });

// Keep the editor container height in sync when the user resizes it
new ResizeObserver(() => { backdrop.scrollTop = textEl.scrollTop; }).observe($('editor'));

$('copy').onclick = async e => {
  try {
    await navigator.clipboard.writeText($('replaced').textContent);
    e.target.textContent = I18N.t('copied');
  } catch {
    e.target.textContent = I18N.t('error');
  }
  setTimeout(() => e.target.textContent = I18N.t('copy'), 1500);
};

$('resetAll').onclick = () => {
  patternEl.value = textEl.value = replaceEl.value = '';
  flags.clear(); flags.add('g'); syncFlags();
  run();
};

// Live language switch: re-render generated texts only; pattern, test text and replacement stay untouched.
addEventListener('langchange', () => { renderPresetNames(); renderCheats(); run(); });

// Start with the first example (sample text in the current language)
$('presets').value = 'email';
$('presets').dispatchEvent(new Event('change'));

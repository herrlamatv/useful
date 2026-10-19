/* ai chat exporter - by herrlamatv
 * Kleiner Markdown -> HTML-Renderer. Gebraucht, wenn nur Markdown da ist:
 * API-Daten ohne passendes DOM, Thinking-Blöcke, Bulk-Export.
 *
 * Kann: Überschriften, Absätze, Listen (verschachtelt, Aufgaben), Zitate,
 * Code-Blöcke, GFM-Tabellen, Trennlinien, Links, Bilder, fett/kursiv/
 * durchgestrichen, Inline-Code, Mathe ($…$, $$…$$, \(…\), \[…\]).
 * Rohes HTML im Markdown wird immer escaped - nichts davon wird ausgeführt.
 */

export function escapeHtml(text) {
  return String(text == null ? '' : text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** Nur harmlose Ziele - javascript: & Co. werden zu '#'. */
export function safeUrl(url, { image = false } = {}) {
  const u = String(url || '').trim();
  if (/^(https?:|mailto:|#|\/(?!\/))/i.test(u)) return u;
  if (image && /^data:image\/(png|jpe?g|gif|webp|svg\+xml);/i.test(u)) return u;
  if (image && /^blob:/i.test(u)) return u;
  return '#';
}

/* ------------------------------- Inline -------------------------------- */

export function inline(text, { breaks = false } = {}) {
  const store = [];
  const keep = (html) => '\u0000' + (store.push(html) - 1) + '\u0000';
  let s = String(text);

  /* Code-Spans zuerst - darin gilt nichts anderes. */
  s = s.replace(/(`+)([\s\S]*?[^`])\1(?!`)/g, (all, ticks, code) => keep('<code>' + escapeHtml(code.replace(/^ (.*) $/, '$1')) + '</code>'));

  /* Mathe: \(…\), \[…\], $$…$$ und $…$ (kein Leerzeichen innen am Rand, keine Ziffer danach - sonst Geldbeträge). */
  s = s.replace(/\\\[([\s\S]+?)\\\]/g, (all, tex) => keep('<span class="math display">' + escapeHtml(tex.trim()) + '</span>'));
  s = s.replace(/\\\(([\s\S]+?)\\\)/g, (all, tex) => keep('<span class="math">' + escapeHtml(tex.trim()) + '</span>'));
  s = s.replace(/\$\$([\s\S]+?)\$\$/g, (all, tex) => keep('<span class="math display">' + escapeHtml(tex.trim()) + '</span>'));
  s = s.replace(/(^|[^\\$\w])\$(?!\s)([^$\n]+?)(?<!\s)\$(?![\d$])/g, (all, pre, tex) => pre + keep('<span class="math">' + escapeHtml(tex) + '</span>'));

  /* Backslash-Escapes. */
  s = s.replace(/\\([\\`*_{}\[\]()#+\-.!|~<>$])/g, (all, ch) => keep(escapeHtml(ch)));

  /* Autolinks <https://…> */
  s = s.replace(/<((?:https?:\/\/|mailto:)[^\s<>]+)>/gi, (all, url) => keep('<a href="' + escapeHtml(safeUrl(url)) + '">' + escapeHtml(url) + '</a>'));

  s = escapeHtml(s);

  /* Bilder und Links - Klammern im Ziel dürfen einmal verschachtelt sein (Wikipedia-Links). */
  const target = '((?:[^()\\s]|\\([^()\\s]*\\))+)(?:\\s+&quot;([^&]*)&quot;)?';
  s = s.replace(new RegExp('!\\[([^\\]]*)\\]\\(\\s*' + target + '\\s*\\)', 'g'), (all, alt, url, title) =>
    keep(
      '<img src="' + escapeHtml(safeUrl(unescape(url), { image: true })) + '" alt="' + alt + '"' + (title ? ' title="' + title + '"' : '') + ' />'
    )
  );
  s = s.replace(new RegExp('\\[((?:[^\\[\\]]|\\[[^\\]]*\\])*)\\]\\(\\s*' + target + '\\s*\\)', 'g'), (all, label, url, title) =>
    keep('<a href="' + escapeHtml(safeUrl(unescape(url))) + '"' + (title ? ' title="' + title + '"' : '') + '>') + label + keep('</a>')
  );

  /* Nackte URLs. */
  s = s.replace(/(^|[\s(])(https?:\/\/[^\s<]+[^\s<.,;:!?)\]'"])/g, (all, pre, url) => pre + keep('<a href="' + url + '">' + url + '</a>'));

  /* Betonung. */
  s = s.replace(/\*\*\*(?=\S)([\s\S]*?\S)\*\*\*/g, '<strong><em>$1</em></strong>');
  s = s.replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, '<strong>$1</strong>');
  s = s.replace(/(^|[^\w])__(?=\S)([\s\S]*?\S)__(?!\w)/g, '$1<strong>$2</strong>');
  s = s.replace(/(^|[^*\w])\*(?=[^\s*])([\s\S]*?[^\s*])\*(?!\*)/g, '$1<em>$2</em>');
  s = s.replace(/(^|[^\w])_(?=[^\s_])([\s\S]*?[^\s_])_(?!\w)/g, '$1<em>$2</em>');
  s = s.replace(/~~(?=\S)([\s\S]*?\S)~~/g, '<del>$1</del>');

  /* Harte Zeilenumbrüche: zwei Leerzeichen oder Backslash am Zeilenende. */
  s = s.replace(/( {2,}|\\)\n/g, '<br />\n');
  if (breaks) s = s.replace(/(?<!<br \/>)\n/g, '<br />\n');

  return s.replace(/\u0000(\d+)\u0000/g, (all, i) => store[Number(i)]).replace(/\u0000(\d+)\u0000/g, (all, i) => store[Number(i)]);
}

/* escapeHtml wurde vor dem Link-Ersetzen angewendet - für die URL zurückdrehen. */
function unescape(text) {
  return text.replace(/&quot;/g, '"').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');
}

/* ------------------------------- Blöcke -------------------------------- */

const FENCE = /^( {0,3})(`{3,}|~{3,})\s*([^\s`{]*)?[^`]*$/;
const HEADING = /^ {0,3}(#{1,6})(?:\s+(.*?))?\s*#*\s*$/;
const HR = /^ {0,3}([-*_])(?:\s*\1){2,}\s*$/;
const QUOTE = /^ {0,3}> ?/;
const LIST = /^( *)([-*+]|\d{1,9}[.)])(\s+|$)(.*)$/;
const TABLE_SEP = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;
const MATH_OPEN = /^\s*(\$\$|\\\[)/;

const indentOf = (line) => line.match(/^ */)[0].length;
const isBlank = (line) => /^\s*$/.test(line);

function startsBlock(line) {
  return FENCE.test(line) || HEADING.test(line) || HR.test(line) || QUOTE.test(line) || LIST.test(line) || MATH_OPEN.test(line);
}

function splitRow(line) {
  let s = line.trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|') && !s.endsWith('\\|')) s = s.slice(0, -1);
  const cells = [];
  let cur = '';
  let inCode = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === '\\' && s[i + 1] === '|') {
      cur += '|';
      i++;
    } else if (ch === '`') {
      inCode = !inCode;
      cur += ch;
    } else if (ch === '|' && !inCode) {
      cells.push(cur.trim());
      cur = '';
    } else cur += ch;
  }
  cells.push(cur.trim());
  return cells;
}

function renderTable(lines) {
  const head = splitRow(lines[0]);
  const aligns = splitRow(lines[1]).map((c) => (/^:-+:$/.test(c) ? 'center' : /-:$/.test(c) ? 'right' : /^:-/.test(c) ? 'left' : ''));
  const cell = (tag, text, i) => '<' + tag + (aligns[i] ? ' style="text-align:' + aligns[i] + '"' : '') + '>' + inline(text) + '</' + tag + '>';
  let html = '<table>\n<thead><tr>' + head.map((c, i) => cell('th', c, i)).join('') + '</tr></thead>\n<tbody>\n';
  for (const line of lines.slice(2)) {
    const cells = splitRow(line);
    html += '<tr>' + head.map((h, i) => cell('td', cells[i] || '', i)).join('') + '</tr>\n';
  }
  return html + '</tbody></table>';
}

function renderList(lines, start) {
  const first = LIST.exec(lines[start]);
  const baseIndent = first[1].length;
  const ordered = /\d/.test(first[2]);
  const items = [];
  let i = start;
  let loose = false;
  let sawBlank = false;

  while (i < lines.length) {
    const m = LIST.exec(lines[i]);
    if (!m || m[1].length !== baseIndent || /\d/.test(m[2]) !== ordered) break;
    if (sawBlank) loose = true;
    sawBlank = false;
    const contentIndent = baseIndent + m[2].length + Math.max(1, Math.min(m[3].length, 4));
    const body = [m[4]];
    i++;
    while (i < lines.length) {
      const line = lines[i];
      if (isBlank(line)) {
        /* Leerzeile: gehört dazu, wenn danach eingerückt weitergeht. */
        let j = i;
        while (j < lines.length && isBlank(lines[j])) j++;
        if (j < lines.length && indentOf(lines[j]) >= contentIndent) {
          body.push('');
          i++;
          continue;
        }
        sawBlank = true;
        i = j;
        break;
      }
      if (indentOf(line) >= contentIndent) {
        body.push(line.slice(contentIndent));
        i++;
        continue;
      }
      const sub = LIST.exec(line);
      if (sub && sub[1].length > baseIndent) {
        body.push(line.slice(Math.min(indentOf(line), contentIndent)));
        i++;
        continue;
      }
      if (sub || startsBlock(line)) break;
      /* „Faule“ Fortsetzung eines Absatzes. */
      body.push(line.trim());
      i++;
    }
    if (body.slice(1).some((l) => l === '')) loose = true;
    items.push({ body, start: ordered ? parseInt(m[2], 10) : null });
  }

  const tag = ordered ? 'ol' : 'ul';
  const startAttr = ordered && items[0].start !== 1 ? ' start="' + items[0].start + '"' : '';
  let html = '<' + tag + startAttr + '>\n';
  for (const item of items) {
    let body = item.body;
    let task = '';
    const tm = /^\[([ xX])\]\s+/.exec(body[0]);
    if (tm) {
      task = '<input type="checkbox" disabled' + (tm[1] !== ' ' ? ' checked' : '') + ' /> ';
      body = [body[0].slice(tm[0].length)].concat(body.slice(1));
    }
    let inner = blocks(body);
    if (!loose) inner = inner.replace(/^<p>([\s\S]*?)<\/p>/, '$1').replace(/<\/p>\n<(ul|ol)/g, '</p>\n<$1');
    html += '<li' + (task ? ' class="task"' : '') + '>' + task + inner + '</li>\n';
  }
  return { html: html + '</' + tag + '>', next: i };
}

function blocks(lines, opts = {}) {
  const out = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (isBlank(line)) {
      i++;
      continue;
    }

    const fence = FENCE.exec(line);
    if (fence) {
      const indent = fence[1].length;
      const mark = fence[2];
      const lang = (fence[3] || '').replace(/[^\w+#.-]/g, '');
      const code = [];
      i++;
      while (i < lines.length) {
        const close = new RegExp('^ {0,3}' + mark[0] + '{' + mark.length + ',}\\s*$');
        if (close.test(lines[i])) {
          i++;
          break;
        }
        code.push(lines[i].replace(new RegExp('^ {0,' + indent + '}'), ''));
        i++;
      }
      out.push(
        '<pre><code' + (lang ? ' class="language-' + escapeHtml(lang) + '"' : '') + '>' + escapeHtml(code.join('\n')) + '</code></pre>'
      );
      continue;
    }

    if (MATH_OPEN.test(line)) {
      const open = line.trim().startsWith('$$') ? '$$' : '\\[';
      const close = open === '$$' ? '$$' : '\\]';
      let rest = line.trim().slice(2);
      const tex = [];
      if (rest.includes(close)) {
        tex.push(rest.slice(0, rest.indexOf(close)));
        i++;
      } else {
        if (rest) tex.push(rest);
        i++;
        while (i < lines.length && !lines[i].includes(close)) tex.push(lines[i++]);
        if (i < lines.length) {
          const last = lines[i];
          tex.push(last.slice(0, last.indexOf(close)));
          i++;
        }
      }
      out.push('<div class="math display">' + escapeHtml(tex.join('\n').trim()) + '</div>');
      continue;
    }

    const heading = HEADING.exec(line);
    if (heading) {
      const level = heading[1].length;
      out.push('<h' + level + '>' + inline(heading[2] || '') + '</h' + level + '>');
      i++;
      continue;
    }

    if (HR.test(line)) {
      out.push('<hr />');
      i++;
      continue;
    }

    if (QUOTE.test(line)) {
      const inner = [];
      while (i < lines.length && !isBlank(lines[i])) {
        if (QUOTE.test(lines[i])) inner.push(lines[i].replace(QUOTE, ''));
        else if (startsBlock(lines[i])) break;
        else inner.push(lines[i]);
        i++;
      }
      out.push('<blockquote>\n' + blocks(inner) + '\n</blockquote>');
      continue;
    }

    if (LIST.test(line)) {
      const list = renderList(lines, i);
      out.push(list.html);
      i = list.next;
      continue;
    }

    if (line.includes('|') && i + 1 < lines.length && TABLE_SEP.test(lines[i + 1]) && lines[i + 1].includes('-')) {
      const rows = [line, lines[i + 1]];
      i += 2;
      while (i < lines.length && !isBlank(lines[i]) && lines[i].includes('|')) rows.push(lines[i++]);
      out.push(renderTable(rows));
      continue;
    }

    /* Absatz. */
    const para = [line];
    i++;
    while (i < lines.length && !isBlank(lines[i]) && !startsBlock(lines[i])) {
      if (lines[i].includes('|') && i + 1 < lines.length && TABLE_SEP.test(lines[i + 1])) break;
      para.push(lines[i]);
      i++;
    }
    out.push('<p>' + inline(para.map((l) => l.replace(/^\s+/, '')).join('\n'), opts) + '</p>');
  }
  return out.join('\n');
}

/** breaks: einfache Zeilenumbrüche als <br> (so zeigen die Chats deine Prompts). */
export function md2html(markdown, opts = {}) {
  const lines = String(markdown == null ? '' : markdown)
    .replace(/\r\n?/g, '\n')
    .replace(/\t/g, '    ')
    .split('\n');
  return blocks(lines, opts);
}

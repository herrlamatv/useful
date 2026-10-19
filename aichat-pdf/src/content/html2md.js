/* ai chat exporter - by herrlamatv
 * DOM -> Markdown für die bereinigte Kopie eines Beitrags (AIX.cleanClone).
 * GFM: Tabellen, Code-Fences mit Sprache, Durchstreichen. Mathe aus KaTeX
 * (annotation application/x-tex) wird zu $…$ bzw. $$…$$.
 */

(() => {
  const AIX = (globalThis.AIX = globalThis.AIX || {});
  if (AIX.html2md) return;

  const PARA = new Set(['P', 'DIV', 'SECTION', 'ARTICLE', 'HEADER', 'FOOTER', 'MAIN', 'ASIDE', 'NAV', 'FIGURE', 'DETAILS', 'DL', 'CENTER', 'FORM', 'FIELDSET', 'ADDRESS']);

  function escapeText(s) {
    return s
      .replace(/\\/g, '\\\\')
      .replace(/\*/g, '\\*')
      .replace(/`/g, '\\`')
      .replace(/(^|[\s([{])_/g, '$1\\_')
      .replace(/_(?=[\s)\]}.,;:!?]|$)/g, '\\_')
      .replace(/<(?=[a-zA-Z/!])/g, '\\<');
  }

  const keepsNewlines = (node) => {
    for (let el = node.parentElement; el; el = el.parentElement) {
      if (el.hasAttribute && el.hasAttribute('data-aix-ws')) return true;
      if (PARA.has(el.tagName) && el.tagName !== 'DIV') return false;
    }
    return false;
  };

  function textOf(node) {
    const value = (node.nodeValue || '').replace(/[\ue000-\uf8ff]/g, '');
    if (keepsNewlines(node)) {
      return value
        .split('\n')
        .map((line) => escapeText(line.replace(/[ \t\r]+/g, ' ')))
        .join('\n');
    }
    return escapeText(value.replace(/[\t\n\r ]+/g, ' '));
  }

  const children = (node, ctx) => Array.from(node.childNodes).map((c) => convert(c, ctx)).join('');

  function wrap(node, ctx, mark) {
    const inner = children(node, ctx);
    if (!inner.trim()) return inner;
    if (/\n\n/.test(inner.trim())) return inner; // Block-Inhalt nicht in ** packen
    const lead = inner.match(/^\s*/)[0];
    const trail = inner.match(/\s*$/)[0];
    return lead + mark + inner.trim() + mark + trail;
  }

  function fence(pre) {
    const code = pre.querySelector('code') || pre;
    const text = (code.textContent || '').replace(/\n$/, '');
    const m = /language-([\w+#.-]+)/.exec(code.className || '');
    const lang = pre.getAttribute('data-lang') || (m ? m[1] : '');
    const runs = text.match(/`{3,}/g);
    const ticks = '`'.repeat(runs ? Math.max(...runs.map((r) => r.length)) + 1 : 3);
    return ticks + lang + '\n' + text + '\n' + ticks;
  }

  function inlineCode(node) {
    const text = (node.textContent || '').replace(/\n/g, ' ');
    if (!text) return '';
    const runs = text.match(/`+/g);
    const ticks = '`'.repeat(runs ? Math.max(...runs.map((r) => r.length)) + 1 : 1);
    const pad = /^`|`$/.test(text) ? ' ' : '';
    return ticks + pad + text + pad + ticks;
  }

  function mathOf(node) {
    let tex = '';
    const ann = node.querySelector && node.querySelector('annotation[encoding="application/x-tex"], annotation');
    if (node.getAttribute('data-tex')) tex = node.getAttribute('data-tex');
    else if (node.getAttribute('data-math')) tex = node.getAttribute('data-math');
    else if (ann) tex = ann.textContent;
    else if (node.tagName.toLowerCase() === 'math') tex = node.getAttribute('alttext') || node.textContent;
    tex = String(tex || '').trim();
    if (!tex) return null;
    const display =
      !!(node.closest && node.closest('.katex-display, .math-display, [data-display="block"]')) ||
      node.getAttribute('display') === 'block' ||
      (node.classList && node.classList.contains('math-block'));
    return display ? '\n\n$$\n' + tex + '\n$$\n\n' : '$' + tex + '$';
  }

  function isMath(node) {
    if (node.tagName.toLowerCase() === 'math') return true;
    if (node.hasAttribute('data-tex') || node.hasAttribute('data-math')) return true;
    return !!(node.classList && node.classList.contains('katex'));
  }

  function link(node, ctx) {
    const text = children(node, ctx).trim();
    const href = node.href || node.getAttribute('href') || '';
    if (!href || /^javascript:/i.test(href) || href === '#') return text;
    if (!text) return '';
    if (text === href || text === href.replace(/^https?:\/\//, '')) return '<' + href + '>';
    return '[' + text.replace(/\n+/g, ' ') + '](' + href.replace(/ /g, '%20').replace(/\(/g, '%28').replace(/\)/g, '%29') + ')';
  }

  function image(node) {
    const src = node.getAttribute('src') || node.src || '';
    if (!src || /^javascript:/i.test(src)) return '';
    const alt = (node.getAttribute('alt') || '').replace(/[[\]\n]/g, ' ').trim();
    return '![' + alt + '](' + src.replace(/ /g, '%20').replace(/\)/g, '%29') + ')';
  }

  function list(node, ctx) {
    const ordered = node.tagName === 'OL';
    let n = parseInt(node.getAttribute('start') || '1', 10) || 1;
    const items = [];
    for (const li of Array.from(node.children)) {
      if (li.tagName !== 'LI') {
        const other = convert(li, ctx).trim();
        if (other) items.push(other);
        continue;
      }
      const marker = ordered ? n++ + '. ' : '- ';
      let content = children(li, ctx)
        .replace(/^\s+|\s+$/g, '')
        /* Absatz direkt vor Unterliste: ohne Leerzeile, sonst wird die Liste „locker“. */
        .replace(/\n{2,}(?=(?:[-*+]|\d+\.) )/g, '\n');
      const pad = ' '.repeat(marker.length);
      content = content
        .split('\n')
        .map((line, i) => (i === 0 || !line ? line : pad + line))
        .join('\n');
      items.push(marker + content);
    }
    const loose = items.some((item) => /\n\n/.test(item));
    return items.join(loose ? '\n\n' : '\n');
  }

  function table(node, ctx) {
    const rows = Array.from(node.querySelectorAll('tr')).filter((tr) => tr.closest('table') === node);
    const matrix = rows
      .map((tr) =>
        Array.from(tr.children)
          .filter((c) => c.tagName === 'TH' || c.tagName === 'TD')
          .map((c) =>
            children(c, ctx)
              .replace(/\s*\n+\s*/g, ' ')
              .trim()
              .replace(/\|/g, '\\|')
          )
      )
      .filter((r) => r.length);
    if (!matrix.length) return '';
    const width = Math.max(...matrix.map((r) => r.length));
    const line = (cells) => '| ' + Array.from({ length: width }, (_, i) => cells[i] || '').join(' | ') + ' |';
    return [line(matrix[0]), '| ' + Array(width).fill('---').join(' | ') + ' |'].concat(matrix.slice(1).map(line)).join('\n');
  }

  function block(text) {
    const t = text.trim();
    return t ? '\n\n' + t + '\n\n' : '';
  }

  function convert(node, ctx) {
    if (node.nodeType === 3) return ctx.pre ? node.nodeValue : textOf(node);
    if (node.nodeType !== 1) return '';
    const tag = node.tagName.toUpperCase();

    if (isMath(node)) {
      const m = mathOf(node);
      if (m !== null) return m;
    }

    switch (tag) {
      case 'BR':
        return '\n';
      case 'HR':
        return '\n\n---\n\n';
      case 'H1':
      case 'H2':
      case 'H3':
      case 'H4':
      case 'H5':
      case 'H6': {
        const text = children(node, ctx).replace(/\s*\n+\s*/g, ' ').trim();
        return text ? '\n\n' + '#'.repeat(Number(tag[1])) + ' ' + text + '\n\n' : '';
      }
      case 'PRE':
        return '\n\n' + fence(node) + '\n\n';
      case 'CODE':
        return inlineCode(node);
      case 'BLOCKQUOTE': {
        const inner = children(node, ctx).trim();
        return inner ? '\n\n' + inner.split('\n').map((l) => (l ? '> ' + l : '>')).join('\n') + '\n\n' : '';
      }
      case 'UL':
      case 'OL':
        return block(list(node, ctx));
      case 'LI':
        return block('- ' + children(node, ctx).trim());
      case 'TABLE':
        return block(table(node, ctx));
      case 'A':
        return link(node, ctx);
      case 'IMG':
        return image(node);
      case 'STRONG':
      case 'B':
        return wrap(node, ctx, '**');
      case 'EM':
      case 'I':
      case 'CITE':
        return wrap(node, ctx, '*');
      case 'DEL':
      case 'S':
      case 'STRIKE':
        return wrap(node, ctx, '~~');
      case 'SUMMARY':
      case 'DT': {
        const text = children(node, ctx).trim();
        return text ? '\n\n**' + text + '**\n\n' : '';
      }
      case 'FIGCAPTION': {
        const text = children(node, ctx).trim();
        return text ? '\n\n*' + text + '*\n\n' : '';
      }
      case 'P': {
        let inner = children(node, ctx).trim();
        /* Text, der wie Markdown-Syntax anfängt, entschärfen. */
        if (/^(#{1,6}\s|>|[-+]\s|\d+[.)]\s)/.test(inner)) inner = '\\' + inner;
        return block(inner);
      }
      default:
        if (PARA.has(tag) || tag === 'DD') return block(children(node, ctx));
        return children(node, ctx);
    }
  }

  /** Leerzeilen zusammenfassen und Zeilenenden säubern - aber nie in Code-Blöcken. */
  function tidy(md) {
    const out = [];
    let fenceMark = null;
    let blank = 0;
    for (const raw of md.split('\n')) {
      const f = /^\s*(`{3,}|~{3,})/.exec(raw);
      if (fenceMark) {
        out.push(raw);
        if (f && f[1][0] === fenceMark[0] && f[1].length >= fenceMark.length && /^\s*[`~]+\s*$/.test(raw)) fenceMark = null;
        continue;
      }
      if (f) fenceMark = f[1];
      const line = raw.replace(/[ \t]+$/, '');
      if (!line.trim()) {
        if (blank++ === 0 && out.length) out.push('');
        continue;
      }
      blank = 0;
      out.push(line);
    }
    return out.join('\n').trim();
  }

  AIX.html2md = (root) => tidy(children(root, {}));
})();

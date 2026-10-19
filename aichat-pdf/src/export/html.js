/* ai chat exporter - by herrlamatv
 * HTML-Dokument für den HTML-Export und die PDF-Seite (ui/render.html).
 * Eine Quelle für beides - was im HTML gut aussieht, sieht im PDF genauso aus.
 *
 * mode: 'styled' = formatiert (Code, Tabellen, Mathe, Bilder)
 *       'raw'    = nur Text, Schreibmaschinen-Optik
 */

import { escapeHtml, md2html, safeUrl } from './md2html.js';
import { mdToText } from './text.js';
import { isoDate } from './filename.js';

const SIZES = { s: ['14px', '9.5pt'], m: ['15px', '10.5pt'], l: ['17px', '12pt'] };
const PAPER = { a4: 'A4', letter: 'letter' };

/* Farben passend zum Cream-Look der Erweiterung, auf Papier etwas heller. */
const STYLED = `
:root{--ink:#1f1d18;--ink-2:#7c7466;--ink-3:#a29a8a;--line:#e7dfd0;--soft:#faf6ec;--paper:#fffdf8;--caramel:#b98b52;--link:#0a5bb5;--code:#f7f3ea;color-scheme:light}
html{-webkit-print-color-adjust:exact;print-color-adjust:exact}
body{margin:0;background:#f3efe5;color:var(--ink);font:var(--size)/1.62 -apple-system,"Segoe UI Variable Text","Segoe UI",system-ui,sans-serif;-webkit-font-smoothing:antialiased}
.doc{max-width:860px;margin:0 auto;padding:40px 28px 64px}
.doc-head{margin-bottom:26px;padding-bottom:16px;border-bottom:2px solid var(--line)}
.kicker{color:var(--caramel);font-weight:700;font-size:.74em;letter-spacing:.07em;text-transform:uppercase}
.doc-head h1{font-size:1.75em;line-height:1.25;margin:4px 0 6px;letter-spacing:-.01em}
.doc-head .meta{color:var(--ink-2);font-size:.84em;overflow-wrap:anywhere}
.doc-head .meta a{color:var(--ink-2)}
.msg{margin:0 0 24px}
.who{font-size:.72em;font-weight:700;letter-spacing:.07em;text-transform:uppercase;color:var(--caramel);margin-bottom:6px;break-after:avoid}
.who .time{font-weight:500;letter-spacing:0;text-transform:none;color:var(--ink-3);margin-left:8px}
.msg.user .body{background:var(--paper);border:1px solid var(--line);border-radius:14px;padding:10px 16px}
.attach{font-size:.84em;color:var(--ink-2);margin-bottom:6px}
.body>:first-child{margin-top:0}.body>:last-child{margin-bottom:0}
.body p{margin:.6em 0}
.body h1,.body h2,.body h3,.body h4,.body h5,.body h6{line-height:1.3;margin:1.1em 0 .45em;break-after:avoid}
.body h1{font-size:1.4em}.body h2{font-size:1.24em}.body h3{font-size:1.1em}.body h4,.body h5,.body h6{font-size:1em}
.body ul,.body ol{padding-left:1.5em;margin:.5em 0}
.body li{margin:.2em 0}.body li>p{margin:.25em 0}
.body li.task{list-style:none;margin-left:-1.3em}
.body blockquote{margin:.7em 0;padding:.1em 0 .1em 1em;border-left:3px solid var(--line);color:var(--ink-2)}
.body a{color:var(--link);text-decoration:none;overflow-wrap:anywhere}
.body code{font-family:ui-monospace,"SF Mono","Cascadia Code",Consolas,monospace;font-size:.87em;background:var(--code);border-radius:5px;padding:.1em .35em}
.body pre{background:var(--code);border:1px solid var(--line);border-radius:10px;padding:12px 14px;margin:.8em 0;white-space:pre-wrap;overflow-wrap:anywhere;font-size:.85em;line-height:1.5}
.body pre code{background:none;padding:0;font-size:inherit;border-radius:0;white-space:inherit}
.body pre[data-lang]::before{content:attr(data-lang);display:block;font:700 .78em system-ui,sans-serif;color:var(--ink-3);text-transform:uppercase;letter-spacing:.06em;margin-bottom:6px}
.body table{border-collapse:collapse;margin:.8em 0;font-size:.92em;max-width:100%}
.body th,.body td{border:1px solid var(--line);padding:6px 10px;text-align:left;vertical-align:top}
.body th{background:var(--soft);font-weight:600}
.body img{max-width:100%;height:auto;border-radius:10px;break-inside:avoid}
.body hr{border:none;border-top:1px solid var(--line);margin:1.2em 0}
.body details{margin:.6em 0}
math{font-family:"Cambria Math","STIX Two Math","Latin Modern Math",math,serif;font-size:1.08em}
math[display=block]{display:block;margin:.7em 0;overflow-x:auto}
.math{font-family:ui-monospace,Consolas,monospace;font-size:.88em;background:var(--code);border-radius:5px;padding:.1em .35em}
.math.display{display:block;text-align:center;padding:.5em;margin:.6em 0;white-space:pre-wrap}
.extra{margin:0 0 10px;border-left:3px solid var(--line);padding:2px 0 2px 12px;color:var(--ink-2);font-size:.9em}
.extra summary{cursor:pointer;font-weight:700;color:var(--ink-3);font-size:.82em;letter-spacing:.04em;text-transform:uppercase}
.extra .body{margin-top:6px}
.sources{margin-top:10px;font-size:.84em;color:var(--ink-2)}
.sources b{font-weight:700;color:var(--ink-3);font-size:.9em;letter-spacing:.04em;text-transform:uppercase}
.sources ol{margin:4px 0 0;padding-left:1.4em}
.sources a{color:var(--link);text-decoration:none;overflow-wrap:anywhere}
.doc-foot{margin-top:32px;padding-top:12px;border-top:1px solid var(--line);color:var(--ink-3);font-size:.76em}
.hljs-comment,.hljs-quote,.token.comment,.token.prolog{color:#a0a1a7;font-style:italic}
.hljs-keyword,.hljs-selector-tag,.hljs-doctag,.token.keyword,.token.atrule{color:#a626a4}
.hljs-string,.hljs-regexp,.hljs-addition,.hljs-meta .hljs-string,.token.string,.token.char,.token.regex{color:#50a14f}
.hljs-number,.hljs-literal,.hljs-symbol,.hljs-bullet,.token.number,.token.boolean,.token.constant{color:#986801}
.hljs-title,.hljs-section,.hljs-title.function_,.token.function,.token.class-name{color:#4078f2}
.hljs-built_in,.hljs-type,.hljs-title.class_,.token.builtin{color:#c18401}
.hljs-attr,.hljs-attribute,.hljs-variable,.hljs-template-variable,.hljs-property,.token.property,.token.attr-name,.token.variable{color:#986801}
.hljs-tag,.hljs-name,.hljs-selector-id,.hljs-selector-class,.token.tag,.token.selector{color:#e45649}
.hljs-meta,.token.operator,.token.punctuation{color:#7c7466}
.hljs-deletion,.token.deleted{color:#e45649}
@media print{
  body{background:#fff}
  .doc{max-width:none;padding:0}
  .msg.user .body{background:var(--soft)}
  .body pre,.body table,.body blockquote{break-inside:auto}
  a{color:var(--link)}
}`;

const RAW = `
html{-webkit-print-color-adjust:exact;print-color-adjust:exact}
body{margin:0;background:#fff;color:#111;font:var(--size)/1.5 ui-monospace,"Cascadia Mono",Consolas,"Courier New",monospace}
.doc{max-width:860px;margin:0 auto;padding:32px 24px}
.raw-head{border-bottom:1px solid #888;padding-bottom:8px;margin-bottom:18px}
.raw-head h1{font-size:1.2em;margin:0 0 4px}
.raw-head div{color:#555;overflow-wrap:anywhere}
.msg{margin:0 0 14px}
.who{font-weight:700;break-after:avoid}
.text{white-space:pre-wrap;overflow-wrap:anywhere}
.sep{border:none;border-top:1px dashed #bbb;margin:14px 0}
@media print{.doc{max-width:none;padding:0}}`;

export function documentCss({ mode = 'styled', fontSize = 'm', paper = 'a4', print = false } = {}) {
  const [screen, pt] = SIZES[fontSize] || SIZES.m;
  const margins = mode === 'raw' ? '18mm 16mm' : '16mm 15mm 18mm';
  return (
    ':root{--size:' + screen + '}@media print{:root{--size:' + pt + '}}' +
    '@page{size:' + (PAPER[paper] || 'A4') + ';margin:' + margins + '}' +
    (print ? '@page{@bottom-center{content:counter(page) " / " counter(pages);font:8pt system-ui,sans-serif;color:#a29a8a}}' : '') +
    (mode === 'raw' ? RAW : STYLED)
  );
}

function timeText(value) {
  if (!value) return '';
  const d = new Date(value);
  return isNaN(d) ? '' : d.toLocaleString();
}

const whoOf = (msg, conv, labels) => (msg.role === 'user' ? labels.you : msg.name || conv.platformName || 'AI');

function headerHtml(conv, labels) {
  const kicker = [conv.platformName, conv.model].filter(Boolean).join(' · ');
  const date = conv.createdAt ? isoDate(conv.createdAt) : '';
  const url = conv.url ? '<a href="' + escapeHtml(safeUrl(conv.url)) + '">' + escapeHtml(conv.url) + '</a>' : '';
  return (
    '<header class="doc-head">' +
    (kicker ? '<div class="kicker">' + escapeHtml(kicker) + '</div>' : '') +
    '<h1>' + escapeHtml(conv.title || 'Chat') + '</h1>' +
    '<div class="meta">' + [date ? escapeHtml(date) : '', url].filter(Boolean).join(' · ') + '</div>' +
    '</header>'
  );
}

function extraHtml(title, html, open) {
  return '<details class="extra"' + (open ? ' open' : '') + '><summary>' + escapeHtml(title) + '</summary><div class="body">' + html + '</div></details>';
}

function styledMessage(msg, conv, { labels, include = {}, print }) {
  const time = include.times && msg.time ? '<span class="time">' + escapeHtml(timeText(msg.time)) + '</span>' : '';
  let html = '<section class="msg ' + (msg.role === 'user' ? 'user' : 'assistant') + '">';
  html += '<div class="who">' + escapeHtml(whoOf(msg, conv, labels)) + time + '</div>';
  if (msg.attachments && msg.attachments.length) {
    html += '<div class="attach">📎 ' + msg.attachments.map((a) => escapeHtml(a.name)).join(', ') + '</div>';
  }
  if (msg.thinking && (msg.thinking.html || msg.thinking.md)) {
    html += extraHtml(labels.thinking, msg.thinking.html || md2html(msg.thinking.md), print);
  }
  for (const tool of msg.tools || []) html += extraHtml(tool.title, md2html(tool.md), print);
  const body = msg.html || md2html(msg.md, { breaks: msg.role === 'user' });
  if (body) html += '<div class="body">' + body + '</div>';
  if (msg.sources && msg.sources.length) {
    html +=
      '<div class="sources"><b>' + escapeHtml(labels.sources) + '</b><ol>' +
      msg.sources
        .map((s) => '<li><a href="' + escapeHtml(safeUrl(s.url)) + '">' + escapeHtml(s.title || s.url) + '</a></li>')
        .join('') +
      '</ol></div>';
  }
  return html + '</section>';
}

function rawMessage(msg, conv, { labels, include = {} }) {
  const time = include.times && msg.time ? ' (' + timeText(msg.time) + ')' : '';
  const parts = [];
  if (msg.attachments && msg.attachments.length) parts.push('[' + labels.attachments + ': ' + msg.attachments.map((a) => a.name).join(', ') + ']');
  if (msg.thinking && msg.thinking.md) parts.push('[' + labels.thinking + ']\n' + mdToText(msg.thinking.md) + '\n[/' + labels.thinking + ']');
  for (const tool of msg.tools || []) parts.push('[' + tool.title + ']\n' + mdToText(tool.md));
  parts.push(mdToText(msg.md));
  if (msg.sources && msg.sources.length) parts.push(labels.sources + ':\n' + msg.sources.map((s, i) => i + 1 + '. ' + s.url).join('\n'));
  return (
    '<section class="msg"><div class="who">' + escapeHtml(whoOf(msg, conv, labels) + time) + ':</div>' +
    '<div class="text">' + escapeHtml(parts.join('\n\n')) + '</div></section>'
  );
}

/** Nur der <body>-Inhalt - die PDF-Seite setzt ihn selbst ein. */
export function renderBody(conv, { labels, include = {}, mode = 'styled', print = false, exportedAt = new Date() }) {
  if (mode === 'raw') {
    const meta = [conv.platformName, conv.model, conv.createdAt ? isoDate(conv.createdAt) : '', conv.url].filter(Boolean).join(' · ');
    return (
      '<div class="doc"><header class="raw-head"><h1>' + escapeHtml(conv.title || 'Chat') + '</h1><div>' + escapeHtml(meta) + '</div></header>' +
      conv.messages.map((m) => rawMessage(m, conv, { labels, include })).join('<hr class="sep" />') +
      '</div>'
    );
  }
  return (
    '<div class="doc">' + headerHtml(conv, labels) + '<main>' +
    conv.messages.map((m) => styledMessage(m, conv, { labels, include, print })).join('') +
    '</main><footer class="doc-foot">' + escapeHtml(labels.exportedWith + ' · ' + isoDate(exportedAt)) + '</footer></div>'
  );
}

/** Komplette, eigenständige HTML-Datei (CSS inline, keine externen Dateien). */
export function toHtml(conv, opts) {
  const { lang = 'en', fontSize = 'm', paper = 'a4', mode = 'styled' } = opts;
  return (
    '<!doctype html>\n<html lang="' + escapeHtml(lang) + '">\n<head>\n<meta charset="utf-8" />\n' +
    '<meta name="viewport" content="width=device-width, initial-scale=1" />\n' +
    '<meta name="generator" content="AI Chat Export" />\n' +
    '<title>' + escapeHtml(conv.title || 'Chat') + '</title>\n' +
    '<style>' + documentCss({ mode, fontSize, paper }) + '</style>\n</head>\n<body>\n' +
    renderBody(conv, Object.assign({}, opts, { print: false })) +
    '\n</body>\n</html>\n'
  );
}

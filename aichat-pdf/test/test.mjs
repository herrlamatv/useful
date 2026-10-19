/* ai chat exporter - by herrlamatv
 * Testet Plattform-Erkennung, Manifest, Dateinamen, Frontmatter, Markdown-
 * Renderer, ChatGPT-/Claude-Parser, Zusammenführen API ↔ DOM, Filter,
 * Exporter, ZIP und die Oberflächentexte. Ohne Abhängigkeiten:
 *
 *   node test/test.mjs
 */

import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { inflateRawSync } from 'node:zlib';
import { PLATFORMS, platformFor, scriptFiles, ALL_MATCHES } from '../src/platforms.js';
import { mergeSettings, setPath, DEFAULT_SETTINGS, FORMATS } from '../src/settings.js';
import { sanitizeSegment, buildPath, fillTemplate, isoDate } from '../src/export/filename.js';
import { yamlString, parseTags, frontmatter } from '../src/export/frontmatter.js';
import { md2html, safeUrl, escapeHtml } from '../src/export/md2html.js';
import { mdToText, toText } from '../src/export/text.js';
import { toMarkdown } from '../src/export/markdown.js';
import { toJson, SCHEMA } from '../src/export/json.js';
import { toHtml, renderBody, documentCss } from '../src/export/html.js';
import { parseChatGPT, resolveCitations, assetPointers } from '../src/parse/chatgpt.js';
import { parseClaude } from '../src/parse/claude.js';
import { buildConversation, filterConversation, attachDomHtml, assignKeys, imageUrls, replaceImageUrls, summarize, plausible } from '../src/model.js';
import { ZipWriter, crc32 } from '../src/zip.js';
import enMessages from '../ui/i18n/en.js';
import deMessages from '../ui/i18n/de.js';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

let fails = 0;
const ok = (name, cond, extra) => {
  if (cond) console.log('  ok   ' + name);
  else {
    fails++;
    console.log('  FAIL ' + name + (extra !== undefined ? '  -> ' + JSON.stringify(extra) : ''));
  }
};
const head = (title) => console.log('\n== ' + title + ' ==');

const LABELS = { you: 'You', thinking: 'Thinking', sources: 'Sources', attachments: 'Attachments', source: 'Source', exportedWith: 'Exported with AI Chat Export' };
const FIXED = new Date(2026, 9, 19, 14, 5, 0);

/* ------------------------------------------------------------------ */
head('Plattformen');

ok('chatgpt.com', platformFor('https://chatgpt.com/c/abc').id === 'chatgpt');
ok('chat.openai.com', platformFor('https://chat.openai.com/c/abc').id === 'chatgpt');
ok('claude.ai', platformFor('https://claude.ai/chat/x').id === 'claude');
ok('grok.com', platformFor('https://grok.com/c/1').id === 'grok');
ok('x.com/i/grok -> grok', platformFor('https://x.com/i/grok?conversation=1').id === 'grok');
ok('x.com/home -> universal', platformFor('https://x.com/home').id === 'universal');
ok('copilot.com -> copilot', platformFor('https://copilot.com/chats/abc').id === 'copilot');
ok('m365 -> copilot', platformFor('https://m365.cloud.microsoft/chat').id === 'copilot');
ok('gemini', platformFor('https://gemini.google.com/app/123').id === 'gemini');
ok('unbekannte Seite -> universal', platformFor('https://example.org/chat').id === 'universal');
ok('chrome:// -> nichts', platformFor('chrome://extensions') === null);
ok('Web Store -> nichts', platformFor('https://chromewebstore.google.com/detail/x') === null);
ok('Müll -> nichts', platformFor('not a url') === null);
ok('Bulk nur ChatGPT + Claude', PLATFORMS.filter((p) => p.bulk).map((p) => p.id).join() === 'chatgpt,claude');
ok('Universal-Dateien mit Picker', scriptFiles('universal').includes('src/content/picker.js'));

head('Manifest passt zu platforms.js');

const manifest = JSON.parse(readFileSync(join(ROOT, 'manifest.json'), 'utf8'));
for (const p of PLATFORMS) {
  const entry = manifest.content_scripts.find((c) => c.js.includes('src/content/adapters/' + p.id + '.js'));
  ok(p.id + ': Content-Script-Eintrag', !!entry);
  if (!entry) continue;
  ok(p.id + ': gleiche Muster', JSON.stringify(entry.matches) === JSON.stringify(p.matches), entry.matches);
  ok(p.id + ': gleiche Dateien', JSON.stringify(entry.js) === JSON.stringify(scriptFiles(p.id)), entry.js);
  ok(p.id + ': host_permissions', p.matches.every((m) => manifest.host_permissions.includes(m)));
}
ok('keine überzähligen Hosts', manifest.host_permissions.every((h) => ALL_MATCHES.includes(h)));
for (const file of [...new Set(manifest.content_scripts.flatMap((c) => c.js)), 'src/content/adapters/universal.js', 'src/content/picker.js', 'src/content/screenshot.js', manifest.background.service_worker, manifest.action.default_popup, manifest.options_ui.page, 'ui/render.html', 'ui/bulk.html']) {
  ok('Datei da: ' + file, existsSync(join(ROOT, file)));
}
for (const size of [16, 32, 48, 128]) ok('Icon ' + size, existsSync(join(ROOT, 'icons', 'icon' + size + '.png')));
ok('Berechtigungen', ['storage', 'downloads', 'contextMenus', 'scripting', 'activeTab', 'debugger'].every((p) => manifest.permissions.includes(p)));

head('Einstellungen');

ok('Müll -> Standard', JSON.stringify(mergeSettings(null)) === JSON.stringify(mergeSettings(DEFAULT_SETTINGS)));
ok('falsches Format -> md', mergeSettings({ format: 'exe' }).format === 'md');
ok('gültiges Format bleibt', mergeSettings({ format: 'pdf-raw' }).format === 'pdf-raw');
ok('Methode geprüft', mergeSettings({ pdfMethod: 'fax' }).pdfMethod === 'direct');
ok('include ergänzt', mergeSettings({ include: { thinking: false } }).include.sources === true);
ok('include nur bool', mergeSettings({ include: { images: 'yes' } }).include.images === true);
ok('leerer Dateiname -> Standard', mergeSettings({ file: '' }).file === DEFAULT_SETTINGS.file);
ok('setPath verschachtelt', setPath(DEFAULT_SETTINGS, 'include.tools', true).include.tools === true);
ok('setPath ändert Original nicht', DEFAULT_SETTINGS.include.tools === false);
ok('alle Formate haben Text', FORMATS.every((f) => enMessages['format.' + f] && deMessages['format.' + f]));

head('Dateinamen');

ok('verbotene Zeichen', sanitizeSegment('a/b:c*d?e"f<g>h|i') === 'a b c d e f g h i', sanitizeSegment('a/b:c*d?e"f<g>h|i'));
ok('Steuerzeichen und Zero-Width', sanitizeSegment('a\u0007b\u200bc') === 'a b c', sanitizeSegment('a\u0007b\u200bc'));
ok('Punkte am Ende weg', sanitizeSegment('Titel...') === 'Titel');
ok('Punkt/Tilde vorn weg', sanitizeSegment('.~versteckt') === 'versteckt');
ok('reservierter Name', sanitizeSegment('CON') === '_CON' && sanitizeSegment('com1.txt') === '_com1.txt');
ok('Länge begrenzt', sanitizeSegment('x'.repeat(300), 50).length === 50);
ok('Emoji bleibt ganz', sanitizeSegment('😀😀😀', 2) === '😀😀');
const sample = { platform: 'claude', platformName: 'Claude', title: 'Python: Listen / sortieren?', model: 'opus', id: 'abc', createdAt: '2026-01-02T10:00:00Z' };
ok('Vorlage', fillTemplate('{platform}-{date}-{created}-{model}-{id}-{unknown}', sample, FIXED) === 'Claude-2026-10-19-2026-01-02-opus-abc-{unknown}');
ok('Titel ohne Schrägstriche', !fillTemplate('{title}', sample, FIXED).includes('/'));
const p1 = buildPath({ folder: 'AI-Chats/{platform}', file: '{date} {title}' }, sample, 'md', FIXED);
ok('Pfad', p1 === 'AI-Chats/Claude/2026-10-19 Python Listen sortieren.md', p1);
ok('kein ..', !buildPath({ folder: '../../x/./y', file: '{title}' }, sample, 'md', FIXED).includes('..'));
ok('leerer Ordner', buildPath({ folder: '', file: '{title}' }, sample, 'pdf', FIXED) === 'Python Listen sortieren.pdf');
ok('Uhrzeit', fillTemplate('{time}', sample, FIXED) === '14-05');
ok('isoDate Müll', isoDate('kaputt') === '');

head('Frontmatter');

ok('yamlString Anführungszeichen', yamlString('a "b" \\ c') === '"a \\"b\\" \\\\ c"');
ok('yamlString Zeilenumbruch', yamlString('a\nb') === '"a\\nb"');
ok('Tags', JSON.stringify(parseTags('#AI Chat, KI; ai-chat\nFoo')) === '["ai-chat","ki","foo"]', parseTags('#AI Chat, KI; ai-chat\nFoo'));
const fm = frontmatter(Object.assign({ messages: [1, 2] }, sample, { url: 'https://claude.ai/chat/abc' }), { tags: 'ai-chat', exportedAt: new Date('2026-10-19T12:00:00Z') });
ok('Frontmatter Rahmen', fm.startsWith('---\n') && fm.endsWith('---\n'));
ok('Frontmatter Titel', fm.includes('title: "Python: Listen / sortieren?"'));
ok('Frontmatter Plattform-Tag', fm.includes('  - "claude"') && fm.includes('  - "ai-chat"'));
ok('Frontmatter Datum', fm.includes('exported: 2026-10-19T12:00:00Z') && fm.includes('messages: 2'));

head('Markdown -> HTML');

ok('Überschrift', md2html('## Hallo *Welt*') === '<h2>Hallo <em>Welt</em></h2>');
ok('Absatz escaped', md2html('a <b> & c') === '<p>a &lt;b&gt; &amp; c</p>');
ok('fett/kursiv/durch', md2html('**f** _k_ ~~d~~') === '<p><strong>f</strong> <em>k</em> <del>d</del></p>', md2html('**f** _k_ ~~d~~'));
ok('snake_case bleibt', md2html('mein_variablen_name') === '<p>mein_variablen_name</p>');
ok('Inline-Code', md2html('x `a<b>*c*` y') === '<p>x <code>a&lt;b&gt;*c*</code> y</p>');
const code = md2html('```python\nprint("<x>")\n```');
ok('Code-Block mit Sprache', code === '<pre><code class="language-python">print(&quot;&lt;x&gt;&quot;)</code></pre>', code);
ok('Code-Block ohne Ende', md2html('```\nnur code').startsWith('<pre><code>nur code'));
const list = md2html('- a\n- b\n  - c\n  - d\n- e');
ok('verschachtelte Liste', list === '<ul>\n<li>a</li>\n<li>b\n<ul>\n<li>c</li>\n<li>d</li>\n</ul></li>\n<li>e</li>\n</ul>', list);
ok('nummerierte Liste mit Start', md2html('3. x\n4. y').startsWith('<ol start="3">'));
ok('Aufgaben', md2html('- [x] fertig\n- [ ] offen').includes('<input type="checkbox" disabled checked />'));
const table = md2html('| A | B |\n|:--|--:|\n| 1 | 2 \\| 3 |');
ok('Tabelle', table.includes('<th style="text-align:left">A</th>') && table.includes('<td style="text-align:right">2 | 3</td>'), table);
ok('Zitat', md2html('> zitat\n> weiter') === '<blockquote>\n<p>zitat\nweiter</p>\n</blockquote>');
ok('Trennlinie', md2html('---') === '<hr />');
ok('Link', md2html('[Doku](https://a.b/c?x=1&y=2)') === '<p><a href="https://a.b/c?x=1&amp;y=2">Doku</a></p>', md2html('[Doku](https://a.b/c?x=1&y=2)'));
ok('javascript-Link entschärft', md2html('[x](javascript:alert(1))').includes('href="#"'));
ok('Bild', md2html('![Alt](https://a.b/i.png)') === '<p><img src="https://a.b/i.png" alt="Alt" /></p>');
ok('nackte URL', md2html('siehe https://a.b/c.').includes('<a href="https://a.b/c">https://a.b/c</a>.'));
ok('Mathe inline', md2html('Die Formel $E=mc^2$ gilt') === '<p>Die Formel <span class="math">E=mc^2</span> gilt</p>');
ok('Geldbetrag keine Mathe', !md2html('kostet $5 und $10').includes('math'));
ok('Mathe Block', md2html('$$\na^2+b^2\n$$') === '<div class="math display">a^2+b^2</div>');
ok('Mathe \\[ \\]', md2html('\\[x\\]') === '<div class="math display">x</div>');
ok('breaks', md2html('a\nb', { breaks: true }) === '<p>a<br />\nb</p>');
ok('safeUrl data-Bild', safeUrl('data:image/png;base64,AA', { image: true }).startsWith('data:image'));
ok('safeUrl data sonst nicht', safeUrl('data:text/html,x') === '#');
ok('escapeHtml', escapeHtml('<"&>') === '&lt;&quot;&amp;&gt;');

head('Markdown -> Text');

ok('Link -> Text (URL)', mdToText('[Doku](https://a.b)') === 'Doku (https://a.b)');
ok('Betonung weg', mdToText('**fett** und *kursiv*') === 'fett und kursiv');
ok('Überschrift', mdToText('## Titel') === 'Titel');
ok('Code bleibt eingerückt', mdToText('```js\nlet a = 1;\n```') === 'let a = 1;');
ok('Liste', mdToText('- a\n- [x] b') === '• a\n☑ b');
ok('Tabelle', mdToText('| a | b |\n|---|---|\n| 1 | 2 |') === 'a  |  b\n1  |  2');

head('ChatGPT-Parser');

const msg = (id, role, content, extra = {}) => ({ id, author: { role, name: extra.name }, create_time: 1760000000 + id.length, content, metadata: extra.metadata || {}, recipient: extra.recipient || 'all' });
const gpt = {
  title: 'Test-Chat',
  conversation_id: 'conv-1',
  create_time: 1760000000,
  update_time: 1760000100,
  default_model_slug: 'gpt-5',
  current_node: 'a3',
  mapping: {
    root: { id: 'root', parent: null, children: ['sys'], message: null },
    sys: { id: 'sys', parent: 'root', children: ['u1'], message: msg('sys', 'system', { content_type: 'text', parts: [''] }) },
    u1: { id: 'u1', parent: 'sys', children: ['a1old', 'a1'], message: msg('u1', 'user', { content_type: 'text', parts: ['Hallo'] }) },
    a1old: { id: 'a1old', parent: 'u1', children: [], message: msg('a1old', 'assistant', { content_type: 'text', parts: ['ALTER AST'] }) },
    a1: {
      id: 'a1',
      parent: 'u1',
      children: ['u2'],
      message: msg('a1', 'assistant', { content_type: 'text', parts: ['Antwort mit Quelle \ue200cite\ue202turn0search1\ue201 und mehr.'] }, {
        metadata: {
          model_slug: 'gpt-5-thinking',
          content_references: [{ matched_text: '\ue200cite\ue202turn0search1\ue201', type: 'grouped_webpages', items: [{ title: 'Wiki', url: 'https://de.wikipedia.org/x' }], alt: '([Wiki](https://de.wikipedia.org/x))' }]
        }
      })
    },
    u2: {
      id: 'u2',
      parent: 'a1',
      children: ['t1'],
      message: msg('u2', 'user', { content_type: 'multimodal_text', parts: [{ content_type: 'image_asset_pointer', asset_pointer: 'sediment://file_1' }, 'Was ist das?'] }, { metadata: { attachments: [{ name: 'foto.png' }] } })
    },
    t1: { id: 't1', parent: 'u2', children: ['c1'], message: msg('t1', 'assistant', { content_type: 'thoughts', thoughts: [{ summary: 'Bild ansehen', content: 'Ich schaue genau hin.' }] }) },
    c1: { id: 'c1', parent: 't1', children: ['o1'], message: msg('c1', 'assistant', { content_type: 'code', language: 'python', text: 'print(1)' }, { recipient: 'python' }) },
    o1: { id: 'o1', parent: 'c1', children: ['r1'], message: msg('o1', 'tool', { content_type: 'execution_output', text: '1' }, { name: 'python' }) },
    r1: { id: 'r1', parent: 'o1', children: ['a3'], message: msg('r1', 'assistant', { content_type: 'reasoning_recap', content: 'Thought for 3s' }) },
    a3: { id: 'a3', parent: 'r1', children: [], message: msg('a3', 'assistant', { content_type: 'text', parts: ['Das ist eine Katze. 【4†source】'] }) },
    hidden: { id: 'hidden', parent: 'u1', children: [], message: msg('hidden', 'user', { content_type: 'text', parts: ['x'] }, { metadata: { is_visually_hidden_from_conversation: true } }) }
  }
};
const g = parseChatGPT(gpt, { assets: { 'sediment://file_1': 'https://chatgpt.com/backend-api/estuary/content?id=file_1' } });
ok('Titel/Modell', g.title === 'Test-Chat' && g.model === 'gpt-5-thinking');
ok('sichtbarer Ast, keine System-Nachricht', g.messages.length === 4 && !g.messages.some((m) => m.md.includes('ALTER AST')), g.messages.map((m) => m.role));
ok('Zitat ersetzt + Marker weg', g.messages[1].md === 'Antwort mit Quelle ([Wiki](https://de.wikipedia.org/x)) und mehr.', g.messages[1].md);
ok('Quelle gesammelt', g.messages[1].sources.length === 1 && g.messages[1].sources[0].url === 'https://de.wikipedia.org/x');
ok('Bild im Prompt', g.messages[2].md.startsWith('![image](https://chatgpt.com/backend-api/estuary/content?id=file_1)'), g.messages[2].md);
ok('Anhang', g.messages[2].attachments[0].name === 'foto.png');
ok('Gedankengang', g.messages[3].thinking && g.messages[3].thinking.md.includes('**Bild ansehen**'));
ok('Werkzeuge gesammelt', g.messages[3].tools.length === 2 && g.messages[3].tools[0].md.startsWith('```python'), g.messages[3].tools);
ok('Antwort zusammengefasst + alte Marker weg', g.messages[3].md === 'Das ist eine Katze.', g.messages[3].md);
ok('IDs der Antwort', g.messages[3].ids.join() === 't1,c1,o1,a3', g.messages[3].ids);
ok('Zeiten ISO', /^2025-|^2026-/.test(g.createdAt));
ok('assetPointers', assetPointers(gpt).join() === 'sediment://file_1');
ok('ohne current_node: letzter Ast', parseChatGPT(Object.assign({}, gpt, { current_node: null })).messages.length >= 2);
ok('resolveCitations alt-Fallback', resolveCitations('a XY b', { content_references: [{ matched_text: 'XY', items: [{ title: 'T', url: 'https://t.de' }] }] }) === 'a [T](https://t.de) b');

head('Claude-Parser');

const claude = {
  uuid: 'c-1',
  name: 'Claude-Test',
  model: 'claude-opus-5-5',
  created_at: '2026-10-18T10:00:00Z',
  updated_at: '2026-10-18T11:00:00Z',
  current_leaf_message_uuid: 'm4',
  chat_messages: [
    { uuid: 'm1', sender: 'human', index: 0, parent_message_uuid: '00000000-0000-4000-8000-000000000000', created_at: '2026-10-18T10:00:00Z', content: [{ type: 'text', text: 'Mach mir eine Seite' }], attachments: [{ file_name: 'notizen.txt' }], files: [{ file_kind: 'image', file_name: 'bild.png', preview_url: '/api/x/files/1/preview' }] },
    { uuid: 'm2old', sender: 'assistant', index: 1, parent_message_uuid: 'm1', content: [{ type: 'text', text: 'ALTE ANTWORT' }] },
    {
      uuid: 'm2',
      sender: 'assistant',
      index: 1,
      parent_message_uuid: 'm1',
      created_at: '2026-10-18T10:01:00Z',
      content: [
        { type: 'thinking', thinking: 'Erst planen.' },
        { type: 'tool_use', name: 'web_search', input: { query: 'html' } },
        { type: 'tool_result', name: 'web_search', content: [{ type: 'knowledge', title: 'MDN', url: 'https://developer.mozilla.org' }] },
        { type: 'text', text: 'Hier:', citations: [{ url: 'https://example.com', title: 'Ex' }] },
        { type: 'tool_use', name: 'artifacts', input: { id: 'page', command: 'create', type: 'text/html', title: 'Seite', content: '<h1>Hi</h1>' } }
      ]
    },
    { uuid: 'm3', sender: 'human', index: 2, parent_message_uuid: 'm2', content: [{ type: 'text', text: 'Größer bitte' }] },
    { uuid: 'm4', sender: 'assistant', index: 3, parent_message_uuid: 'm3', content: [{ type: 'tool_use', name: 'artifacts', input: { id: 'page', command: 'update', old_str: '<h1>', new_str: '<h1 style="font-size:4em">' } }, { type: 'text', text: 'Erledigt.' }] }
  ]
};
const c = parseClaude(claude);
ok('Ast über parent_message_uuid', c.messages.length === 4 && !c.messages.some((m) => m.md.includes('ALTE')), c.messages.map((m) => m.md));
ok('Bild aus files mit Origin', c.messages[0].md.startsWith('![bild.png](https://claude.ai/api/x/files/1/preview)'), c.messages[0].md);
ok('Text-Anhang', c.messages[0].attachments[0].name === 'notizen.txt');
ok('Gedankengang', c.messages[1].thinking.md === 'Erst planen.');
ok('Quellen aus Zitat und Suche', c.messages[1].sources.map((s) => s.url).join() === 'https://developer.mozilla.org,https://example.com', c.messages[1].sources);
ok('Werkzeug-Aufruf', c.messages[1].tools.length === 1 && c.messages[1].tools[0].title === 'Tool · web_search');
ok('Artifact als Code-Block', c.messages[1].md.includes('**Seite**\n\n```html\n<h1>Hi</h1>\n```'), c.messages[1].md);
ok('Artifact-Update als Diff', c.messages[3].md.includes('```diff\n- <h1>\n+ <h1 style="font-size:4em">\n```'), c.messages[3].md);
ok('Kopf', c.title === 'Claude-Test' && c.model === 'claude-opus-5-5' && c.id === 'c-1');
ok('ohne Baum: nach index', parseClaude({ chat_messages: [{ uuid: 'b', sender: 'assistant', index: 1, content: [{ type: 'text', text: '2' }] }, { uuid: 'a', sender: 'human', index: 0, text: '1' }] }).messages.map((m) => m.md).join() === '1,2');

head('Zusammenführen');

const raw = (api, dom) => ({ platform: 'chatgpt', title: 'DOM-Titel', url: 'https://chatgpt.com/c/conv-1', dom, api });
const dom = [
  { id: 'u1', ids: ['u1'], role: 'user', md: 'Hallo', html: '<p>Hallo</p>' },
  { id: 'a1', ids: ['a1'], role: 'assistant', md: 'x', html: '<p>Antwort mit Quelle (Wiki) und mehr.</p>' },
  { id: 'u2', ids: ['u2'], role: 'user', md: 'x', html: '<p><img src="x" />Was ist das?</p>' },
  { id: 'a3', ids: ['a3'], role: 'assistant', md: 'x', html: '<p>Das ist eine <b>Katze</b>.</p>' }
];
const conv = buildConversation(raw({ kind: 'chatgpt', data: gpt, assets: {} }, dom));
ok('Quelle API', conv.source === 'api' && conv.title === 'Test-Chat');
ok('DOM-HTML angehängt (alle passen)', conv.messages.every((m) => m.html), conv.messages.map((m) => !!m.html));
const partial = buildConversation(raw({ kind: 'chatgpt', data: gpt, assets: {} }, dom.slice(2)));
ok('nur teilweise im DOM -> gar kein DOM-HTML', partial.messages.every((m) => !m.html));
ok('plausible: zu wenig Text', !plausible('<p>a</p>', 'ein langer Text mit vielen Wörtern'));
ok('plausible: Bild fehlt', !plausible('<p>Text</p>', '![x](y) Text'));
const byOrder = attachDomHtml([{ role: 'user', md: 'a' }, { role: 'assistant', md: 'b' }], [{ role: 'user', html: '<p>a</p>' }, { role: 'assistant', html: '<p>b</p>' }], 'order');
ok('Zuordnung nach Reihenfolge', byOrder[1].html === '<p>b</p>');
const domOnly = buildConversation({ platform: 'gemini', title: '  Mein   Chat ', dom: [{ role: 'user', md: 'Frage' }, { role: 'assistant', md: '', html: '' }, { role: 'assistant', md: 'Antwort' }] });
ok('nur DOM, leere raus', domOnly.source === 'dom' && domOnly.messages.length === 2 && domOnly.title === 'Mein Chat');
ok('unbekannte Plattform -> Universal', buildConversation({ platform: 'xyz', dom: [] }).platformName === 'Universal');
const broken = buildConversation(raw({ kind: 'chatgpt', data: { mapping: 'kaputt' } }, dom));
ok('kaputte API -> DOM', broken.source === 'dom' && broken.messages.length === 4);
const keyed = assignKeys([{ role: 'user', md: 'ok' }, { role: 'user', md: 'ok' }, { role: 'assistant', md: 'ok' }]);
ok('Schlüssel eindeutig', new Set(keyed.map((m) => m.key)).size === 3);
ok('Schlüssel stabil', assignKeys([{ role: 'user', md: 'ok' }])[0].key === keyed[0].key);
ok('summarize', summarize(conv).messages.length === 4 && summarize(conv).messages[3].thinking === true);

head('Filtern & Bilder');

const fRoles = filterConversation(conv, { roles: 'assistant' });
ok('nur Antworten', fRoles.messages.every((m) => m.role === 'assistant') && fRoles.messages.length === 2);
const fKeys = filterConversation(conv, { keys: [conv.messages[0].key] });
ok('Auswahl', fKeys.messages.length === 1 && fKeys.messages[0].md === 'Hallo');
const fInc = filterConversation(conv, { include: { thinking: false, sources: false, tools: false, attachments: false, images: false } });
ok('Extras aus', fInc.messages.every((m) => !m.thinking && !m.sources.length && !m.tools.length && !m.attachments.length));
ok('Bilder aus', !fInc.messages[2].md.includes('![') && !fInc.messages[2].html.includes('<img'));
ok('Original unverändert', conv.messages[3].thinking !== null);
const imgConv = { messages: [{ md: '![a](https://x.de/1.png) ![b](data:image/png;base64,AA)', html: '<img src="https://x.de/2.png?a=1&amp;b=2">' }] };
ok('imageUrls', imageUrls(imgConv).join() === 'https://x.de/1.png,https://x.de/2.png?a=1&b=2', imageUrls(imgConv));
const swapped = replaceImageUrls(imgConv, { 'https://x.de/2.png?a=1&b=2': 'data:image/png;base64,ZZ' });
ok('replaceImageUrls (auch &amp;)', swapped.messages[0].html === '<img src="data:image/png;base64,ZZ">', swapped.messages[0].html);

head('Exporter');

const opts = { labels: LABELS, include: { times: true }, exportedAt: new Date('2026-10-19T12:00:00Z'), tags: 'ai-chat' };
const md = toMarkdown(conv, Object.assign({ frontmatter: true }, opts));
ok('MD Frontmatter + Titel', md.startsWith('---\n') && md.includes('\n# Test-Chat\n'));
ok('MD Rollen', md.includes('## You') && md.includes('## ChatGPT'));
ok('MD Gedankengang als Callout', md.includes('> [!note]- Thinking\n> **Bild ansehen**'));
ok('MD Werkzeug als Callout', md.includes('> [!example]- Tool · python'));
ok('MD Quellen', md.includes('**Sources**\n1. [Wiki](https://de.wikipedia.org/x)'));
ok('MD ohne Frontmatter mit Meta-Zeile', toMarkdown(conv, Object.assign({ frontmatter: false }, opts)).includes('[Source](https://chatgpt.com/c/conv-1)'));
const txt = toText(conv, opts);
ok('TXT Kopf + Rollen', txt.startsWith('Test-Chat\n') && txt.includes('You (') && txt.includes('ChatGPT ('));
ok('TXT ohne Markdown-Link', txt.includes('Wiki (https://de.wikipedia.org/x)'));
const json = JSON.parse(toJson(conv, opts));
ok('JSON Schema + Nachrichten', json.schema === SCHEMA && json.messages.length === 4 && json.messages[3].thinking.includes('Bild'));
ok('JSON ohne eingebettete Bilder', !toJson({ messages: [{ role: 'user', md: '![a](data:image/png;base64,AAAA)' }] }).includes('base64'));
const html = toHtml(Object.assign({}, conv, { title: '<script>' }), Object.assign({ lang: 'de' }, opts));
ok('HTML eigenständig', html.startsWith('<!doctype html>') && html.includes('<html lang="de">') && html.includes('<style>'));
ok('HTML Titel escaped', html.includes('<title>&lt;script&gt;</title>') && !html.includes('<title><script>'));
ok('HTML nutzt DOM-HTML', html.includes('Das ist eine <b>Katze</b>.'));
ok('HTML Gedankengang zu', html.includes('<details class="extra"><summary>Thinking</summary>'));
ok('PDF-Body Gedankengang offen', renderBody(conv, Object.assign({ print: true }, opts)).includes('<details class="extra" open>'));
const rawBody = renderBody(conv, Object.assign({ mode: 'raw' }, opts));
ok('Raw ohne HTML-Tags im Text', rawBody.includes('<div class="text">') && !rawBody.includes('<b>Katze</b>'));
ok('CSS Papier', documentCss({ paper: 'letter' }).includes('size:letter') && documentCss({ print: true }).includes('counter(pages)'));
ok('CSS Raw', documentCss({ mode: 'raw' }).includes('monospace'));

head('ZIP');

const zip = new ZipWriter();
const big = 'Hallo Welt! '.repeat(500);
await zip.add('Claude/a.md', big, FIXED);
await zip.add('Claude/a.md', 'zweite', FIXED);
await zip.add('Ümläut/ä.txt', 'kurz', FIXED);
const bytes = new Uint8Array(await zip.blob().arrayBuffer());
const view = new DataView(bytes.buffer);
ok('lokaler Header', view.getUint32(0, true) === 0x04034b50);
const endAt = bytes.length - 22;
ok('End-Record', view.getUint32(endAt, true) === 0x06054b50 && view.getUint16(endAt + 10, true) === 3);
const entries = [];
let pos = view.getUint32(endAt + 16, true);
for (let i = 0; i < 3; i++) {
  const nameLen = view.getUint16(pos + 28, true);
  entries.push({
    method: view.getUint16(pos + 10, true),
    crc: view.getUint32(pos + 16, true),
    comp: view.getUint32(pos + 20, true),
    size: view.getUint32(pos + 24, true),
    offset: view.getUint32(pos + 42, true),
    name: new TextDecoder().decode(bytes.subarray(pos + 46, pos + 46 + nameLen))
  });
  pos += 46 + nameLen;
}
ok('Namen + Dubletten', entries.map((e) => e.name).join('|') === 'Claude/a.md|Claude/a (2).md|Ümläut/ä.txt', entries.map((e) => e.name));
const first = entries[0];
const dataStart = first.offset + 30 + view.getUint16(first.offset + 26, true);
const unpacked = inflateRawSync(bytes.subarray(dataStart, dataStart + first.comp));
ok('Deflate entpackt', first.method === 8 && unpacked.toString('utf8') === big);
ok('CRC stimmt', first.crc === crc32(new TextEncoder().encode(big)));
ok('kleine Datei gespeichert', entries[2].method === 0 && entries[2].comp === entries[2].size);
ok('crc32 bekannt', crc32(new TextEncoder().encode('123456789')) === 0xcbf43926);

head('Texte');

const enKeys = Object.keys(enMessages).sort();
const deKeys = Object.keys(deMessages).sort();
ok('DE und EN gleiche Schlüssel', JSON.stringify(enKeys) === JSON.stringify(deKeys), enKeys.filter((k) => !deKeys.includes(k)).concat(deKeys.filter((k) => !enKeys.includes(k))));
ok('keine leeren Texte', enKeys.every((k) => enMessages[k] && deMessages[k]));
for (const [file, keys] of Object.entries({ 'ui/popup.html': null, 'ui/options.html': null, 'ui/bulk.html': null, 'ui/render.html': null })) {
  const used = [...readFileSync(join(ROOT, file), 'utf8').matchAll(/data-i18n(?:-ph|-title)?="([^"]+)"/g)].map((m) => m[1]);
  const missing = used.filter((k) => !enMessages[k]);
  ok(file + ': alle Schlüssel da', !missing.length, missing);
}
for (const file of ['src/background.js', 'ui/popup.js', 'ui/options.js', 'ui/bulk.js', 'ui/render.js']) {
  const used = [...readFileSync(join(ROOT, file), 'utf8').matchAll(/\bt\('([a-z]+\.[\w.-]+)'/g)].map((m) => m[1]);
  const missing = used.filter((k) => !enMessages[k]);
  ok(file + ': alle t()-Schlüssel da', !missing.length, missing);
}
const loc = (lang) => JSON.parse(readFileSync(join(ROOT, '_locales', lang, 'messages.json'), 'utf8'));
ok('_locales gleich', JSON.stringify(Object.keys(loc('en'))) === JSON.stringify(Object.keys(loc('de'))));

console.log('\n' + (fails ? fails + ' FEHLER' : 'alles grün'));
process.exit(fails ? 1 : 0);

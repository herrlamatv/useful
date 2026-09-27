I18N.add({
  en: {
    docTitle: 'Live Code Editor', back: '← Overview', heading: 'Live Code Editor',
    loadTemplate: 'Load template', loadTemplateOpt: 'Load template…',
    tplHello: 'Hello World', tplButton: 'Animated button', tplCard: 'Glass card',
    tplBounce: 'Bouncing ball', tplClock: 'Digital clock (JS)',
    autorun: 'Auto-run', run: 'Run', runHint: 'Ctrl+Enter',
    layout: 'Layout', layoutHint: 'Switch layout',
    export: 'Export', exportHint: 'Save as HTML file', reset: 'Reset',
    preview: 'Preview', line: 'line', exportFile: 'my-page.html',
    helloTitle: 'Hello world', helloText: 'Edit HTML, CSS and JS on the left.',
    helloLog: 'Hello from the JS panel!', hoverMe: 'Hover me', click: 'Click!',
    cardText: 'Semi-transparent with blur.', clockLocale: 'en-GB'
  },
  de: {
    docTitle: 'Live Code Editor', back: '← Übersicht', heading: 'Live Code Editor',
    loadTemplate: 'Vorlage laden', loadTemplateOpt: 'Vorlage laden…',
    tplHello: 'Hello World', tplButton: 'Animierter Button', tplCard: 'Glas-Karte',
    tplBounce: 'Hüpfender Ball', tplClock: 'Digitaluhr (JS)',
    autorun: 'Auto-Run', run: 'Ausführen', runHint: 'Strg+Enter',
    layout: 'Layout', layoutHint: 'Layout wechseln',
    export: 'Export', exportHint: 'Als HTML-Datei speichern', reset: 'Reset',
    preview: 'Vorschau', line: 'Zeile', exportFile: 'meine-seite.html',
    helloTitle: 'Hallo Welt', helloText: 'Bearbeite HTML, CSS und JS links.',
    helloLog: 'Hallo aus dem JS-Panel!', hoverMe: 'Hover mich', click: 'Klick!',
    cardText: 'Halbtransparent mit Blur.', clockLocale: 'de-DE'
  }
});

const STORAGE_KEY = 'uitesting-code-editor';
const $ = id => document.getElementById(id);
const editors = { html: $('html'), css: $('css'), js: $('js') };
const output = $('output');
const consoleEl = $('console');

// Templates are built on demand so their texts follow the current language.
const getTemplates = () => ({
  hello: {
    html: `<h1>${I18N.t('helloTitle')}</h1>\n<p>${I18N.t('helloText')}</p>`,
    css: 'body {\n  font-family: system-ui, sans-serif;\n  display: grid;\n  place-items: center;\n  height: 100vh;\n  margin: 0;\n  background: #f3efe5;\n  color: #1f1d18;\n  text-align: center;\n}\nh1 { font-size: 3rem; margin: 0; letter-spacing: -.02em; }\np { color: #7c7466; }',
    js: `console.log('${I18N.t('helloLog')}');`
  },
  button: {
    html: `<button class="btn">${I18N.t('hoverMe')}</button>`,
    css: 'body { display:grid; place-items:center; height:100vh; margin:0; background:#111; }\n.btn {\n  padding: 16px 40px;\n  font-size: 1.2rem;\n  color: #fff;\n  border: 0;\n  border-radius: 50px;\n  cursor: pointer;\n  background: linear-gradient(90deg, #ff007a, #7a00ff, #00d4ff, #ff007a);\n  background-size: 300%;\n  animation: flow 4s linear infinite;\n  transition: transform .2s, box-shadow .2s;\n}\n.btn:hover {\n  transform: scale(1.08);\n  box-shadow: 0 0 30px #7a00ff;\n}\n@keyframes flow { to { background-position: 300%; } }',
    js: `document.querySelector('.btn').onclick = () => console.log('${I18N.t('click')}');`
  },
  card: {
    html: `<div class="card">\n  <h2>Glassmorphism</h2>\n  <p>${I18N.t('cardText')}</p>\n</div>`,
    css: 'body {\n  margin:0; height:100vh; display:grid; place-items:center;\n  font-family: system-ui;\n  background: url("data:image/svg+xml,%3Csvg xmlns=\'http://www.w3.org/2000/svg\'%3E%3Ccircle cx=\'30%25\' cy=\'35%25\' r=\'120\' fill=\'%23ff6b6b\'/%3E%3Ccircle cx=\'70%25\' cy=\'65%25\' r=\'140\' fill=\'%234ecdc4\'/%3E%3C/svg%3E"), #1a1a2e;\n}\n.card {\n  padding: 40px;\n  width: 300px;\n  color: white;\n  border-radius: 20px;\n  background: rgba(255,255,255,.12);\n  border: 1px solid rgba(255,255,255,.3);\n  backdrop-filter: blur(16px);\n  box-shadow: 0 8px 32px rgba(0,0,0,.35);\n}',
    js: ''
  },
  bounce: {
    html: '<div class="ball"></div>\n<div class="shadow"></div>',
    css: 'body { margin:0; height:100vh; display:flex; flex-direction:column; align-items:center; justify-content:center; background:#fdf6e3; }\n.ball {\n  width: 80px; height: 80px; border-radius: 50%;\n  background: radial-gradient(circle at 30% 30%, #ff9a9e, #e0245e);\n  animation: bounce .8s cubic-bezier(.5,0,.5,1) infinite alternate;\n}\n.shadow {\n  width: 80px; height: 14px; border-radius: 50%; background: #0003;\n  animation: shadow .8s cubic-bezier(.5,0,.5,1) infinite alternate;\n}\n@keyframes bounce { from { transform: translateY(-180px); } to { transform: translateY(0) scale(1.1, .9); } }\n@keyframes shadow { from { transform: scale(.4); opacity:.3; } to { transform: scale(1); opacity:1; } }',
    js: ''
  },
  clock: {
    html: '<div id="clock">00:00:00</div>\n<div id="date"></div>',
    css: 'body { margin:0; height:100vh; display:flex; flex-direction:column; align-items:center; justify-content:center; background:#000; color:#0f0; font-family: Consolas, monospace; }\n#clock { font-size: 5rem; text-shadow: 0 0 20px #0f0; }\n#date { opacity:.7; font-size:1.3rem; }',
    js: "function tick() {\n  const d = new Date();\n  document.getElementById('clock').textContent = d.toLocaleTimeString('" + I18N.t('clockLocale') + "');\n  document.getElementById('date').textContent = d.toLocaleDateString('" + I18N.t('clockLocale') + "', { weekday:'long', day:'numeric', month:'long', year:'numeric' });\n}\ntick();\nsetInterval(tick, 1000);"
  }
});

// Bridge script injected into the preview: forwards console output and errors to the parent.
const bridge = () => `<script>
(function(){
  function send(type, args){ parent.postMessage({ __editor: true, type, msg: Array.from(args).map(a => {
    try { return typeof a === 'object' ? JSON.stringify(a) : String(a); } catch(e) { return String(a); }
  }).join(' ') }, '*'); }
  ['log','info','warn','error'].forEach(k => { const o = console[k]; console[k] = function(){ send(k, arguments); o.apply(console, arguments); }; });
  window.onerror = function(m, s, l){ send('error', [m + ' (${I18N.t('line')} ' + l + ')']); };
})();
<\/script>`;

function buildDoc() {
  return `<!DOCTYPE html><html><head><meta charset="UTF-8">${bridge()}<style>${editors.css.value}</style></head>` +
         `<body>${editors.html.value}<script>${editors.js.value.replace(/<\/script>/gi, '<\\/script>')}<\/script></body></html>`;
}

function run() {
  consoleEl.innerHTML = '';
  output.srcdoc = buildDoc();
  save();
}

function save() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      html: editors.html.value, css: editors.css.value, js: editors.js.value,
      layout: $('workspace').className, autorun: $('autorun').checked
    }));
  } catch (e) { /* storage unavailable */ }
}

function load(tpl) {
  for (const k in editors) editors[k].value = tpl[k] || '';
  run();
}

window.addEventListener('message', e => {
  if (!e.data || !e.data.__editor) return;
  const line = document.createElement('div');
  line.className = e.data.type;
  line.textContent = '› ' + e.data.msg;
  consoleEl.appendChild(line);
  consoleEl.scrollTop = consoleEl.scrollHeight;
});

let timer;
for (const k in editors) {
  const ta = editors[k];
  ta.addEventListener('input', () => {
    if (!$('autorun').checked) return save();
    clearTimeout(timer);
    timer = setTimeout(run, 400);
  });
  ta.addEventListener('keydown', e => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const { selectionStart: s, selectionEnd: end, value } = ta;
      ta.value = value.slice(0, s) + '  ' + value.slice(end);
      ta.selectionStart = ta.selectionEnd = s + 2;
      ta.dispatchEvent(new Event('input'));
    } else if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault();
      run();
    }
  });
}

$('run').onclick = run;
$('autorun').onchange = save;
$('layout').onclick = () => {
  const ws = $('workspace');
  ws.className = ws.className === 'layout-side' ? 'layout-top' : 'layout-side';
  save();
};
$('template').onchange = e => {
  const tpl = getTemplates()[e.target.value];
  if (tpl) load(tpl);
  e.target.value = '';
};
$('reset').onclick = () => load(getTemplates().hello);
$('export').onclick = () => {
  const blob = new Blob([`<!DOCTYPE html>\n<html>\n<head>\n<meta charset="UTF-8">\n<style>\n${editors.css.value}\n</style>\n</head>\n<body>\n${editors.html.value}\n<script>\n${editors.js.value}\n<\/script>\n</body>\n</html>\n`], { type: 'text/html' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = I18N.t('exportFile');
  a.click();
  URL.revokeObjectURL(a.href);
};

// Initial state
let saved = null;
try { saved = JSON.parse(localStorage.getItem(STORAGE_KEY)); } catch (e) { /* ignore */ }
if (saved) {
  $('workspace').className = saved.layout || 'layout-side';
  $('autorun').checked = saved.autorun !== false;
  load(saved);
} else {
  load(getTemplates().hello);
}

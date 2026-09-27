// Tiny i18n helper shared by every page. English is the default, the choice is remembered per browser.
//
// Usage in a page:
//   <script src="../i18n.js"></script>            (before the page's own script)
//   I18N.add({ en: { hello: 'Hello {name}' }, de: { hello: 'Hallo {name}' } });
//   I18N.t('hello', { name: 'Max' })
//   <h1 data-i18n="title"></h1>                     → textContent
//   <p data-i18n-html="intro"></p>                  → innerHTML (only for own, trusted strings)
//   <input data-i18n-attr="placeholder:search;title:searchHint">
//   <div data-lang-switch></div>                    → filled with an EN / DE toggle
//   addEventListener('langchange', rerender)        → fired after switching
(function () {
  const KEY = 'uitoolbox-lang';
  const LANGS = ['en', 'de'];
  const dict = { en: {}, de: {} };
  let lang = 'en';
  try {
    const saved = localStorage.getItem(KEY);
    if (LANGS.includes(saved)) lang = saved;
  } catch (e) { /* storage unavailable */ }
  document.documentElement.lang = lang;

  // Default look for the switch; works on light and dark pages, pages may override it.
  const style = document.createElement('style');
  style.textContent =
    '.lang-switch{display:inline-flex;gap:2px;padding:2px;border-radius:8px;background:rgba(127,127,127,.14);vertical-align:middle}' +
    '.lang-switch button{all:unset;cursor:pointer;font:600 11px/1 system-ui,sans-serif;letter-spacing:.05em;padding:5px 8px;border-radius:6px;color:inherit;opacity:.55}' +
    '.lang-switch button:hover{opacity:.85}' +
    '.lang-switch button.on{opacity:1;background:rgba(127,127,127,.22)}';
  document.head.prepend(style);

  function t(key, vars) {
    let s = dict[lang][key] ?? dict.en[key] ?? key;
    if (vars) s = s.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));
    return s;
  }

  function apply(root = document) {
    root.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
    root.querySelectorAll('[data-i18n-html]').forEach(el => { el.innerHTML = t(el.dataset.i18nHtml); });
    root.querySelectorAll('[data-i18n-attr]').forEach(el => {
      el.dataset.i18nAttr.split(';').forEach(pair => {
        const [attr, key] = pair.split(':').map(s => s.trim());
        if (attr && key) el.setAttribute(attr, t(key));
      });
    });
    document.querySelectorAll('[data-lang-switch]').forEach(buildSwitch);
  }

  function buildSwitch(host) {
    host.classList.add('lang-switch');
    if (!host.children.length) {
      LANGS.forEach(l => {
        const b = document.createElement('button');
        b.type = 'button';
        b.dataset.lang = l;
        b.textContent = l.toUpperCase();
        b.title = l === 'en' ? 'English' : 'Deutsch';
        b.addEventListener('click', () => set(l));
        host.appendChild(b);
      });
    }
    host.querySelectorAll('button').forEach(b => {
      b.classList.toggle('on', b.dataset.lang === lang);
      b.setAttribute('aria-pressed', b.dataset.lang === lang);
    });
  }

  function set(l) {
    if (!LANGS.includes(l) || l === lang) return;
    lang = l;
    try { localStorage.setItem(KEY, l); } catch (e) { /* ignore */ }
    document.documentElement.lang = l;
    apply();
    dispatchEvent(new CustomEvent('langchange', { detail: l }));
  }

  function add(d) {
    for (const l in d) Object.assign(dict[l] || (dict[l] = {}), d[l]);
  }

  window.I18N = { t, add, apply, set, get lang() { return lang; } };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', () => apply());
  else apply();
})();

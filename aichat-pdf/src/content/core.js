/* ai chat exporter - by herrlamatv
 * Content-Script-Kern. Klassisches Script (Manifest-Content-Scripts können
 * keine Module sein) - alles hängt am Namensraum globalThis.AIX, die
 * Adapter-Dateien danach melden sich mit AIX.register(…) an.
 *
 * Aufgaben: Nachrichten vom Service Worker annehmen, Chat nachladen
 * (hochscrollen), Nachrichten aus dem DOM lesen und bereinigen, Bilder als
 * data:-URL holen (mit den Cookies der Seite), Zwischenablage, Hinweis-Toast.
 */

(() => {
  const AIX = (globalThis.AIX = globalThis.AIX || {});
  if (AIX.core) return; // schon da (z. B. erneut injiziert)
  AIX.core = true;
  AIX.adapters = [];
  AIX.handlers = {};

  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  AIX.sleep = sleep;

  AIX.register = (adapter) => {
    if (!AIX.adapters.some((a) => a.id === adapter.id)) AIX.adapters.push(adapter);
  };

  /** Passender Adapter für diese Seite - sonst Universal (falls injiziert). */
  AIX.adapter = () => AIX.adapters.find((a) => a.id !== 'universal' && a.match()) || AIX.adapters.find((a) => a.id === 'universal') || null;

  /* Sprache und Einstellungen (für Knöpfe, Picker, Hinweise im Chat). */
  const browserLang = (navigator.language || '').toLowerCase().startsWith('de') ? 'de' : 'en';
  AIX.settings = {};
  AIX.lang = browserLang;
  const applySettings = (s) => {
    AIX.settings = s || {};
    AIX.lang = AIX.settings.language === 'de' || AIX.settings.language === 'en' ? AIX.settings.language : browserLang;
    for (const fn of AIX.onSettings) fn(AIX.settings);
  };
  AIX.onSettings = [];
  AIX.L = (dict) => dict[AIX.lang] || dict.en;
  try {
    chrome.storage.local.get('aichat').then((d) => applySettings(d.aichat), () => {});
    chrome.storage.onChanged.addListener((changes, area) => {
      if (area === 'local' && changes.aichat) applySettings(changes.aichat.newValue);
    });
  } catch (e) {}

  AIX.cookie = (name) => {
    const m = document.cookie.match(new RegExp('(?:^|;\\s*)' + name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '=([^;]*)'));
    return m ? decodeURIComponent(m[1]) : '';
  };

  AIX.text = (node) => (node ? String(node.textContent || '').replace(/\s+/g, ' ').trim() : '');

  /** querySelectorAll als Array - ein ungültiger Selektor gibt [] statt eines Fehlers. */
  AIX.all = (selector, root = document) => {
    try {
      return Array.from(root.querySelectorAll(selector));
    } catch (e) {
      return [];
    }
  };

  AIX.first = (root, selectors) => {
    if (!root) return null;
    for (const sel of [].concat(selectors)) {
      try {
        const hit = root.querySelector(sel);
        if (hit) return hit;
      } catch (e) {}
    }
    return null;
  };

  /** Nur die äußersten Elemente behalten (keins, das in einem anderen steckt). */
  AIX.outermost = (nodes) => nodes.filter((n) => !nodes.some((o) => o !== n && o.contains(n)));

  /** Titel: Adapter, sonst <title> ohne „ - ChatGPT“ & Co. */
  AIX.pageTitle = (suffixes = []) => {
    let t = document.title || '';
    for (const s of suffixes) t = t.replace(s, '');
    return t.replace(/\s+/g, ' ').trim();
  };

  /* ------------------------------ Scrollen ------------------------------ */

  function isScrollable(el) {
    if (!el || el === document.documentElement) return false;
    const style = getComputedStyle(el);
    return /(auto|scroll|overlay)/.test(style.overflowY) && el.scrollHeight > el.clientHeight + 20;
  }

  /** Scroll-Container des Chats: Adapter, sonst vom ersten Beitrag aufwärts suchen. */
  AIX.scroller = (adapter) => {
    const own = adapter && adapter.scroller ? adapter.scroller() : null;
    if (own) return own;
    const turns = adapter ? adapter.turns() : [];
    let el = turns.length ? turns[0].node : null;
    while (el && el !== document.body) {
      if (isScrollable(el)) return el;
      el = el.parentElement;
    }
    return document.scrollingElement || document.documentElement;
  };

  /**
   * Ältere Nachrichten nachladen: nach oben scrollen, bis sich weder Anzahl
   * noch Höhe ändern. Danach dieselbe Stelle (vom Ende gemessen) wiederherstellen.
   */
  AIX.loadAll = async (adapter, { timeout = 45000 } = {}) => {
    const scroller = AIX.scroller(adapter);
    if (!scroller || scroller.scrollHeight <= scroller.clientHeight + 20) return;
    const fromBottom = scroller.scrollHeight - scroller.scrollTop;
    const t0 = Date.now();
    let lastCount = -1;
    let lastHeight = -1;
    let stable = 0;
    while (Date.now() - t0 < timeout) {
      scroller.scrollTop = 0;
      await sleep(stable ? 900 : 650);
      const count = adapter.turns().length;
      const height = scroller.scrollHeight;
      if (count === lastCount && height === lastHeight && scroller.scrollTop === 0) {
        if (++stable >= 2) break;
      } else stable = 0;
      lastCount = count;
      lastHeight = height;
    }
    scroller.scrollTop = Math.max(0, scroller.scrollHeight - fromBottom);
    await sleep(150);
  };

  /**
   * Für Seiten, die Beiträge außerhalb des Bildschirms wieder aus dem DOM
   * werfen (virtuelle Listen): Stück für Stück von oben nach unten scrollen und
   * dabei einsammeln. Doppelte erkennt man an id bzw. Rolle + Textanfang.
   */
  AIX.collectWhileScrolling = async (adapter, read) => {
    const scroller = AIX.scroller(adapter);
    const keep = scroller.scrollTop;
    const seen = new Map();
    const order = [];
    const take = () => {
      for (const turn of adapter.turns()) {
        const key = turn.id || turn.role + ':' + AIX.text(turn.node).slice(0, 300);
        if (seen.has(key)) continue;
        const msg = read(turn);
        if (!msg) continue;
        seen.set(key, msg);
        order.push(msg);
      }
    };
    scroller.scrollTop = 0;
    await sleep(500);
    for (let guard = 0; guard < 2000; guard++) {
      take();
      const before = scroller.scrollTop;
      scroller.scrollTop = before + Math.max(200, scroller.clientHeight * 0.8);
      await sleep(250);
      if (scroller.scrollTop === before) break;
    }
    take();
    scroller.scrollTop = keep;
    return order;
  };

  /* ---------------------------- Bereinigen ----------------------------- */

  /* Was nie in eine Datei gehört. KaTeX: .katex-mathml (MathML) bleibt, nur die
     Bildschirm-Version .katex-html fliegt raus - MathML kann Chrome selbst setzen. */
  const DROP = [
    'script', 'style', 'noscript', 'template', 'iframe', 'object', 'embed', 'canvas', 'video', 'audio',
    'button', 'input', 'textarea', 'select', 'form', 'svg', 'link', 'meta',
    '.sr-only', '.visually-hidden', '.cdk-visually-hidden', '[data-aix-skip]', '.katex-html', '[data-aix-host]'
  ].join(',');

  /**
   * Arbeitskopie eines Beitrags. `skip` sind Original-Knoten, die nicht mit
   * sollen (z. B. der Gedankengang, der getrennt ausgegeben wird). Kleine
   * Bilder (Favicons, Avatare) fliegen ebenfalls raus - das geht nur am
   * Original, weil die Kopie keine Größe kennt.
   */
  AIX.cleanClone = (node, adapter, skip = []) => {
    const marked = [];
    const mark = (el) => {
      if (el && !el.hasAttribute('data-aix-skip')) {
        el.setAttribute('data-aix-skip', '');
        marked.push(el);
      }
    };
    for (const el of skip) if (el && node.contains(el) && el !== node) mark(el);
    const imgs = Array.from(node.querySelectorAll('img'));
    for (const img of imgs) {
      const w = img.naturalWidth || img.width;
      const h = img.naturalHeight || img.height;
      if (w && h && w <= 40 && h <= 40) mark(img);
      /* srcset/lazy: die tatsächlich geladene Adresse kennt nur das Original. */
      else if (img.currentSrc) img.setAttribute('data-aix-src', img.currentSrc);
    }
    /* Sprache von Code-Blöcken am Original merken (Kopfzeilen stehen manchmal außerhalb von <pre>). */
    for (const pre of node.querySelectorAll('pre')) {
      const lang = (adapter && adapter.codeLang && adapter.codeLang(pre)) || codeLangOf(pre);
      if (lang) pre.setAttribute('data-aix-lang', lang);
    }
    /* Text mit echten Zeilenumbrüchen in white-space:pre-wrap (typisch: deine Prompts)
       - die Umbrüche sollen bleiben, obwohl die Kopie kein CSS mehr hat. */
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    for (let t = walker.nextNode(), i = 0; t && i < 20000; t = walker.nextNode(), i++) {
      const p = t.parentElement;
      if (!p || !t.nodeValue.includes('\n') || !t.nodeValue.trim() || p.hasAttribute('data-aix-ws') || p.closest('pre')) continue;
      if (/^(pre|pre-wrap|pre-line|break-spaces)$/.test(getComputedStyle(p).whiteSpace)) {
        p.setAttribute('data-aix-ws', '');
        marked.push(p);
      }
    }
    const clone = node.cloneNode(true);
    for (const el of marked) {
      el.removeAttribute('data-aix-skip');
      el.removeAttribute('data-aix-ws');
    }
    for (const pre of node.querySelectorAll('pre[data-aix-lang]')) pre.removeAttribute('data-aix-lang');
    for (const img of imgs) img.removeAttribute('data-aix-src');
    for (const img of clone.querySelectorAll('img[data-aix-src]')) {
      img.setAttribute('src', img.getAttribute('data-aix-src'));
      img.removeAttribute('data-aix-src');
      img.removeAttribute('srcset');
    }

    /* Erst umbauen (liest z. B. die Sprache aus Code-Kopfzeilen), dann UI-Reste weg. */
    if (adapter && adapter.prepare) adapter.prepare(clone);
    if (adapter && adapter.strip) for (const el of AIX.all(adapter.strip, clone)) el.remove();
    for (const el of AIX.all(DROP, clone)) el.remove();
    normalizeCode(clone);
    return clone;
  };

  function codeLangOf(pre) {
    const code = pre.querySelector('code');
    const cls = ((code && code.className) || '') + ' ' + (pre.className || '');
    const m = /(?:^|\s)(?:language|lang)-([\w+#.-]+)/.exec(cls);
    if (m) return m[1];
    const attr = pre.getAttribute('data-language') || (code && code.getAttribute('data-language'));
    return attr || '';
  }
  AIX.codeLangOf = codeLangOf;

  /** Text eines Code-Blocks - auch wenn jede Zeile ein eigenes <div> ist. */
  function codeText(el) {
    const blocks = Array.from(el.children).filter((c) => /^(DIV|P)$/.test(c.tagName));
    if (blocks.length > 1 && blocks.length === el.children.length) return blocks.map((b) => b.textContent).join('\n');
    return el.textContent;
  }

  /** <pre> auf <pre><code class="language-x"> zurückstutzen (Kopfzeilen, Kopier-Knöpfe raus). */
  function normalizeCode(root) {
    for (const pre of root.querySelectorAll('pre')) {
      const lang = pre.getAttribute('data-aix-lang') || codeLangOf(pre);
      const code = pre.querySelector('code');
      const fresh = document.createElement('code');
      if (code && !code.querySelector('div > span, div > div')) {
        for (const child of Array.from(code.childNodes)) fresh.append(child);
      } else {
        fresh.textContent = codeText(code || pre);
      }
      if (lang) fresh.className = 'language-' + lang;
      pre.replaceChildren(fresh);
      if (lang) pre.setAttribute('data-lang', lang);
      pre.removeAttribute('data-aix-lang');
    }
  }

  /* ----------------------------- Sanitizer ----------------------------- */

  const KEEP = new Set(
    (
      'p br h1 h2 h3 h4 h5 h6 ul ol li pre code span strong b em i u s del ins mark sub sup small kbd blockquote a img ' +
      'table thead tbody tfoot tr th td caption hr div details summary figure figcaption dl dt dd ' +
      'math semantics mrow mi mo mn ms mtext mspace msup msub msubsup mfrac msqrt mroot mover munder munderover ' +
      'mtable mtr mtd mstyle mpadded mphantom menclose mlabeledtr'
    ).split(' ')
  );
  const MATH_ATTRS = new Set(['display', 'mathvariant', 'stretchy', 'fence', 'separator', 'lspace', 'rspace', 'accent', 'accentunder', 'columnalign', 'displaystyle', 'scriptlevel', 'linethickness', 'width', 'height', 'depth', 'notation']);
  const STYLE_OK = /^(color|background-color|font-weight|font-style|text-decoration)$/;

  function safeHref(url, image) {
    const u = String(url || '').trim();
    if (/^(https?:|mailto:)/i.test(u)) return u;
    if (image && /^(data:image\/|blob:)/i.test(u)) return u;
    return '';
  }

  function cleanStyle(style) {
    const out = [];
    for (const part of String(style || '').split(';')) {
      const [prop, ...rest] = part.split(':');
      const p = (prop || '').trim().toLowerCase();
      const v = rest.join(':').trim();
      if (STYLE_OK.test(p) && v && !/url\(|expression|javascript/i.test(v)) out.push(p + ':' + v);
    }
    return out.join(';');
  }

  /* Icon-Schriften (Pfeile, Logos) liegen im Private-Use-Bereich - im PDF wären das leere Kästchen. */
  const ICON_CHARS = /[\ue000-\uf8ff]/g;
  AIX.ICON_CHARS = ICON_CHARS;

  function copyNode(src, into, ws = false) {
    ws = ws || (src.hasAttribute && src.hasAttribute('data-aix-ws'));
    for (const child of Array.from(src.childNodes)) {
      if (child.nodeType === 3) {
        const value = child.nodeValue.replace(ICON_CHARS, '');
        if (ws && value.includes('\n')) {
          value.split('\n').forEach((line, i) => {
            if (i) into.append(document.createElement('br'));
            into.append(document.createTextNode(line));
          });
        } else into.append(document.createTextNode(value));
        continue;
      }
      if (child.nodeType !== 1) continue;
      const tag = child.tagName.toLowerCase();
      if (tag === 'annotation' || tag === 'annotation-xml') continue;
      if (!KEEP.has(tag)) {
        copyNode(child, into, ws); // unbekannt: auspacken, Inhalt behalten
        continue;
      }
      const isMath = child.namespaceURI === 'http://www.w3.org/1998/Math/MathML';
      const el = isMath ? document.createElementNS('http://www.w3.org/1998/Math/MathML', tag) : document.createElement(tag);
      if (tag === 'a') {
        const href = safeHref(child.href || child.getAttribute('href'));
        if (href) el.setAttribute('href', href);
      } else if (tag === 'img') {
        const src = safeHref(child.currentSrc || child.src || child.getAttribute('src'), true);
        if (!src) continue;
        el.setAttribute('src', src);
        if (child.alt) el.setAttribute('alt', child.alt);
      } else if (tag === 'ol' && child.getAttribute('start')) {
        el.setAttribute('start', child.getAttribute('start'));
      } else if ((tag === 'td' || tag === 'th')) {
        for (const a of ['colspan', 'rowspan']) if (child.getAttribute(a)) el.setAttribute(a, child.getAttribute(a));
        const align = child.style && child.style.textAlign;
        if (align) el.setAttribute('style', 'text-align:' + align);
      } else if (tag === 'pre' && child.getAttribute('data-lang')) {
        el.setAttribute('data-lang', child.getAttribute('data-lang'));
      } else if (isMath) {
        for (const a of Array.from(child.attributes)) if (MATH_ATTRS.has(a.name)) el.setAttribute(a.name, a.value);
      }
      if (tag === 'code' || tag === 'span') {
        const cls = String(child.className && child.className.baseVal !== undefined ? child.className.baseVal : child.className || '')
          .split(/\s+/)
          .filter((c) => /^(language-|hljs|token|math)/.test(c))
          .join(' ');
        if (cls) el.setAttribute('class', cls);
        const style = cleanStyle(child.getAttribute('style'));
        if (style && tag === 'span') el.setAttribute('style', style);
      }
      if (tag === 'li' && child.querySelector(':scope > input[type=checkbox]')) el.setAttribute('class', 'task');
      if (tag === 'details') el.setAttribute('open', '');
      copyNode(child, el, ws);
      into.append(el);
    }
  }

  /** Bereinigte Kopie -> sicheres HTML (Allowlist für Tags und Attribute). */
  AIX.sanitize = (root) => {
    const box = document.createElement('div');
    /* KaTeX: MathML behalten, als Block markieren, wenn es eine Formelzeile ist. */
    for (const disp of root.querySelectorAll('.katex-display math, .math-block math, [data-display="block"] math')) disp.setAttribute('display', 'block');
    copyNode(root, box);
    return box.innerHTML.replace(/<(div|span)>\s*<\/\1>/g, '').trim();
  };

  /* ---------------------------- Auslesen -------------------------------- */

  /** Ein Beitrag -> { role, html, md, thinking, sources, attachments } */
  AIX.readTurn = (adapter, turn) => {
    const content = (adapter.content && adapter.content(turn.node, turn.role)) || turn.node;
    const thinkingNode = adapter.thinking ? adapter.thinking(turn.node, turn.role) : null;
    const sourceNodes = adapter.sourceNodes ? adapter.sourceNodes(turn.node, turn.role) : [];
    const skip = [thinkingNode].concat(sourceNodes).filter(Boolean);
    const clone = AIX.cleanClone(content, adapter, skip);
    const msg = {
      id: turn.id || null,
      ids: turn.ids || (turn.id ? [turn.id] : []),
      role: turn.role,
      name: turn.name || null,
      time: turn.time || null,
      html: AIX.sanitize(clone),
      md: AIX.html2md(clone),
      thinking: null,
      sources: adapter.sources ? adapter.sources(turn.node, turn.role) : AIX.linksIn(sourceNodes),
      attachments: adapter.attachments ? adapter.attachments(turn.node, turn.role) : []
    };
    if (thinkingNode) {
      const tc = AIX.cleanClone(thinkingNode, adapter);
      const md = AIX.html2md(tc);
      if (md.trim()) msg.thinking = { md, html: AIX.sanitize(tc) };
    }
    return msg;
  };

  /** Links (Quellen) aus Knoten einsammeln, ohne Dubletten. */
  AIX.linksIn = (nodes) => {
    const out = [];
    for (const node of [].concat(nodes || [])) {
      if (!node) continue;
      const links = node.tagName === 'A' ? [node] : Array.from(node.querySelectorAll('a[href]'));
      for (const a of links) {
        const url = a.href;
        if (!/^https?:/i.test(url) || out.some((s) => s.url === url)) continue;
        out.push({ title: AIX.text(a) || a.title || url, url });
      }
    }
    return out;
  };

  AIX.extractDom = async (adapter, { loadAll = true } = {}) => {
    if (loadAll) {
      try {
        if (adapter.virtual) return await AIX.collectWhileScrolling(adapter, (t) => AIX.readTurn(adapter, t));
        await AIX.loadAll(adapter);
      } catch (e) {}
    }
    return adapter.turns().map((t) => AIX.readTurn(adapter, t));
  };

  /* ------------------------ Raten (Universal) --------------------------- */

  const USER_RE = /(^|[^a-z])(user|human|me|you|query|prompt|request|question|outgoing|sent|self)([^a-z]|$)/i;
  const BOT_RE = /(^|[^a-z])(assistant|bot|ai|model|response|answer|reply|incoming|received|copilot|gpt|llm)([^a-z]|$)/i;
  const SKIP_TAGS = /^(SCRIPT|STYLE|NAV|ASIDE|HEADER|FOOTER|FORM|BUTTON|TEXTAREA|INPUT|SELECT|SVG|NOSCRIPT|TEMPLATE)$/;

  /** 'user' | 'assistant' | null - aus data-Attributen und Klassennamen. */
  AIX.roleHint = (el) => {
    if (!el || el.nodeType !== 1) return null;
    const direct = el.getAttribute('data-message-author-role') || el.getAttribute('data-role') || el.getAttribute('data-author') || el.getAttribute('data-sender');
    if (direct) {
      if (USER_RE.test(direct)) return 'user';
      if (BOT_RE.test(direct)) return 'assistant';
    }
    const text = [el.getAttribute('data-testid'), el.getAttribute('data-content'), el.getAttribute('data-type'), typeof el.className === 'string' ? el.className : '']
      .filter(Boolean)
      .join(' ')
      .replace(/([a-z])([A-Z])/g, '$1-$2');
    if (!/(message|msg|bubble|turn|chat|query|response|answer|reply|prompt)/i.test(text)) return null;
    const u = USER_RE.test(text);
    const b = BOT_RE.test(text);
    return u && !b ? 'user' : b && !u ? 'assistant' : null;
  };

  function visible(el) {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  }

  /** Steht die „Blase“ eher rechts? Dann ist es meistens die eigene Nachricht. */
  function alignedRight(el) {
    const box = el.getBoundingClientRect();
    let inner = el;
    for (let i = 0; i < 8 && inner.children.length === 1; i++) inner = inner.children[0];
    const r = (inner.children[0] || inner).getBoundingClientRect();
    return r.width < box.width * 0.85 && r.left + r.width / 2 > box.left + box.width * 0.55;
  }

  /**
   * Beiträge raten, wenn keine Selektoren passen:
   * 1. Elemente mit Rollen-Hinweis (data-role, Klassen wie user-message / ai-message)
   * 2. sonst der Container mit den meisten ähnlichen, textreichen Kindern -
   *    Rolle nach Ausrichtung, notfalls abwechselnd (Frage, Antwort, …).
   */
  AIX.guessTurns = (root) => {
    const picked = root || AIX.picked;
    const scope = picked || document.querySelector('main') || document.body;

    if (!picked) {
      const hinted = AIX.all('*', scope).filter((el) => !SKIP_TAGS.test(el.tagName) && AIX.roleHint(el) && (el.textContent || '').trim().length > 1);
      const outer = AIX.outermost(hinted).filter((el) => !el.closest('nav, aside, header, footer, form, [contenteditable="true"]'));
      const roles = new Set(outer.map(AIX.roleHint));
      if (outer.length >= 2 && roles.has('user') && roles.has('assistant')) {
        return outer.map((node) => ({ node, role: AIX.roleHint(node) }));
      }
    }

    let best = null;
    const total = (scope.textContent || '').length || 1;
    const candidates = picked ? [picked] : [scope].concat(AIX.all('div, section, ol, ul, article, main', scope));
    for (const el of candidates) {
      const kids = Array.from(el.children).filter((c) => !SKIP_TAGS.test(c.tagName) && (c.textContent || '').trim().length > 1 && visible(c));
      if (kids.length < 2) continue;
      if (!picked && kids.filter((c) => /^(P|LI|TR|SPAN|H\d|PRE|TABLE|BLOCKQUOTE)$/.test(c.tagName)).length > kids.length / 2) continue;
      const text = kids.reduce((n, c) => n + c.textContent.length, 0);
      if (!picked && text < 120) continue;
      const same = kids.filter((c) => c.tagName === kids[0].tagName).length / kids.length;
      const score = (text / total) * Math.sqrt(Math.min(kids.length, 60)) * same;
      if (!best || score > best.score) best = { kids, score };
    }
    if (!best) return [];
    const hints = best.kids.map((k) => AIX.roleHint(k) || Array.from(k.querySelectorAll('*')).slice(0, 300).map(AIX.roleHint).find(Boolean) || null);
    const aligned = best.kids.map(alignedRight);
    const useAlign = aligned.some(Boolean) && !aligned.every(Boolean);
    return best.kids.map((node, i) => ({
      node,
      role: hints[i] || (useAlign ? (aligned[i] ? 'user' : 'assistant') : i % 2 === 0 ? 'user' : 'assistant')
    }));
  };

  /* ----------------------- Adapter aus Selektoren ------------------------ */

  const byDocumentOrder = (a, b) => (a.node.compareDocumentPosition(b.node) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1);

  /**
   * Die meisten Seiten brauchen nur Selektoren. cfg:
   *   user / assistant:  Selektor(en) für die Beiträge - der erste, der etwas findet, gilt
   *   userBody / assistantBody: Inhalt im Beitrag (optional)
   *   thinking, sources: Selektor(en) im Beitrag (optional)
   *   strip, scroller, title, codeLang, prepare, virtual, match
   * Findet keiner etwas, wird geraten (AIX.guessTurns).
   */
  AIX.domAdapter = (cfg) => {
    const pick = (sels) => {
      for (const sel of [].concat(sels || [])) {
        const found = AIX.outermost(AIX.all(sel));
        if (found.length) return found;
      }
      return [];
    };
    return {
      id: cfg.id,
      name: cfg.name,
      match: cfg.match,
      virtual: !!cfg.virtual,
      strip: cfg.strip,
      prepare: cfg.prepare,
      codeLang: cfg.codeLang,
      attachments: cfg.attachments,
      chatId: cfg.chatId,
      api: cfg.api,
      bulk: cfg.bulk,
      turns() {
        if (cfg.turns) {
          const own = cfg.turns();
          if (own && own.length) return own;
        }
        const users = pick(cfg.user).map((node) => ({ node, role: 'user' }));
        const bots = pick(cfg.assistant).map((node) => ({ node, role: 'assistant' }));
        /* Verschachtelt (Antwort steckt im selben Block wie die Frage)? Dann gilt das Innere. */
        const all = users.concat(bots).filter((t, i, arr) => !arr.some((o) => o !== t && t.node.contains(o.node)));
        if (users.length && bots.length) return all.sort(byDocumentOrder);
        /* Nur eine Seite gefunden - wahrscheinlich hat sich das DOM geändert: lieber raten. */
        const guessed = AIX.guessTurns();
        return guessed.length ? guessed : all.sort(byDocumentOrder);
      },
      content(node, role) {
        const sel = role === 'user' ? cfg.userBody : cfg.assistantBody;
        return (sel && AIX.first(node, sel)) || node;
      },
      thinking: cfg.thinking ? (node, role) => (role === 'assistant' ? AIX.first(node, cfg.thinking) : null) : null,
      sourceNodes: cfg.sources ? (node, role) => (role === 'assistant' ? [].concat(cfg.sources).flatMap((s) => AIX.all(s, node)) : []) : null,
      scroller: cfg.scroller ? () => AIX.first(document, cfg.scroller) : null,
      title: () => (cfg.title && cfg.title()) || AIX.pageTitle(cfg.titleSuffix ? [cfg.titleSuffix] : [])
    };
  };

  /* --------------------------- Handler --------------------------------- */

  AIX.handlers['aix:ping'] = async () => {
    const a = AIX.adapter();
    return { platform: a ? a.id : null };
  };

  AIX.handlers['aix:extract'] = async ({ loadAll = true, api = true }) => {
    const adapter = AIX.adapter();
    if (!adapter) throw new Error('no-adapter');
    const raw = {
      platform: adapter.id,
      url: location.href,
      title: (adapter.title && adapter.title()) || AIX.pageTitle(),
      id: adapter.chatId ? adapter.chatId() : '',
      model: adapter.model ? adapter.model() : '',
      dom: [],
      api: null
    };
    if (api && adapter.api) {
      try {
        raw.api = await adapter.api();
      } catch (e) {
        raw.apiError = String((e && e.message) || e);
      }
    }
    /* Mit API-Daten reicht das, was gerade im DOM steht (nur für schönes HTML). */
    raw.dom = await AIX.extractDom(adapter, { loadAll: loadAll && !raw.api });
    return raw;
  };

  /** Bilder als data:-URL - mit den Cookies dieser Seite, damit auch geschützte gehen. */
  AIX.handlers['aix:images'] = async ({ urls = [], max = 60, maxBytes = 12 * 1024 * 1024 }) => {
    const map = {};
    const queue = urls.slice(0, max);
    const worker = async () => {
      while (queue.length) {
        const url = queue.shift();
        try {
          const res = await fetch(url, { credentials: 'include' });
          if (!res.ok) continue;
          const blob = await res.blob();
          if (!/^image\//.test(blob.type) || blob.size > maxBytes) continue;
          map[url] = await new Promise((resolve, reject) => {
            const fr = new FileReader();
            fr.onload = () => resolve(fr.result);
            fr.onerror = reject;
            fr.readAsDataURL(blob);
          });
        } catch (e) {}
      }
    };
    await Promise.all([worker(), worker(), worker(), worker()]);
    return { map };
  };

  AIX.copy = async (text) => {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (e) {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.cssText = 'position:fixed;top:-1000px;opacity:0';
      document.body.append(ta);
      ta.select();
      const ok = document.execCommand('copy');
      ta.remove();
      return ok;
    }
  };

  AIX.handlers['aix:copy'] = async ({ text }) => ({ copied: await AIX.copy(text) });

  /* Kleiner Hinweis unten rechts - für Schnell-Export, Kontextmenü, Buttons. */
  AIX.toast = (text, kind = 'ok') => {
    let host = document.getElementById('aix-toast-host');
    if (!host) {
      host = document.createElement('div');
      host.id = 'aix-toast-host';
      host.setAttribute('data-aix-host', '');
      host.style.cssText = 'position:fixed;right:18px;bottom:18px;z-index:2147483647;pointer-events:none';
      host.attachShadow({ mode: 'open' }).innerHTML =
        '<style>.t{font:500 13px/1.4 -apple-system,"Segoe UI",system-ui,sans-serif;background:#fffdf8;color:#1f1d18;border:1px solid #e7dfd0;' +
        'border-radius:12px;padding:9px 14px;box-shadow:0 10px 26px rgba(74,62,40,.14);margin-top:8px;max-width:320px;transition:opacity .3s}' +
        '.t.err{border-color:#f4cfd1;color:#b4282d}.t b{color:#b98b52}</style><div id="box"></div>';
      document.documentElement.append(host);
    }
    const box = host.shadowRoot.getElementById('box');
    const t = document.createElement('div');
    t.className = 't' + (kind === 'error' ? ' err' : '');
    const b = document.createElement('b');
    b.textContent = 'AI Chat Export · ';
    t.append(b, document.createTextNode(text));
    box.append(t);
    setTimeout(() => {
      t.style.opacity = '0';
      setTimeout(() => t.remove(), 400);
    }, kind === 'busy' ? 2200 : 3200);
  };

  AIX.handlers['aix:toast'] = async ({ text, kind }) => {
    AIX.toast(text, kind);
    return {};
  };

  AIX.handlers['aix:bulk:list'] = async ({ offset = 0 }) => {
    const a = AIX.adapter();
    if (!a || !a.bulk) throw new Error('bulk-unsupported');
    return a.bulk.list(offset);
  };

  AIX.handlers['aix:bulk:get'] = async ({ id }) => {
    const a = AIX.adapter();
    if (!a || !a.bulk) throw new Error('bulk-unsupported');
    return a.bulk.get(id);
  };

  chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
    if (!msg || typeof msg.type !== 'string' || !AIX.handlers[msg.type]) return false;
    Promise.resolve()
      .then(() => AIX.handlers[msg.type](msg))
      .then(
        (data) => sendResponse({ ok: true, data }),
        (err) => sendResponse({ ok: false, error: String((err && err.message) || err) })
      );
    return true;
  });

  /** Fehler mit HTTP-Status - der Bulk-Export wartet bei 429 und versucht es erneut. */
  AIX.fetchJson = async (url, init = {}) => {
    const res = await fetch(url, Object.assign({ credentials: 'include' }, init));
    if (!res.ok) throw new Error('http-' + res.status);
    return res.json();
  };
})();

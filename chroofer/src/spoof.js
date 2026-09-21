/* Chroofer - by herrlamatv
 *
 * Diese Funktion wird per chrome.scripting.executeScript in die MAIN-World der
 * Seite injiziert. Sie wird dabei zu Text serialisiert - sie darf deshalb NICHTS
 * außerhalb ihres eigenen Rumpfs referenzieren (keine Imports, keine Konstanten
 * aus diesem Modul). Alles kommt über das Argument `data`.
 */

export function chrooferSpoof(data) {
  'use strict';

  const STATE_KEY = Symbol.for('chroofer.state');

  function apply(win) {
    let nav;
    try {
      nav = win.navigator;
    } catch (e) {
      return; // fremder Origin
    }
    if (!nav) return;

    /* --- Zustand pro Fenster: Function.prototype.toString nur einmal patchen -- */
    let state;
    try {
      state = win[STATE_KEY];
    } catch (e) {
      return;
    }

    if (!state) {
      const map = new WeakMap();
      try {
        const orig = win.Function.prototype.toString;
        const patched = new win.Proxy(orig, {
          apply(target, thisArg, args) {
            const name = map.get(thisArg);
            if (typeof name === 'string') return 'function ' + name + '() { [native code] }';
            return win.Reflect.apply(target, thisArg, args);
          }
        });
        map.set(patched, 'toString');
        win.Function.prototype.toString = patched;
      } catch (e) {
        /* Seite blockiert das - Spoofing läuft trotzdem, nur weniger getarnt. */
      }
      state = { map, stamp: null };
      try {
        Object.defineProperty(win, STATE_KEY, { value: state, configurable: true, writable: true, enumerable: false });
      } catch (e) {
        return;
      }
    }

    if (state.stamp === data.stamp) return; // schon mit genau dieser Konfiguration gepatcht
    state.stamp = data.stamp;

    const mark = (fn, name) => {
      try {
        state.map.set(fn, name);
      } catch (e) {}
      return fn;
    };

    const defineGetter = (target, prop, getter) => {
      if (!target) return;
      mark(getter, 'get ' + prop);
      try {
        Object.defineProperty(target, prop, { get: getter, configurable: true, enumerable: true });
      } catch (e) {}
    };

    const defineValue = (target, prop, val) => {
      defineGetter(target, prop, function () {
        return val;
      });
    };

    const drop = (target, prop) => {
      if (!target) return;
      try {
        delete target[prop];
      } catch (e) {}
      try {
        if (prop in target) Object.defineProperty(target, prop, { get: mark(function () { return undefined; }, 'get ' + prop), configurable: true });
      } catch (e) {}
    };

    const NavProto = (win.Navigator && win.Navigator.prototype) || nav;
    const opts = data.opts || {};

    /* ---------------------------- navigator.* ---------------------------- */
    if (opts.navigator !== false) {
      defineValue(NavProto, 'userAgent', data.ua);
      defineValue(NavProto, 'appVersion', data.ua.replace(/^Mozilla\//, ''));
      defineValue(NavProto, 'platform', data.navPlatform);
      defineValue(NavProto, 'vendor', data.vendor);
      defineValue(NavProto, 'vendorSub', '');
      defineValue(NavProto, 'product', 'Gecko');
      defineValue(NavProto, 'productSub', data.engine === 'gecko' ? '20100101' : '20030107');
      defineValue(NavProto, 'maxTouchPoints', data.maxTouchPoints | 0);

      if (data.engine === 'gecko') {
        defineValue(NavProto, 'oscpu', data.oscpu || '');
        defineValue(NavProto, 'buildID', '20181001000000');
      } else {
        drop(NavProto, 'oscpu');
        drop(NavProto, 'buildID');
      }
    }

    if (opts.hideWebdriver !== false) {
      defineValue(NavProto, 'webdriver', false);
    }

    /* ------------------------ navigator.userAgentData -------------------- */
    const ch = data.ch || {};
    if (opts.navigator !== false) {
      if (ch.supported && Array.isArray(ch.brands) && ch.brands.length) {
        const brands = ch.brands.map((b) => ({ brand: b.brand, version: b.version }));
        const fullList = (ch.fullVersionList || brands).map((b) => ({ brand: b.brand, version: b.version }));
        const copy = (list) => list.map((b) => ({ brand: b.brand, version: b.version }));

        const proto = win.NavigatorUAData && win.NavigatorUAData.prototype;
        const uad = proto ? Object.create(proto) : {};

        defineGetter(uad, 'brands', function () { return Object.freeze(copy(brands)); });
        defineGetter(uad, 'mobile', function () { return !!ch.mobile; });
        defineGetter(uad, 'platform', function () { return ch.platform; });

        const high = {
          architecture: ch.architecture || '',
          bitness: ch.bitness || '',
          formFactors: ch.mobile ? ['Mobile'] : ['Desktop'],
          model: ch.model || '',
          platformVersion: ch.platformVersion || '',
          uaFullVersion: ch.uaFullVersion || '',
          wow64: false
        };

        const getHighEntropyValues = mark(function getHighEntropyValues(hints) {
          try {
            const out = { brands: copy(brands), mobile: !!ch.mobile, platform: ch.platform };
            const wanted = Array.isArray(hints) ? hints : [];
            for (const h of wanted) {
              if (h === 'fullVersionList') out.fullVersionList = copy(fullList);
              else if (Object.prototype.hasOwnProperty.call(high, h)) out[h] = high[h];
            }
            return win.Promise.resolve(out);
          } catch (e) {
            return win.Promise.reject(e);
          }
        }, 'getHighEntropyValues');

        const toJSON = mark(function toJSON() {
          return { brands: copy(brands), mobile: !!ch.mobile, platform: ch.platform };
        }, 'toJSON');

        try {
          Object.defineProperty(uad, 'getHighEntropyValues', { value: getHighEntropyValues, configurable: true, writable: true, enumerable: false });
          Object.defineProperty(uad, 'toJSON', { value: toJSON, configurable: true, writable: true, enumerable: false });
        } catch (e) {}

        defineValue(NavProto, 'userAgentData', uad);
      } else {
        /* Firefox und Safari kennen userAgentData nicht. */
        drop(NavProto, 'userAgentData');
        try {
          delete win.NavigatorUAData;
        } catch (e) {}
      }
    }

    /* ------------------------------ Sprachen ----------------------------- */
    if (opts.languages) {
      const langs = String(opts.languagesValue || '')
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
      if (langs.length) {
        defineValue(NavProto, 'language', langs[0]);
        defineGetter(NavProto, 'languages', function () { return Object.freeze(langs.slice()); });
      }
    }

    /* ------------------------------ Hardware ----------------------------- */
    if (opts.hardware) {
      const hw = opts.hardwareValue || {};
      if (hw.cores) defineValue(NavProto, 'hardwareConcurrency', hw.cores | 0);
      if (data.engine === 'blink') {
        if (hw.memory) defineValue(NavProto, 'deviceMemory', Number(hw.memory));
      } else {
        drop(NavProto, 'deviceMemory');
      }
    } else if (data.engine !== 'blink') {
      drop(NavProto, 'deviceMemory');
    }

    /* ------------------------------- Screen ------------------------------ */
    if (opts.screen) {
      const s = opts.screenValue || {};
      const w = s.width | 0;
      const h = s.height | 0;
      if (w > 0 && h > 0) {
        const ScreenProto = (win.Screen && win.Screen.prototype) || win.screen;
        defineValue(ScreenProto, 'width', w);
        defineValue(ScreenProto, 'height', h);
        defineValue(ScreenProto, 'availWidth', w);
        defineValue(ScreenProto, 'availHeight', data.mobile ? h : Math.max(0, h - 40));
        defineValue(ScreenProto, 'colorDepth', s.colorDepth | 0 || 24);
        defineValue(ScreenProto, 'pixelDepth', s.colorDepth | 0 || 24);
        try {
          Object.defineProperty(win, 'devicePixelRatio', {
            get: mark(function () { return Number(s.dpr) || 1; }, 'get devicePixelRatio'),
            configurable: true
          });
        } catch (e) {}
      }
    }

    /* ------------------------------ Zeitzone ----------------------------- */
    if (opts.timezone && opts.timezoneValue) {
      const tz = String(opts.timezoneValue);
      try {
        /* Wirft, wenn die Zone unbekannt ist - dann lassen wir die Zeit in Ruhe. */
        new win.Intl.DateTimeFormat('en-US', { timeZone: tz }).format(new win.Date());

        const OrigDTF = win.Intl.DateTimeFormat;
        const withZone = (args) => {
          const a = Array.prototype.slice.call(args);
          a[1] = Object.assign({ timeZone: tz }, a[1] || {});
          return a;
        };
        const PatchedDTF = new win.Proxy(OrigDTF, {
          construct(target, args, newTarget) {
            return win.Reflect.construct(target, withZone(args), newTarget);
          },
          apply(target, thisArg, args) {
            return win.Reflect.apply(target, thisArg, withZone(args));
          }
        });
        mark(PatchedDTF, 'DateTimeFormat');
        win.Intl.DateTimeFormat = PatchedDTF;

        const offsetAt = (date) => {
          const parts = new OrigDTF('en-US', {
            timeZone: tz,
            hourCycle: 'h23',
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
          }).formatToParts(date);
          const o = {};
          for (const p of parts) o[p.type] = p.value;
          const asUTC = win.Date.UTC(+o.year, +o.month - 1, +o.day, +o.hour, +o.minute, +o.second);
          return Math.round((date.getTime() - asUTC) / 60000);
        };

        const getTimezoneOffset = mark(function getTimezoneOffset() {
          try {
            return offsetAt(this instanceof win.Date ? this : new win.Date());
          } catch (e) {
            return 0;
          }
        }, 'getTimezoneOffset');
        win.Date.prototype.getTimezoneOffset = getTimezoneOffset;
      } catch (e) {}
    }

    /* -------------------------- Engine-Spuren ---------------------------- */
    if (opts.hideEngineTraces && data.engine !== 'blink') {
      /* drop() fällt auf einen Getter zurück, wenn delete nicht erlaubt ist. */
      drop(win, 'chrome');
    }
  }

  /* --------------------- Anwenden: Hauptfenster + iframes ---------------- */
  apply(window);

  const patchFrame = (el) => {
    try {
      const w = el.contentWindow;
      if (w && w !== window) apply(w);
    } catch (e) {}
  };

  const patchAll = (root) => {
    try {
      if (!root || !root.querySelectorAll) return;
      const frames = root.querySelectorAll('iframe,frame');
      for (const el of frames) patchFrame(el);
    } catch (e) {}
  };

  try {
    patchAll(document);
    const observer = new MutationObserver((records) => {
      for (const rec of records) {
        for (const node of rec.addedNodes) {
          if (!node || node.nodeType !== 1) continue;
          const tag = node.tagName;
          if (tag === 'IFRAME' || tag === 'FRAME') {
            patchFrame(node);
            try {
              node.addEventListener('load', () => patchFrame(node));
            } catch (e) {}
          } else {
            patchAll(node);
          }
        }
      }
    });
    const start = () => {
      try {
        observer.observe(document.documentElement || document, { childList: true, subtree: true });
      } catch (e) {}
    };
    if (document.documentElement) start();
    else document.addEventListener('readystatechange', start, { once: true });
  } catch (e) {}
}

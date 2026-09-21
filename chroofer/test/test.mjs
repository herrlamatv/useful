/* Chroofer - by herrlamatv
 * Testet Profil-Bau, Host-Auflösung, Netzwerk-Regeln und die injizierte
 * Spoof-Funktion (in einem eigenen VM-Realm). Ohne Abhängigkeiten:
 *
 *   node test/test.mjs
 */

import vm from 'node:vm';
import { buildUserAgent, buildClientHints, brandsToHeader, resolveProfile, profileLabel } from '../src/profiles.js';
import {
  mergeSettings,
  normalizeHost,
  findSiteEntry,
  resolveForHost,
  randomProfileFor,
  newSite
} from '../src/settings.js';
import { buildRules, tabRuleFor, headerActionsFor, TAB_RULE_BASE } from '../src/rules.js';
import { chrooferSpoof } from '../src/spoof.js';
import enMessages from '../ui/i18n/en.js';
import deMessages from '../ui/i18n/de.js';
import { readFileSync, readdirSync } from 'node:fs';

let fails = 0;
const ok = (name, cond, extra) => {
  if (cond) console.log('  ok   ' + name);
  else {
    fails++;
    console.log('  FAIL ' + name + (extra ? '  -> ' + extra : ''));
  }
};
const head = (title) => console.log('\n== ' + title + ' ==');

const P = {
  win11Chrome: { os: 'windows', osVersion: '11', browser: 'chrome', browserVersion: '142' },
  win10Edge: { os: 'windows', osVersion: '10', browser: 'edge', browserVersion: '140' },
  win11Firefox: { os: 'windows', osVersion: '11', browser: 'firefox', browserVersion: '145' },
  macSafari: { os: 'macos', osVersion: '15', browser: 'safari', browserVersion: '18.5' },
  macChrome: { os: 'macos', osVersion: '14', browser: 'chrome', browserVersion: '141' },
  ubuntuFirefox: { os: 'linux', osVersion: 'ubuntu', browser: 'firefox', browserVersion: '144' },
  androidChrome: { os: 'android', osVersion: '15', browser: 'chrome', browserVersion: '142' },
  iosSafari: { os: 'ios', osVersion: '18', browser: 'safari', browserVersion: '18.5' },
  iosChrome: { os: 'ios', osVersion: '18', browser: 'chrome', browserVersion: '142' },
  chromeos: { os: 'chromeos', osVersion: '16', browser: 'chrome', browserVersion: '140' },
  opera: { os: 'windows', osVersion: '11', browser: 'opera', browserVersion: '126' }
};
const all = Object.values(P);

/* -------------------------------------------------------------------- */
head('User-Agent-Strings');
for (const p of all) console.log('  ' + profileLabel(p).padEnd(34) + buildUserAgent(p));

head('Plausibilität');
for (const p of all) {
  const ua = buildUserAgent(p);
  ok('sauberer UA: ' + profileLabel(p), ua.startsWith('Mozilla/5.0 (') && !/undefined|NaN|\{|\}/.test(ua), ua);
}
ok('Edge hängt Edg/ an', buildUserAgent(P.win10Edge).includes(' Edg/140.0.0.0'));
ok('Firefox hat kein AppleWebKit', !buildUserAgent(P.win11Firefox).includes('AppleWebKit'));
ok('Safari hat kein Chrome/', !buildUserAgent(P.macSafari).includes('Chrome/'));
ok('Android-Chrome ist Mobile', buildUserAgent(P.androidChrome).includes('Mobile Safari/537.36'));
ok('Chrome auf iOS nutzt CriOS', buildUserAgent(P.iosChrome).includes('CriOS/'));
ok(
  'Opera 126 -> Chromium 142',
  buildUserAgent(P.opera).includes('Chrome/142.0.0.0') && buildUserAgent(P.opera).includes('OPR/126')
);
ok('eigener User-Agent schlägt alles', buildUserAgent({ ...P.win11Chrome, customUA: '  MeinAgent/1.0  ' }) === 'MeinAgent/1.0');

/* -------------------------------------------------------------------- */
head('Client Hints');
const chWin = buildClientHints(P.win11Chrome);
console.log('  sec-ch-ua:', brandsToHeader(chWin.brands));
ok('Windows 11 -> Platform-Version 15.0.0', chWin.platformVersion === '15.0.0');
ok('Firefox sendet keine CH', buildClientHints(P.win11Firefox).supported === false);
ok('Safari sendet keine CH', buildClientHints(P.macSafari).supported === false);
ok('Chrome auf iOS sendet keine CH', buildClientHints(P.iosChrome).supported === false);
ok('Android mobile=?1', buildClientHints(P.androidChrome).mobile === true);
ok('Android Model gesetzt', buildClientHints(P.androidChrome).model === 'K');
ok('Desktop ohne Model', chWin.model === '');

/* -------------------------------------------------------------------- */
head('Header-Aktionen');
const opts = mergeSettings({}).options;
const aWin = headerActionsFor(P.win11Chrome, opts);
const aFf = headerActionsFor(P.win11Firefox, opts);
ok('UA wird gesetzt', aWin.some((a) => a.header === 'user-agent' && a.operation === 'set'));
ok('sec-ch-ua-platform = "Windows"', aWin.some((a) => a.header === 'sec-ch-ua-platform' && a.value === '"Windows"'));
ok('High-Entropy standardmäßig entfernt', aWin.some((a) => a.header === 'sec-ch-ua-platform-version' && a.operation === 'remove'));
ok('Firefox: alle CH entfernt', aFf.filter((a) => a.header.startsWith('sec-ch-ua')).every((a) => a.operation === 'remove'));
ok(
  'High-Entropy an -> gesetzt',
  headerActionsFor(P.win11Chrome, { ...opts, highEntropyHeaders: true }).some(
    (a) => a.header === 'sec-ch-ua-platform-version' && a.value === '"15.0.0"'
  )
);
const lang = headerActionsFor(P.win11Chrome, { ...opts, languages: true, languagesValue: 'de-DE,de,en' }).find(
  (a) => a.header === 'accept-language'
);
ok('accept-language mit q-Werten', lang && lang.value === 'de-DE,de;q=0.9,en;q=0.8', lang && lang.value);

/* -------------------------------------------------------------------- */
head('Host erkennen');
ok('URL -> Host', normalizeHost('https://WWW.Example.com:8443/pfad?x=1') === 'www.example.com');
ok('*.host -> host', normalizeHost('*.example.com') === 'example.com');
ok('Müll -> leer', normalizeHost('nicht gültig!') === '');
ok('Wort ohne Punkt -> leer', normalizeHost('einfachnurtext') === '');
ok('localhost erlaubt', normalizeHost('http://localhost:3000/x') === 'localhost');

/* -------------------------------------------------------------------- */
head('Standardzustand (frische Installation)');
const fresh = mergeSettings({});
ok('Chroofer selbst ist an', fresh.enabled === true);
ok('global steht auf "nicht spoofen"', fresh.global.mode === 'off', fresh.global.mode);
ok('keine Website-Regeln', fresh.sites.length === 0);
ok('nichts wird gespooft', resolveForHost(fresh, 'irgendeine-seite.de').active === false);
ok('Quelle ist global-off', resolveForHost(fresh, 'irgendeine-seite.de').source === 'global-off');
ok('keine Netzwerk-Regeln', buildRules(fresh, fresh.global.profile, {}).length === 0);
ok(
  'eine Website-Regel greift trotzdem sofort',
  resolveForHost(mergeSettings({ sites: [{ pattern: 'nur-hier.de', profile: P.macSafari }] }), 'nur-hier.de').active === true
);
ok('kaputter Modus fällt auf aus zurück', mergeSettings({ global: { mode: 'quatsch' } }).global.mode === 'off');

/* -------------------------------------------------------------------- */
head('Regel-Auflösung pro Website');
const st = mergeSettings({
  global: { mode: 'spoof', profile: P.win11Chrome },
  sites: [
    { pattern: 'example.com', mode: 'spoof', profile: P.macSafari },
    { pattern: 'shop.example.com', mode: 'off' },
    { pattern: 'exakt.de', mode: 'spoof', profile: P.win10Edge, subdomains: false },
    { pattern: 'pausiert.de', mode: 'spoof', profile: P.macChrome, enabled: false },
    { pattern: 'weiter.de', mode: 'spoof', profile: P.ubuntuFirefox, redirects: true },
    { pattern: 'random.test', mode: 'random' }
  ]
});

ok('neue Regel hat sinnvolle Vorgaben', (() => {
  const s = newSite('neu.de', P.win11Chrome);
  return s.enabled && s.subdomains && !s.redirects && s.thirdParty && s.mode === 'spoof';
})());

ok('exakter Treffer', findSiteEntry(st, 'example.com').pattern === 'example.com');
ok('Subdomain erbt', findSiteEntry(st, 'www.example.com').pattern === 'example.com');
ok('spezifischer schlägt allgemein', findSiteEntry(st, 'shop.example.com').pattern === 'shop.example.com');
ok('Subdomain der spezifischen Regel', findSiteEntry(st, 'a.shop.example.com').pattern === 'shop.example.com');
ok('fremde Domain -> global', findSiteEntry(st, 'anders.de') === null);
ok('ohne Subdomains: Host trifft', findSiteEntry(st, 'exakt.de').pattern === 'exakt.de');
ok('ohne Subdomains: Subdomain trifft nicht', findSiteEntry(st, 'sub.exakt.de') === null);
ok('pausierte Regel zählt nicht', findSiteEntry(st, 'pausiert.de') === null);
ok('pausiert -> globales Profil', resolveForHost(st, 'pausiert.de').source === 'global');
ok('shop ist aus', resolveForHost(st, 'shop.example.com').active === false);
ok('example.com spooft Safari', resolveForHost(st, 'example.com').profile.browser === 'safari');
ok('Master-Aus schlägt alles', resolveForHost({ ...st, enabled: false }, 'example.com').active === false);
ok('Geltungsbereich wird mitgeliefert', resolveForHost(st, 'weiter.de').scope.redirects === true);
ok('global ohne Weiterleitungs-Vererbung', resolveForHost(st, 'anders.de').scope.redirects === false);
ok('Zufall pro Domain stabil', randomProfileFor('random.test').browser === randomProfileFor('random.test').browser);
ok(
  'Zufall nutzt das Muster, nicht den Host',
  JSON.stringify(resolveForHost(st, 'sub.random.test').profile) === JSON.stringify(randomProfileFor('random.test'))
);

/* -------------------------------------------------------------------- */
head('Domain-Regeln (declarativeNetRequest)');
const rules = buildRules(st, P.win11Chrome, { 'random.test': randomProfileFor('random.test') });
const ids = rules.map((r) => r.id);
ok('IDs eindeutig', new Set(ids).size === ids.length);
ok('IDs unter dem Tab-Bereich', ids.every((i) => Number.isInteger(i) && i >= 1 && i < TAB_RULE_BASE));
ok('Prioritäten >= 1', rules.every((r) => Number.isInteger(r.priority) && r.priority >= 1));

const globalRule = rules.find((r) => r.condition.urlFilter === '*');
ok('genau eine globale Regel', rules.filter((r) => r.condition.urlFilter === '*').length === 1);
ok('globale Regel hat die kleinste Priorität', rules.every((r) => r === globalRule || r.priority > globalRule.priority));

const navRules = rules.filter((r) => r.condition.resourceTypes);
ok(
  'Domain-Regeln greifen nur bei Navigationen',
  navRules.every((r) => JSON.stringify(r.condition.resourceTypes) === JSON.stringify(['main_frame', 'sub_frame']))
);

const exact = rules.find((r) => r.condition.regexFilter);
ok('ohne Subdomains -> exakter Regex', !!exact && exact.condition.regexFilter === '^https?://exakt\\.de(?::\\d+)?/', exact && exact.condition.regexFilter);
ok('ohne Subdomains keine initiator-Regel', !rules.some((r) => r.condition.initiatorDomains && r.condition.initiatorDomains[0] === 'exakt.de'));
ok('pausierte Regel erzeugt keine Regeln', !rules.some((r) => JSON.stringify(r.condition).includes('pausiert.de')));

const offRules = rules.filter((r) => r.action.type === 'allow');
ok('Aus-Regel als allow', offRules.length === 2 && offRules.every((r) => r.priority > 100));
const shopPrio = offRules[0].priority;
const examplePrio = rules.find(
  (r) => r.action.type === 'modifyHeaders' && r.condition.requestDomains && r.condition.requestDomains[0] === 'example.com'
).priority;
ok('shop.example.com sticht example.com', shopPrio > examplePrio, shopPrio + ' vs ' + examplePrio);
ok('Header aus -> keine Regeln', buildRules({ ...st, options: { ...st.options, headers: false } }, P.win11Chrome, {}).length === 0);
ok('Master aus -> keine Regeln', buildRules({ ...st, enabled: false }, P.win11Chrome, {}).length === 0);
console.log('  Domain-Regeln gesamt:', rules.length);

head('Tab-Regeln');
const tabRule = tabRuleFor(7, { active: true, scope: { thirdParty: true } }, P.macSafari, opts);
ok('ID aus dem Tab-Bereich', tabRule.id === TAB_RULE_BASE + 7);
ok('gilt nur für diesen Tab', JSON.stringify(tabRule.condition.tabIds) === '[7]');
ok('Navigation bleibt den Domain-Regeln', JSON.stringify(tabRule.condition.excludedResourceTypes) === '["main_frame"]');
ok('sticht jede Domain-Regel', rules.every((r) => tabRule.priority > r.priority));
ok('setzt den Safari-UA', tabRule.action.requestHeaders.some((a) => a.value === buildUserAgent(P.macSafari)));
const firstPartyRule = tabRuleFor(7, { active: true, scope: { thirdParty: false } }, P.macSafari, opts);
ok('ohne Drittanbieter -> firstParty', firstPartyRule.condition.domainType === 'firstParty');
ok('mit Drittanbietern kein domainType', tabRule.condition.domainType === undefined);
const offTabRule = tabRuleFor(7, { active: false, scope: { thirdParty: true } }, null, opts);
ok('inaktiver Tab -> allow', offTabRule.action.type === 'allow');
ok('Header aus -> keine Tab-Regel', tabRuleFor(7, { active: true, scope: {} }, P.macSafari, { ...opts, headers: false }) === null);

/* -------------------------------------------------------------------- */
/* Die injizierte Funktion in einem eigenen Realm                        */
/* -------------------------------------------------------------------- */
head('Injizierte Spoof-Funktion');

function makeRealm() {
  const ctx = vm.createContext({});
  vm.runInContext(
    `
    var __realUA = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/200.0.0.0 Safari/537.36';
    function Navigator() {}
    Object.defineProperties(Navigator.prototype, {
      userAgent: { get() { return __realUA; }, configurable: true, enumerable: true },
      platform: { get() { return 'Linux x86_64'; }, configurable: true, enumerable: true },
      vendor: { get() { return 'Google Inc.'; }, configurable: true, enumerable: true },
      webdriver: { get() { return true; }, configurable: true, enumerable: true },
      maxTouchPoints: { get() { return 0; }, configurable: true, enumerable: true },
      hardwareConcurrency: { get() { return 24; }, configurable: true, enumerable: true },
      deviceMemory: { get() { return 32; }, configurable: true, enumerable: true },
      language: { get() { return 'en-US'; }, configurable: true, enumerable: true },
      languages: { get() { return ['en-US']; }, configurable: true, enumerable: true },
      userAgentData: { get() { return { brands: [{ brand: 'Echt', version: '200' }], mobile: false, platform: 'Linux' }; }, configurable: true, enumerable: true }
    });
    function NavigatorUAData() {}
    function Screen() {}
    Object.defineProperties(Screen.prototype, {
      width: { get() { return 2560; }, configurable: true, enumerable: true },
      height: { get() { return 1440; }, configurable: true, enumerable: true },
      availWidth: { get() { return 2560; }, configurable: true, enumerable: true },
      availHeight: { get() { return 1400; }, configurable: true, enumerable: true },
      colorDepth: { get() { return 30; }, configurable: true, enumerable: true },
      pixelDepth: { get() { return 30; }, configurable: true, enumerable: true }
    });
    var navigator = Object.create(Navigator.prototype);
    var screen = Object.create(Screen.prototype);
    var devicePixelRatio = 2;
    Object.defineProperty(globalThis, 'chrome', { value: { runtime: {} }, configurable: true, writable: true });
    var document = { documentElement: {}, querySelectorAll() { return []; }, addEventListener() {} };
    function MutationObserver() { this.observe = function () {}; }
    globalThis.window = globalThis;
    `,
    ctx
  );
  return ctx;
}

const runSpoof = (ctx, payload) => vm.runInContext('(' + chrooferSpoof.toString() + ')', ctx)(payload);

function payloadFor(profile, o) {
  const r = resolveProfile(profile);
  return {
    stamp: JSON.stringify([r.ua, r.ch, o]),
    ua: r.ua,
    ch: r.ch,
    engine: r.engine,
    navPlatform: r.navPlatform,
    oscpu: r.oscpu,
    vendor: r.vendor,
    maxTouchPoints: r.maxTouchPoints,
    mobile: r.mobile,
    opts: o
  };
}

const baseOpts = {
  navigator: true,
  hideWebdriver: true,
  languages: false,
  languagesValue: '',
  timezone: false,
  timezoneValue: '',
  screen: false,
  screenValue: {},
  hardware: false,
  hardwareValue: {},
  hideEngineTraces: false
};

let ctx = makeRealm();
runSpoof(ctx, payloadFor(P.win11Chrome, baseOpts));
const g = (expr) => vm.runInContext(expr, ctx);
ok('userAgent ersetzt', g('navigator.userAgent') === buildUserAgent(P.win11Chrome), g('navigator.userAgent'));
ok('platform = Win32', g('navigator.platform') === 'Win32');
ok('appVersion passt zum UA', g('navigator.appVersion') === buildUserAgent(P.win11Chrome).replace('Mozilla/', ''));
ok('webdriver = false', g('navigator.webdriver') === false);
ok('uaData.platform = Windows', g('navigator.userAgentData.platform') === 'Windows');
ok('uaData.brands gefälscht', !JSON.stringify(g('navigator.userAgentData.brands')).includes('Echt'));
ok(
  'Getter sieht nativ aus',
  g("Object.getOwnPropertyDescriptor(Navigator.prototype,'userAgent').get.toString()") ===
    'function get userAgent() { [native code] }'
);
ok('toString selbst bleibt getarnt', g('Function.prototype.toString.toString()') === 'function toString() { [native code] }');
ok('echte Funktionen bleiben lesbar', g('(function abc(){ return 1 }).toString()').includes('return 1'));

const highVal = await g('navigator.userAgentData.getHighEntropyValues(["platformVersion","architecture","fullVersionList","model"])');
ok('platformVersion aus dem Profil', highVal.platformVersion === '15.0.0');
ok('architecture gesetzt', highVal.architecture === 'x86');
ok('fullVersionList vorhanden', Array.isArray(highVal.fullVersionList) && highVal.fullVersionList.length > 0);
ok('nicht angefragte Hints fehlen', !('bitness' in highVal));

runSpoof(ctx, payloadFor(P.win11Chrome, baseOpts));
ok('zweiter Lauf mit gleichem Stempel ist ein No-Op', g('navigator.userAgent') === buildUserAgent(P.win11Chrome));

runSpoof(ctx, payloadFor(P.win11Firefox, baseOpts));
ok('Profilwechsel greift', g('navigator.userAgent') === buildUserAgent(P.win11Firefox), g('navigator.userAgent'));
ok('Firefox: userAgentData weg', g('navigator.userAgentData') === undefined);
ok('Firefox: oscpu gesetzt', g('navigator.oscpu') === 'Windows NT 10.0; Win64; x64');
ok('Firefox: productSub 20100101', g('navigator.productSub') === '20100101');

ctx = makeRealm();
runSpoof(
  ctx,
  payloadFor(P.win11Chrome, {
    ...baseOpts,
    languages: true,
    languagesValue: 'de-DE, de, en-US',
    timezone: true,
    timezoneValue: 'America/New_York',
    screen: true,
    screenValue: { width: 1920, height: 1080, colorDepth: 24, dpr: 1 },
    hardware: true,
    hardwareValue: { cores: 8, memory: 8 }
  })
);
const g2 = (expr) => vm.runInContext(expr, ctx);
ok('language = de-DE', g2('navigator.language') === 'de-DE');
ok('languages Liste', JSON.stringify(g2('navigator.languages')) === '["de-DE","de","en-US"]');
ok('screen.width', g2('screen.width') === 1920);
ok('screen.availHeight abzüglich Leiste', g2('screen.availHeight') === 1040);
ok('devicePixelRatio', g2('devicePixelRatio') === 1);
ok('hardwareConcurrency', g2('navigator.hardwareConcurrency') === 8);
ok('deviceMemory', g2('navigator.deviceMemory') === 8);
ok('Zeitzone gemeldet', g2('Intl.DateTimeFormat().resolvedOptions().timeZone') === 'America/New_York');
ok('getTimezoneOffset Winter New York = 300', g2('new Date("2026-01-15T12:00:00Z").getTimezoneOffset()') === 300);
ok('getTimezoneOffset Sommer New York = 240', g2('new Date("2026-07-15T12:00:00Z").getTimezoneOffset()') === 240);
ok(
  'explizite Zeitzone der Seite bleibt erhalten',
  g2('new Intl.DateTimeFormat("en-US",{timeZone:"UTC"}).resolvedOptions().timeZone') === 'UTC'
);

ctx = makeRealm();
runSpoof(ctx, payloadFor(P.win11Firefox, { ...baseOpts, hideEngineTraces: true }));
ok('window.chrome entfernt bei Firefox-Spoof', vm.runInContext('typeof chrome', ctx) === 'undefined');
ctx = makeRealm();
runSpoof(ctx, payloadFor(P.win11Chrome, { ...baseOpts, hideEngineTraces: true }));
ok('window.chrome bleibt bei Chrome-Spoof', vm.runInContext('typeof chrome', ctx) === 'object');

ctx = makeRealm();
runSpoof(ctx, payloadFor(P.win11Chrome, { ...baseOpts, timezone: true, timezoneValue: 'Nicht/Existent' }));
ok('ungültige Zeitzone wird ignoriert', vm.runInContext('navigator.userAgent', ctx) === buildUserAgent(P.win11Chrome));

/* -------------------------------------------------------------------- */
head('Übersetzungen');

const enKeys = Object.keys(enMessages).sort();
const deKeys = Object.keys(deMessages).sort();
const onlyEn = enKeys.filter((k) => !(k in deMessages));
const onlyDe = deKeys.filter((k) => !(k in enMessages));
ok('beide Sprachen haben dieselben Schlüssel', onlyEn.length === 0 && onlyDe.length === 0,
  'nur en: ' + onlyEn.join(', ') + ' | nur de: ' + onlyDe.join(', '));
ok('keine leeren Texte', [...enKeys, ...deKeys].every((k) => {
  const bundle = k in enMessages ? enMessages : deMessages;
  return typeof bundle[k] === 'string' && bundle[k].trim().length > 0;
}));

const holes = (text) => (String(text).match(/\{[a-zA-Z]+\}/g) || []).sort().join(',');
const mismatched = enKeys.filter((k) => k in deMessages && holes(enMessages[k]) !== holes(deMessages[k]));
ok('Platzhalter stimmen überein', mismatched.length === 0, mismatched.join(', '));

/* Jeder im Code benutzte Schlüssel muss es auch geben - fängt Tippfehler. */
const used = new Set();
const KEY = /^[a-z][A-Za-z]*(?:\.[A-Za-z]+)+$/;
for (const file of readdirSync('ui')) {
  if (!/[.](js|html)$/.test(file)) continue;
  const text = readFileSync('ui/' + file, 'utf8');
  for (const re of [/\bt\(\s*'([^']+)'/g, /\btParts\(\s*'([^']+)'/g, /data-i18n(?:-ph|-title)?="([^"]+)"/g]) {
    let m;
    while ((m = re.exec(text)) !== null) {
      if (KEY.test(m[1])) used.add(m[1]);
    }
  }
}
const unknown = [...used].filter((k) => !(k in enMessages)).sort();
ok('alle benutzten Schlüssel sind übersetzt (' + used.size + ' Stück)', unknown.length === 0, unknown.join(', '));

/* _locales für Name und Beschreibung im Manifest */
const manifest = JSON.parse(readFileSync('manifest.json', 'utf8'));
ok('manifest hat default_locale', manifest.default_locale === 'en');
const localeKeys = [];
for (const lang of ['en', 'de']) {
  const msgs = JSON.parse(readFileSync('_locales/' + lang + '/messages.json', 'utf8'));
  localeKeys.push(Object.keys(msgs).sort().join(','));
  ok('_locales/' + lang + ' hat alle Manifest-Texte',
    ['extName', 'extDescription', 'actionTitle'].every((k) => msgs[k] && msgs[k].message));
}
ok('_locales in beiden Sprachen deckungsgleich', localeKeys[0] === localeKeys[1]);
for (const field of ['name', 'description']) {
  const value = manifest[field];
  const key = /^__MSG_(\w+)__$/.exec(value);
  ok('manifest.' + field + ' verweist auf _locales', !!key, value);
}

console.log('\n' + (fails === 0 ? 'Alle Tests bestanden.' : fails + ' Test(s) fehlgeschlagen.'));
process.exit(fails === 0 ? 0 : 1);

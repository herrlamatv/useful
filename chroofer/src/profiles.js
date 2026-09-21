/* Chroofer - by herrlamatv
 * Profil-Daten: Betriebssysteme, OS-Versionen, Browser + User-Agent/Client-Hints-Bau.
 */

export const OS_FAMILIES = {
  windows: {
    label: 'Windows',
    navPlatform: 'Win32',
    chPlatform: 'Windows',
    arch: 'x86',
    bitness: '64',
    mobile: false,
    maxTouchPoints: 0,
    versions: [
      { id: '11',  label: 'Windows 11',  ch: '15.0.0', ua: 'Windows NT 10.0; Win64; x64', oscpu: 'Windows NT 10.0; Win64; x64' },
      { id: '10',  label: 'Windows 10',  ch: '10.0.0', ua: 'Windows NT 10.0; Win64; x64', oscpu: 'Windows NT 10.0; Win64; x64' },
      { id: '8.1', label: 'Windows 8.1', ch: '0.3.0',  ua: 'Windows NT 6.3; Win64; x64',  oscpu: 'Windows NT 6.3; Win64; x64' },
      { id: '7',   label: 'Windows 7',   ch: '0.1.0',  ua: 'Windows NT 6.1; Win64; x64',  oscpu: 'Windows NT 6.1; Win64; x64' }
    ]
  },
  macos: {
    label: 'macOS',
    navPlatform: 'MacIntel',
    chPlatform: 'macOS',
    arch: 'arm',
    bitness: '64',
    mobile: false,
    maxTouchPoints: 0,
    versions: [
      { id: '15', label: 'macOS 15 (Sequoia)',  ch: '15.6.0', ua: 'Macintosh; Intel Mac OS X 10_15_7', gecko: 'Macintosh; Intel Mac OS X 10.15', oscpu: 'Intel Mac OS X 10.15' },
      { id: '14', label: 'macOS 14 (Sonoma)',   ch: '14.6.0', ua: 'Macintosh; Intel Mac OS X 10_15_7', gecko: 'Macintosh; Intel Mac OS X 10.15', oscpu: 'Intel Mac OS X 10.15' },
      { id: '13', label: 'macOS 13 (Ventura)',  ch: '13.6.0', ua: 'Macintosh; Intel Mac OS X 10_15_7', gecko: 'Macintosh; Intel Mac OS X 10.15', oscpu: 'Intel Mac OS X 10.15' },
      { id: '12', label: 'macOS 12 (Monterey)', ch: '12.7.0', ua: 'Macintosh; Intel Mac OS X 10_15_7', gecko: 'Macintosh; Intel Mac OS X 10.15', oscpu: 'Intel Mac OS X 10.15' }
    ]
  },
  linux: {
    label: 'Linux',
    navPlatform: 'Linux x86_64',
    chPlatform: 'Linux',
    arch: 'x86',
    bitness: '64',
    mobile: false,
    maxTouchPoints: 0,
    versions: [
      { id: 'x86_64',  label: 'Linux x86_64',   ch: '', ua: 'X11; Linux x86_64',        oscpu: 'Linux x86_64' },
      { id: 'aarch64', label: 'Linux aarch64',  ch: '', ua: 'X11; Linux aarch64',       oscpu: 'Linux aarch64', arch: 'arm' },
      { id: 'ubuntu',  label: 'Ubuntu x86_64',  ch: '', ua: 'X11; Ubuntu; Linux x86_64', oscpu: 'Linux x86_64' }
    ]
  },
  chromeos: {
    label: 'ChromeOS',
    navPlatform: 'Linux x86_64',
    chPlatform: 'Chrome OS',
    arch: 'x86',
    bitness: '64',
    mobile: false,
    maxTouchPoints: 0,
    versions: [
      { id: '16', label: 'ChromeOS 16', ch: '16181.0.0', ua: 'X11; CrOS x86_64 16181.0.0', oscpu: 'Linux x86_64' },
      { id: '14', label: 'ChromeOS 14', ch: '14541.0.0', ua: 'X11; CrOS x86_64 14541.0.0', oscpu: 'Linux x86_64' }
    ]
  },
  android: {
    label: 'Android',
    navPlatform: 'Linux armv81',
    chPlatform: 'Android',
    arch: 'arm',
    bitness: '64',
    mobile: true,
    maxTouchPoints: 5,
    model: 'K',
    versions: [
      { id: '15', label: 'Android 15', ch: '15.0.0', ua: 'Linux; Android 10; K', gecko: 'Android 15; Mobile' },
      { id: '14', label: 'Android 14', ch: '14.0.0', ua: 'Linux; Android 10; K', gecko: 'Android 14; Mobile' },
      { id: '13', label: 'Android 13', ch: '13.0.0', ua: 'Linux; Android 10; K', gecko: 'Android 13; Mobile' },
      { id: '12', label: 'Android 12', ch: '12.0.0', ua: 'Linux; Android 10; K', gecko: 'Android 12; Mobile' }
    ]
  },
  ios: {
    label: 'iOS',
    navPlatform: 'iPhone',
    chPlatform: 'iOS',
    arch: 'arm',
    bitness: '64',
    mobile: true,
    maxTouchPoints: 5,
    versions: [
      { id: '18', label: 'iOS 18 (iPhone)', ch: '18.5.0', ua: 'iPhone; CPU iPhone OS 18_5 like Mac OS X', ver: '18.5' },
      { id: '17', label: 'iOS 17 (iPhone)', ch: '17.6.0', ua: 'iPhone; CPU iPhone OS 17_6 like Mac OS X', ver: '17.6' },
      { id: '16', label: 'iOS 16 (iPhone)', ch: '16.7.0', ua: 'iPhone; CPU iPhone OS 16_7 like Mac OS X', ver: '16.7' }
    ]
  },
  ipados: {
    label: 'iPadOS',
    navPlatform: 'MacIntel',
    chPlatform: 'iOS',
    arch: 'arm',
    bitness: '64',
    mobile: false,
    maxTouchPoints: 5,
    versions: [
      { id: '18', label: 'iPadOS 18 (iPad)', ch: '18.5.0', ua: 'iPad; CPU OS 18_5 like Mac OS X', ver: '18.5' },
      { id: '17', label: 'iPadOS 17 (iPad)', ch: '17.6.0', ua: 'iPad; CPU OS 17_6 like Mac OS X', ver: '17.6' }
    ]
  }
};

export const BROWSERS = {
  chrome:  { label: 'Chrome',  engine: 'blink',  versions: ['142', '141', '140', '139', '138', '136', '133', '131', '128', '120'] },
  edge:    { label: 'Edge',    engine: 'blink',  versions: ['142', '141', '140', '139', '138', '136', '131', '128'] },
  opera:   { label: 'Opera',   engine: 'blink',  versions: ['126', '125', '124', '120', '115'] },
  brave:   { label: 'Brave',   engine: 'blink',  versions: ['142', '141', '140', '138', '133'] },
  firefox: { label: 'Firefox', engine: 'gecko',  versions: ['145', '144', '143', '140', '138', '128'] },
  safari:  { label: 'Safari',  engine: 'webkit', versions: ['18.5', '18.2', '18.0', '17.6', '17.0'] }
};

/* Opera zählt eigene Versionen; die Chromium-Basis liegt rund 16 darüber. */
const operaToChromium = (v) => String(Number(v) + 16);

export const DEFAULT_PROFILE = Object.freeze({
  os: 'windows',
  osVersion: '11',
  browser: 'chrome',
  browserVersion: '142',
  customUA: ''
});

export function osEntry(profile) {
  const fam = OS_FAMILIES[profile.os] || OS_FAMILIES.windows;
  const ver = fam.versions.find((v) => v.id === profile.osVersion) || fam.versions[0];
  return { fam, ver };
}

const majorOf = (v) => String(v || '').split('.')[0] || '0';
const fullVersionOf = (v) => (String(v || '0').includes('.') ? String(v) : String(v) + '.0.0.0');

/* ------------------------------------------------------------------ */
/* User-Agent-String                                                    */
/* ------------------------------------------------------------------ */

export function buildUserAgent(profile) {
  if (profile.customUA && profile.customUA.trim()) return profile.customUA.trim();

  const { fam, ver } = osEntry(profile);
  const br = BROWSERS[profile.browser] || BROWSERS.chrome;
  const bv = profile.browserVersion || br.versions[0];
  const major = majorOf(bv);
  const isApple = profile.os === 'ios' || profile.os === 'ipados';

  if (br.engine === 'webkit') {
    const sv = String(bv).includes('.') ? bv : bv + '.0';
    if (isApple) {
      return 'Mozilla/5.0 (' + ver.ua + ') AppleWebKit/605.1.15 (KHTML, like Gecko) Version/' + sv + ' Mobile/15E148 Safari/604.1';
    }
    return 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/' + sv + ' Safari/605.1.15';
  }

  if (br.engine === 'gecko') {
    if (isApple) {
      return 'Mozilla/5.0 (' + ver.ua + ') AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/' + major + '.0 Mobile/15E148 Safari/605.1.15';
    }
    if (profile.os === 'android') {
      const tok = ver.gecko || 'Android 14; Mobile';
      return 'Mozilla/5.0 (' + tok + '; rv:' + major + '.0) Gecko/' + major + '.0 Firefox/' + major + '.0';
    }
    const token = ver.gecko || ver.ua;
    return 'Mozilla/5.0 (' + token + '; rv:' + major + '.0) Gecko/20100101 Firefox/' + major + '.0';
  }

  /* Blink */
  const chromiumMajor = profile.browser === 'opera' ? operaToChromium(major) : major;

  if (isApple) {
    /* Chromium auf iOS ist in Wahrheit WebKit - deshalb CriOS. */
    return 'Mozilla/5.0 (' + ver.ua + ') AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/' + chromiumMajor + '.0.0.0 Mobile/15E148 Safari/604.1';
  }

  const mobileTag = fam.mobile ? 'Mobile ' : '';
  let ua = 'Mozilla/5.0 (' + ver.ua + ') AppleWebKit/537.36 (KHTML, like Gecko) Chrome/' + chromiumMajor + '.0.0.0 ' + mobileTag + 'Safari/537.36';
  if (profile.browser === 'edge') ua += ' Edg/' + major + '.0.0.0';
  if (profile.browser === 'opera') ua += ' OPR/' + major + '.0.0.0';
  return ua;
}

/* ------------------------------------------------------------------ */
/* Client Hints                                                         */
/* ------------------------------------------------------------------ */

const GREASE = { brand: 'Not=A?Brand', version: '24' };

export function buildBrands(profile, full) {
  const br = BROWSERS[profile.browser] || BROWSERS.chrome;
  if (br.engine !== 'blink') return [];

  const bv = profile.browserVersion || br.versions[0];
  const major = majorOf(bv);
  const chromiumMajor = profile.browser === 'opera' ? operaToChromium(major) : major;
  const v = (m) => (full ? fullVersionOf(m) : m);

  const list = [
    { brand: 'Chromium', version: v(chromiumMajor) },
    { brand: GREASE.brand, version: full ? fullVersionOf(GREASE.version) : GREASE.version }
  ];

  const named = { chrome: 'Google Chrome', edge: 'Microsoft Edge', opera: 'Opera', brave: 'Brave' }[profile.browser];
  if (named) list.unshift({ brand: named, version: v(major) });
  return list;
}

export function brandsToHeader(brands) {
  return brands.map((b) => '"' + b.brand + '";v="' + b.version + '"').join(', ');
}

export function buildClientHints(profile) {
  const { fam, ver } = osEntry(profile);
  const br = BROWSERS[profile.browser] || BROWSERS.chrome;
  const isApple = profile.os === 'ios' || profile.os === 'ipados';

  return {
    /* Nur echte Chromium-Desktop/Android-Kombis senden Client Hints. */
    supported: br.engine === 'blink' && !isApple,
    brands: buildBrands(profile, false),
    fullVersionList: buildBrands(profile, true),
    mobile: !!fam.mobile,
    platform: fam.chPlatform,
    platformVersion: ver.ch || '',
    architecture: ver.arch || fam.arch || '',
    bitness: fam.bitness || '',
    model: fam.model && fam.mobile ? fam.model : '',
    uaFullVersion: fullVersionOf(profile.browserVersion || br.versions[0]),
    wow64: false
  };
}

/* Alles, was Injector und Netzwerk-Regeln brauchen - an einer Stelle. */
export function resolveProfile(profile) {
  const p = Object.assign({}, DEFAULT_PROFILE, profile || {});
  const { fam, ver } = osEntry(p);
  const br = BROWSERS[p.browser] || BROWSERS.chrome;

  return {
    profile: p,
    ua: buildUserAgent(p),
    ch: buildClientHints(p),
    engine: br.engine,
    osLabel: ver.label,
    browserLabel: br.label + ' ' + p.browserVersion,
    navPlatform: fam.navPlatform,
    oscpu: ver.oscpu || '',
    vendor: br.engine === 'webkit' ? 'Apple Computer, Inc.' : br.engine === 'gecko' ? '' : 'Google Inc.',
    maxTouchPoints: fam.maxTouchPoints,
    mobile: !!fam.mobile
  };
}

/**
 * Kurzbeschreibung eines Profils, z. B. "Windows 11 · Chrome 142".
 * OS- und Browsernamen sind Produktnamen und in jeder Sprache gleich; nur der
 * Hinweis auf einen eigenen User-Agent wird übersetzt und deshalb reingereicht.
 */
export function profileLabel(profile, customLabel) {
  if (profile && profile.customUA && profile.customUA.trim()) return customLabel || 'Custom user agent';
  const r = resolveProfile(profile);
  return r.osLabel + ' · ' + r.browserLabel;
}

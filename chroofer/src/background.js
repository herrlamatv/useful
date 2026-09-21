/* Chroofer - by herrlamatv
 * Service Worker: Netzwerk-Regeln pflegen, Seiten-Kontext patchen, Badge setzen.
 *
 * Grundregel: das Profil gilt pro Tab und richtet sich nach der Seite in der
 * Adresszeile. Eingebettete Frames bekommen dasselbe Profil - genau wie ein
 * echter Browser überall denselben User-Agent sendet.
 */

import { resolveProfile } from './profiles.js';
import {
  loadSettings,
  saveSettings,
  mergeSettings,
  normalizeHost,
  hostOfUrl,
  resolveForHost,
  randomProfileFor,
  sanitizeProfile,
  newSite,
  DEFAULT_SETTINGS
} from './settings.js';
import { buildRules, tabRuleFor, TAB_RULE_BASE } from './rules.js';
import { chrooferSpoof } from './spoof.js';

let cache = null;

/** tabId -> { host, decision, profile, inherited, sig } */
const tabStates = new Map();

async function getSettings() {
  if (!cache) cache = await loadSettings();
  return cache;
}

/* Das Zufallsprofil bleibt pro Browser-Sitzung stabil - sonst sieht jede
   Anfrage anders aus und das fällt stärker auf als ein festes Profil. */
async function sessionSeed() {
  const got = await chrome.storage.session.get('seed');
  if (got.seed) return got.seed;
  const seed = 'chroofer-' + Date.now() + '-' + Math.random().toString(36).slice(2);
  await chrome.storage.session.set({ seed });
  return seed;
}

/** Das Profil, mit dem wirklich gearbeitet wird (Zufallsmodus schon aufgelöst). */
async function effectiveProfile(settings, decision) {
  if (!decision || !decision.active) return null;
  if (decision.source === 'global' && settings.global.mode === 'random') {
    return randomProfileFor(await sessionSeed());
  }
  return decision.profile;
}

function siteProfilesOf(settings) {
  const map = {};
  for (const site of settings.sites) {
    if (site.enabled && site.mode === 'random') map[site.pattern] = randomProfileFor(site.pattern);
  }
  return map;
}

/* ------------------------------------------------------------------ */
/* Netzwerk-Regeln                                                      */
/* ------------------------------------------------------------------ */

async function applyRules() {
  const settings = await getSettings();
  const globalProfile =
    settings.global.mode === 'random' ? randomProfileFor(await sessionSeed()) : settings.global.profile;
  const rules = buildRules(settings, globalProfile, siteProfilesOf(settings));

  try {
    const existing = await chrome.declarativeNetRequest.getSessionRules();
    /* Tab-Regeln bleiben stehen, die gehören den offenen Tabs. */
    const removeRuleIds = existing.filter((r) => r.id < TAB_RULE_BASE).map((r) => r.id);
    await chrome.declarativeNetRequest.updateSessionRules({ removeRuleIds, addRules: rules });
  } catch (err) {
    console.error('[Chroofer] Regeln konnten nicht gesetzt werden:', err);
  }
}

async function setTabRule(tabId, decision, profile, options) {
  const rule = tabRuleFor(tabId, decision, profile, options);
  const sig = rule ? JSON.stringify([rule.action, rule.condition]) : 'none';
  const state = tabStates.get(tabId);
  if (state && state.sig === sig) return sig;

  try {
    await chrome.declarativeNetRequest.updateSessionRules({
      removeRuleIds: [TAB_RULE_BASE + tabId],
      addRules: rule ? [rule] : []
    });
  } catch (err) {
    console.error('[Chroofer] Tab-Regel fehlgeschlagen:', err);
  }
  return sig;
}

async function clearTabRule(tabId) {
  tabStates.delete(tabId);
  try {
    await chrome.declarativeNetRequest.updateSessionRules({ removeRuleIds: [TAB_RULE_BASE + tabId] });
  } catch (e) {}
}

/** Nach einem Neustart des Service Workers können Regeln geschlossener Tabs übrig sein. */
async function purgeOrphanTabRules() {
  try {
    const [existing, tabs] = await Promise.all([
      chrome.declarativeNetRequest.getSessionRules(),
      chrome.tabs.query({})
    ]);
    const open = new Set(tabs.map((t) => TAB_RULE_BASE + t.id));
    const stale = existing.filter((r) => r.id >= TAB_RULE_BASE && !open.has(r.id)).map((r) => r.id);
    if (stale.length) await chrome.declarativeNetRequest.updateSessionRules({ removeRuleIds: stale });
  } catch (e) {}
}

/* ------------------------------------------------------------------ */
/* Seiten-Kontext                                                       */
/* ------------------------------------------------------------------ */

function payloadFor(profile, options) {
  const r = resolveProfile(sanitizeProfile(profile));
  const opts = {
    navigator: options.navigator,
    hideWebdriver: options.hideWebdriver,
    languages: options.languages,
    languagesValue: options.languagesValue,
    timezone: options.timezone,
    timezoneValue: options.timezoneValue,
    screen: options.screen,
    screenValue: options.screenValue,
    hardware: options.hardware,
    hardwareValue: options.hardwareValue,
    hideEngineTraces: options.hideEngineTraces
  };
  return {
    stamp: JSON.stringify([r.ua, r.ch, opts]),
    ua: r.ua,
    ch: r.ch,
    engine: r.engine,
    navPlatform: r.navPlatform,
    oscpu: r.oscpu,
    vendor: r.vendor,
    maxTouchPoints: r.maxTouchPoints,
    mobile: r.mobile,
    opts
  };
}

async function injectFrame(tabId, frameId, profile, options) {
  if (!profile || options.navigator === false) return;
  try {
    await chrome.scripting.executeScript({
      target: { tabId, frameIds: [frameId] },
      world: 'MAIN',
      injectImmediately: true,
      func: chrooferSpoof,
      args: [payloadFor(profile, options)]
    });
  } catch (err) {
    /* Seite wurde schon wieder verlassen, oder Chrome erlaubt hier kein Skript. */
  }
}

/** Zustand eines Tabs neu bestimmen (nach Neustart des Service Workers). */
async function rebuildTabState(tabId) {
  try {
    const tab = await chrome.tabs.get(tabId);
    const host = hostOfUrl(tab.url || '');
    if (!host) return null;
    const settings = await getSettings();
    const decision = resolveForHost(settings, host);
    const profile = await effectiveProfile(settings, decision);
    const state = { host, decision: plainDecision(decision), profile, inherited: false, sig: null };
    tabStates.set(tabId, state);
    return state;
  } catch (e) {
    return null;
  }
}

function plainDecision(decision) {
  return {
    active: decision.active,
    source: decision.source,
    scope: decision.scope,
    random: !!decision.random,
    pattern: decision.entry ? decision.entry.pattern : null
  };
}

const REDIRECT_QUALIFIERS = ['server_redirect', 'client_redirect'];

chrome.webNavigation.onCommitted.addListener(
  async (details) => {
    const { tabId, frameId, url } = details;

    if (frameId !== 0) {
      /* Eingebetteter Frame: Profil kommt vom Tab, nicht vom Frame-Host. */
      const state = tabStates.get(tabId) || (await rebuildTabState(tabId));
      if (state && state.profile) {
        const settings = await getSettings();
        injectFrame(tabId, frameId, state.profile, settings.options);
      }
      return;
    }

    const host = hostOfUrl(url);
    if (!host) {
      await clearTabRule(tabId);
      updateBadge(tabId, null);
      return;
    }

    const settings = await getSettings();
    let decision = resolveForHost(settings, host);
    let inherited = false;

    /* Weiterleitung: wenn die vorherige Seite "Weiterleitungen folgen" erlaubt
       und das Ziel keine eigene Regel hat, bleibt ihr Profil aktiv. */
    const prev = tabStates.get(tabId);
    const isRedirect = (details.transitionQualifiers || []).some((q) => REDIRECT_QUALIFIERS.includes(q));
    if (isRedirect && !decision.entry && prev && prev.decision.scope && prev.decision.scope.redirects) {
      decision = { ...prev.decision, entry: null, profile: prev.profile, scope: prev.decision.scope };
      inherited = true;
    }

    const profile = inherited ? prev.profile : await effectiveProfile(settings, decision);
    const state = {
      host,
      decision: inherited ? prev.decision : plainDecision(decision),
      profile,
      inherited,
      sig: prev ? prev.sig : null
    };
    tabStates.set(tabId, state);

    state.sig = await setTabRule(tabId, { active: !!profile, scope: state.decision.scope }, profile, settings.options);
    tabStates.set(tabId, state);

    await chrome.storage.session.set({ lastSite: { host, url, tabId } });

    injectFrame(tabId, 0, profile, settings.options);
    updateBadge(tabId, state);
  },
  { url: [{ schemes: ['http'] }, { schemes: ['https'] }] }
);

chrome.tabs.onRemoved.addListener((tabId) => clearTabRule(tabId));

/* ------------------------------------------------------------------ */
/* Badge                                                                */
/* ------------------------------------------------------------------ */

async function updateBadge(tabId, state) {
  const settings = await getSettings();
  if (!settings.options.badge || !state) {
    chrome.action.setBadgeText({ tabId, text: '' }).catch(() => {});
    return;
  }
  const active = !!state.profile;
  const bySite = !!state.decision.pattern || state.inherited;
  try {
    await chrome.action.setBadgeText({ tabId, text: active ? (bySite ? 'SITE' : 'ON') : 'OFF' });
    await chrome.action.setBadgeBackgroundColor({
      tabId,
      color: active ? (bySite ? '#0071e3' : '#34c759') : '#b5ada0'
    });
    await chrome.action.setBadgeTextColor({ tabId, color: '#ffffff' }).catch(() => {});
  } catch (e) {}
}

async function refreshBadgeForTab(tabId) {
  const state = tabStates.get(tabId) || (await rebuildTabState(tabId));
  await updateBadge(tabId, state);
}

/** Alle offenen Tabs neu bewerten - Regeln in einem Rutsch, damit es auch mit
    vielen Tabs flüssig bleibt. */
async function refreshAllTabs() {
  const tabs = await chrome.tabs.query({});
  const settings = await getSettings();
  const addRules = [];
  const removeRuleIds = [];

  for (const tab of tabs) {
    if (tab.id == null) continue;
    const host = hostOfUrl(tab.url || '');
    if (!host) {
      tabStates.delete(tab.id);
      removeRuleIds.push(TAB_RULE_BASE + tab.id);
      updateBadge(tab.id, null);
      continue;
    }

    const decision = resolveForHost(settings, host);
    const profile = await effectiveProfile(settings, decision);
    const state = { host, decision: plainDecision(decision), profile, inherited: false, sig: null };
    const rule = tabRuleFor(tab.id, { active: !!profile, scope: state.decision.scope }, profile, settings.options);

    removeRuleIds.push(TAB_RULE_BASE + tab.id);
    if (rule) {
      addRules.push(rule);
      state.sig = JSON.stringify([rule.action, rule.condition]);
    } else {
      state.sig = 'none';
    }

    tabStates.set(tab.id, state);
    updateBadge(tab.id, state);
  }

  try {
    await chrome.declarativeNetRequest.updateSessionRules({ removeRuleIds, addRules });
  } catch (err) {
    console.error('[Chroofer] Tab-Regeln fehlgeschlagen:', err);
  }
}

chrome.tabs.onActivated.addListener(({ tabId }) => {
  refreshBadgeForTab(tabId);
  chrome.tabs
    .get(tabId)
    .then((tab) => {
      const host = hostOfUrl(tab.url || '');
      if (host) chrome.storage.session.set({ lastSite: { host, url: tab.url, tabId } });
    })
    .catch(() => {});
});

/* ------------------------------------------------------------------ */
/* Nachrichten aus Popup / Optionen                                     */
/* ------------------------------------------------------------------ */

async function persist(settings) {
  cache = await saveSettings(settings);
  await applyRules();
  await refreshAllTabs();
  return cache;
}

async function upsertSite(host, patch) {
  const settings = await getSettings();
  const pattern = normalizeHost(host);
  /* Code statt Text - übersetzt wird in der Oberfläche. */
  if (!pattern) throw new Error('invalidHost');

  const sites = settings.sites.slice();
  const idx = sites.findIndex((s) => s.pattern === pattern);
  const base = idx >= 0 ? sites[idx] : newSite(pattern, settings.global.profile);
  const next = Object.assign({}, base, patch, { pattern });
  if (idx >= 0) sites[idx] = next;
  else sites.push(next);

  return persist(Object.assign({}, settings, { sites }));
}

async function removeSite(host) {
  const settings = await getSettings();
  const pattern = normalizeHost(host);
  return persist(Object.assign({}, settings, { sites: settings.sites.filter((s) => s.pattern !== pattern) }));
}

/** Die zuletzt besuchte echte Website - für "Seite, auf der ich gerade bin". */
async function currentSite() {
  try {
    const [active] = await chrome.tabs.query({ active: true, currentWindow: true });
    const host = active ? hostOfUrl(active.url || '') : '';
    if (host) return { host, url: active.url, tabId: active.id };
  } catch (e) {}

  const stored = await chrome.storage.session.get('lastSite');
  if (stored.lastSite && stored.lastSite.host) return stored.lastSite;

  try {
    const tabs = await chrome.tabs.query({});
    const candidates = tabs
      .filter((t) => hostOfUrl(t.url || ''))
      .sort((a, b) => (b.lastAccessed || 0) - (a.lastAccessed || 0));
    if (candidates.length) {
      const t = candidates[0];
      return { host: hostOfUrl(t.url), url: t.url, tabId: t.id };
    }
  } catch (e) {}

  return null;
}

async function overview() {
  const settings = await getSettings();
  const randomProfile = randomProfileFor(await sessionSeed());
  return {
    settings,
    randomProfile,
    randomUA: resolveProfile(randomProfile).ua,
    currentSite: await currentSite()
  };
}

async function describeTab(tabId) {
  const settings = await getSettings();
  let tab = null;
  try {
    tab =
      tabId != null
        ? await chrome.tabs.get(tabId)
        : (await chrome.tabs.query({ active: true, currentWindow: true }))[0];
  } catch (e) {}

  const url = tab && tab.url ? tab.url : '';
  const host = hostOfUrl(url);
  if (!host) return { settings, tabId: tab ? tab.id : null, url, host: '', supported: false };

  const state = (tab && tabStates.get(tab.id)) || null;
  const decision = resolveForHost(settings, host);
  const profile = await effectiveProfile(settings, decision);
  const inherited = !!(state && state.inherited && state.host === host);
  const shown = inherited ? state.profile : profile;

  return {
    settings,
    tabId: tab ? tab.id : null,
    url,
    host,
    supported: true,
    decision: plainDecision(decision),
    entry: decision.entry || null,
    inherited,
    inheritedFrom: inherited ? state.decision.pattern : null,
    effective: shown,
    effectiveUA: shown ? resolveProfile(shown).ua : null
  };
}

/** Liest die Werte, die die Seite gerade wirklich sieht - zur Kontrolle. */
async function readLive(tabId) {
  const [res] = await chrome.scripting.executeScript({
    target: { tabId },
    world: 'MAIN',
    func: () => ({
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      vendor: navigator.vendor,
      languages: (navigator.languages || []).join(', '),
      uaPlatform: navigator.userAgentData ? navigator.userAgentData.platform : null,
      uaMobile: navigator.userAgentData ? navigator.userAgentData.mobile : null,
      brands: navigator.userAgentData
        ? navigator.userAgentData.brands.map((b) => b.brand + ' ' + b.version).join(', ')
        : null,
      screen: screen.width + '×' + screen.height + ' @' + devicePixelRatio + 'x',
      cores: navigator.hardwareConcurrency,
      memory: navigator.deviceMemory ?? null,
      touch: navigator.maxTouchPoints,
      timezone: (() => {
        try {
          return Intl.DateTimeFormat().resolvedOptions().timeZone;
        } catch (e) {
          return null;
        }
      })()
    })
  });
  return res ? res.result : null;
}

const handlers = {
  async getState() {
    return overview();
  },
  async describeTab({ tabId }) {
    return describeTab(tabId);
  },
  async currentSite() {
    return currentSite();
  },
  async saveSettings({ settings }) {
    await persist(mergeSettings(settings));
    return overview();
  },
  async setEnabled({ value }) {
    const settings = await getSettings();
    await persist(Object.assign({}, settings, { enabled: !!value }));
    return overview();
  },
  async setGlobal({ mode, profile }) {
    const settings = await getSettings();
    const global = Object.assign({}, settings.global);
    if (mode) global.mode = mode;
    if (profile) global.profile = sanitizeProfile(profile);
    await persist(Object.assign({}, settings, { global }));
    return overview();
  },
  async setSite({ host, patch }) {
    const clean = patch && patch.profile ? Object.assign({}, patch, { profile: sanitizeProfile(patch.profile) }) : patch;
    await upsertSite(host, clean || {});
    return overview();
  },
  async removeSite({ host }) {
    await removeSite(host);
    return overview();
  },
  async reroll() {
    await chrome.storage.session.remove('seed');
    await sessionSeed();
    await applyRules();
    await refreshAllTabs();
    return overview();
  },
  async reset() {
    await persist(mergeSettings(DEFAULT_SETTINGS));
    return overview();
  },
  async readLive({ tabId }) {
    return readLive(tabId);
  }
};

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  const handler = msg && handlers[msg.type];
  if (!handler) return false;
  Promise.resolve(handler(msg))
    .then((data) => sendResponse({ ok: true, data }))
    .catch((err) => sendResponse({ ok: false, error: String((err && err.message) || err) }));
  return true; // asynchrone Antwort
});

/* ------------------------------------------------------------------ */
/* Lebenszyklus                                                         */
/* ------------------------------------------------------------------ */

chrome.runtime.onInstalled.addListener(async (details) => {
  cache = await loadSettings();
  await saveSettings(cache);
  await applyRules();
  await refreshAllTabs();
  if (details.reason === 'install') {
    chrome.tabs.create({ url: chrome.runtime.getURL('ui/options.html') });
  }
});

chrome.runtime.onStartup.addListener(async () => {
  cache = null;
  tabStates.clear();
  await chrome.storage.session.remove('seed');
  await applyRules();
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area === 'local' && changes.chroofer) {
    cache = mergeSettings(changes.chroofer.newValue);
    applyRules().then(refreshAllTabs);
  }
});

/* Beim Aufwachen des Service Workers aufräumen und die Regeln neu setzen. */
purgeOrphanTabRules().then(applyRules);

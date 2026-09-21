/* Chroofer - by herrlamatv
 * declarativeNetRequest-Regeln: sie ersetzen User-Agent und Client-Hint-Header
 * schon beim Senden - das ist, was der Server wirklich sieht.
 *
 * Zwei Sorten Regeln:
 *
 *  1. Domain-Regeln (statisch, aus den Einstellungen)
 *     - Navigationen zu einer Domain (main_frame / sub_frame)
 *     - alles, was eine Seite dieser Domain anfragt (initiatorDomains)
 *
 *  2. Tab-Regeln (dynamisch, vom Service Worker bei jedem Seitenaufruf gesetzt)
 *     - alles, was ein bestimmter Tab lädt, außer der Navigation selbst
 *     - dadurch exakt steuerbar: Subdomains ja/nein, Drittanbieter ja/nein,
 *       und ein Profil kann einer Weiterleitung auf eine andere Domain folgen
 */

import { resolveProfile, brandsToHeader } from './profiles.js';

const LOW_ENTROPY = ['sec-ch-ua', 'sec-ch-ua-mobile', 'sec-ch-ua-platform'];

const HIGH_ENTROPY = [
  'sec-ch-ua-arch',
  'sec-ch-ua-bitness',
  'sec-ch-ua-full-version',
  'sec-ch-ua-full-version-list',
  'sec-ch-ua-model',
  'sec-ch-ua-platform-version',
  'sec-ch-ua-wow64',
  'sec-ch-ua-form-factors'
];

const ALL_CH = LOW_ENTROPY.concat(HIGH_ENTROPY);

const NAVIGATION = ['main_frame', 'sub_frame'];

export const TAB_RULE_BASE = 1000000;
export const TAB_RULE_PRIORITY = 900000;

/** Header-Aktionen für ein aufgelöstes Profil. */
export function headerActionsFor(profile, options) {
  const r = resolveProfile(profile);
  const ch = r.ch;
  const actions = [{ header: 'user-agent', operation: 'set', value: r.ua }];

  if (!ch.supported) {
    /* Firefox, Safari und alles auf iOS senden gar keine Client Hints. */
    for (const h of ALL_CH) actions.push({ header: h, operation: 'remove' });
  } else {
    actions.push({ header: 'sec-ch-ua', operation: 'set', value: brandsToHeader(ch.brands) });
    actions.push({ header: 'sec-ch-ua-mobile', operation: 'set', value: ch.mobile ? '?1' : '?0' });
    actions.push({ header: 'sec-ch-ua-platform', operation: 'set', value: '"' + ch.platform + '"' });

    if (options && options.highEntropyHeaders) {
      const high = {
        'sec-ch-ua-arch': '"' + ch.architecture + '"',
        'sec-ch-ua-bitness': '"' + ch.bitness + '"',
        'sec-ch-ua-full-version': '"' + ch.uaFullVersion + '"',
        'sec-ch-ua-full-version-list': brandsToHeader(ch.fullVersionList),
        'sec-ch-ua-model': '"' + ch.model + '"',
        'sec-ch-ua-platform-version': '"' + ch.platformVersion + '"',
        'sec-ch-ua-wow64': '?0',
        'sec-ch-ua-form-factors': ch.mobile ? '"Mobile"' : '"Desktop"'
      };
      for (const [h, v] of Object.entries(high)) actions.push({ header: h, operation: 'set', value: v });
    } else {
      /* Sonst lieber entfernen, als echte Werte durchrutschen zu lassen. */
      for (const h of HIGH_ENTROPY) actions.push({ header: h, operation: 'remove' });
    }
  }

  if (options && options.languages && options.languagesValue) {
    const langs = String(options.languagesValue)
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    if (langs.length) {
      const value = langs
        .map((l, i) => (i === 0 ? l : l + ';q=' + Math.max(0.1, 1 - i * 0.1).toFixed(1)))
        .join(',');
      actions.push({ header: 'accept-language', operation: 'set', value });
    }
  }

  return actions;
}

/* Längeres Muster = spezifischer = höhere Priorität. */
function sitePriority(pattern, off) {
  return 100 + Math.min(pattern.length, 180) * 10 + (off ? 5 : 0);
}

function escapeRegex(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** Trifft genau diesen Host, keine Subdomains. */
function exactHostCondition(pattern) {
  return { regexFilter: '^https?://' + escapeRegex(pattern) + '(?::\\d+)?/' };
}

/**
 * Domain-Regeln aus den Einstellungen.
 *
 * @param settings      gemergte Einstellungen
 * @param globalProfile Profil, das global gilt (im Zufallsmodus schon gewürfelt)
 * @param siteProfiles  Map Muster -> Profil (im Zufallsmodus schon gewürfelt)
 */
export function buildRules(settings, globalProfile, siteProfiles) {
  const rules = [];
  let id = 1;

  if (!settings.enabled || !settings.options.headers) return rules;

  for (const site of settings.sites) {
    if (!site.enabled) continue;

    const off = site.mode === 'off';
    const priority = sitePriority(site.pattern, off);
    const action = off
      ? { type: 'allow' }
      : {
          type: 'modifyHeaders',
          requestHeaders: headerActionsFor((siteProfiles && siteProfiles[site.pattern]) || site.profile, settings.options)
        };

    /* 1. Navigation zu dieser Domain. */
    const target = site.subdomains ? { requestDomains: [site.pattern] } : exactHostCondition(site.pattern);
    rules.push({
      id: id++,
      priority,
      action,
      condition: Object.assign({ resourceTypes: NAVIGATION }, target)
    });

    /* 2. Was eine Seite dieser Domain nachlädt. initiatorDomains deckt in Chrome
       immer auch Subdomains ab - ohne Subdomain-Option übernimmt das die
       Tab-Regel, die exakt auf den Tab zugeschnitten ist. */
    if (site.subdomains) {
      const condition = { initiatorDomains: [site.pattern] };
      if (!site.thirdParty) condition.domainType = 'firstParty';
      rules.push({ id: id++, priority, action, condition });
    }
  }

  if (settings.global.mode !== 'off' && globalProfile) {
    rules.push({
      id: id++,
      priority: 10,
      action: {
        type: 'modifyHeaders',
        requestHeaders: headerActionsFor(globalProfile, settings.options)
      },
      condition: { urlFilter: '*' }
    });
  }

  return rules;
}

/**
 * Regel für genau einen Tab. Deckt alles ab, was der Tab lädt - außer der
 * Navigation selbst, die gehört den Domain-Regeln (sonst würde ein Klick auf
 * einen externen Link noch das Profil der alten Seite mitnehmen).
 *
 * @returns Regel-Objekt oder null, wenn der Tab keine eigene Regel braucht
 */
export function tabRuleFor(tabId, decision, profile, options) {
  if (!options.headers) return null;
  if (!Number.isInteger(tabId) || tabId < 0) return null;

  const condition = { tabIds: [tabId], excludedResourceTypes: ['main_frame'] };
  if (decision.scope && decision.scope.thirdParty === false) condition.domainType = 'firstParty';

  const action = decision.active
    ? { type: 'modifyHeaders', requestHeaders: headerActionsFor(profile, options) }
    : { type: 'allow' };

  return { id: TAB_RULE_BASE + tabId, priority: TAB_RULE_PRIORITY, action, condition };
}

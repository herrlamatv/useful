const KEY = 'sites';

function toOrigin(url) {
  try {
    const u = new URL(url);
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return null;
    return u.origin;
  } catch (_) {
    return null;
  }
}

async function list() {
  const data = await chrome.storage.local.get(KEY);
  return new Set(data[KEY] || []);
}

function save(sites) {
  return chrome.storage.local.set({ [KEY]: [...sites] });
}

async function drop(origin) {
  const ids = [`js:${origin}`, `css:${origin}`];
  const found = await chrome.scripting.getRegisteredContentScripts({ ids }).catch(() => []);
  if (found.length) await chrome.scripting.unregisterContentScripts({ ids: found.map((s) => s.id) });
}

async function add(origin) {
  await drop(origin);
  const matches = [`${origin}/*`];
  await chrome.scripting.registerContentScripts([
    {
      id: `js:${origin}`,
      matches,
      js: ['content.js'],
      runAt: 'document_start',
      world: 'MAIN',
      allFrames: true,
      persistAcrossSessions: true,
    },
    {
      id: `css:${origin}`,
      matches,
      css: ['content.css'],
      runAt: 'document_start',
      allFrames: true,
      persistAcrossSessions: true,
    },
  ]);
}

async function toggle(origin, on) {
  const sites = await list();
  if (on) {
    await add(origin);
    sites.add(origin);
  } else {
    await drop(origin);
    sites.delete(origin);
    await chrome.permissions.remove({ origins: [`${origin}/*`] }).catch(() => {});
  }
  await save(sites);
}

async function badge(tabId, url) {
  const origin = toOrigin(url);
  const on = origin ? (await list()).has(origin) : false;
  chrome.action.setBadgeBackgroundColor({ tabId, color: '#1a7f37' }).catch(() => {});
  chrome.action.setBadgeText({ tabId, text: on ? 'ON' : '' }).catch(() => {});
}

// rechte koennen in den chrome-einstellungen entzogen worden sein,
// und registrierungen ueberleben ein reload der extension
async function sync() {
  const sites = await list();
  for (const origin of sites) {
    const ok = await chrome.permissions.contains({ origins: [`${origin}/*`] }).catch(() => false);
    if (ok) {
      await add(origin);
    } else {
      await drop(origin);
      sites.delete(origin);
    }
  }
  await save(sites);

  const keep = new Set([...sites].flatMap((o) => [`js:${o}`, `css:${o}`]));
  const all = await chrome.scripting.getRegisteredContentScripts().catch(() => []);
  const stale = all.filter((s) => !keep.has(s.id)).map((s) => s.id);
  if (stale.length) await chrome.scripting.unregisterContentScripts({ ids: stale });
}

chrome.runtime.onInstalled.addListener(sync);
chrome.runtime.onStartup.addListener(sync);

chrome.permissions.onRemoved.addListener(async (p) => {
  const sites = await list();
  for (const pattern of p.origins || []) {
    const origin = pattern.replace(/\/\*$/, '');
    if (!sites.has(origin)) continue;
    await drop(origin);
    sites.delete(origin);
  }
  await save(sites);
});

chrome.tabs.onUpdated.addListener((tabId, info, tab) => {
  if (info.status || info.url) badge(tabId, tab.url);
});

chrome.tabs.onActivated.addListener(async ({ tabId }) => {
  const tab = await chrome.tabs.get(tabId).catch(() => null);
  if (tab) badge(tabId, tab.url);
});

chrome.runtime.onMessage.addListener((msg, sender, respond) => {
  (async () => {
    if (msg.type === 'state') {
      respond({ on: (await list()).has(msg.origin) });
    } else if (msg.type === 'toggle') {
      await toggle(msg.origin, msg.on);
      if (msg.tabId != null) await badge(msg.tabId, `${msg.origin}/`);
      respond({ on: msg.on });
    }
  })();
  return true;
});

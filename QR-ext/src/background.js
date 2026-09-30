/* QR-ext - by herrlamatv
 * Hintergrunddienst: nur das Rechtsklick-Menü.
 *
 * Ein Klick legt den Text in chrome.storage.session ab und öffnet das Popup
 * (Chrome 127+). Geht das nicht, öffnet sich die große Ansicht im Tab.
 */

import { setPending } from './store.js';
import { MENUS, textFor } from './menu.js';

function buildMenus() {
  chrome.contextMenus.removeAll(() => {
    for (const m of MENUS) {
      chrome.contextMenus.create({ id: m.id, contexts: m.contexts, title: chrome.i18n.getMessage(m.title) });
    }
  });
}

chrome.runtime.onInstalled.addListener(buildMenus);
chrome.runtime.onStartup.addListener(buildMenus);

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const text = textFor(info.menuItemId, info, tab);
  if (!text) return;
  await setPending(text);
  try {
    await chrome.action.openPopup(tab && tab.windowId ? { windowId: tab.windowId } : undefined);
  } catch (e) {
    // älteres Chrome oder kein fokussiertes Fenster -> große Ansicht
    await chrome.tabs.create({ url: chrome.runtime.getURL('ui/qr.html'), index: tab ? tab.index + 1 : undefined });
  }
});

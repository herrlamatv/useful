/* QR-ext - by herrlamatv
 * Einträge im Rechtsklick-Menü (ohne chrome.*, damit testbar).
 */

export const MENUS = [
  { id: 'qr-link', contexts: ['link'], title: 'menuLink' },
  { id: 'qr-selection', contexts: ['selection'], title: 'menuSelection' },
  { id: 'qr-image', contexts: ['image'], title: 'menuImage' },
  { id: 'qr-page', contexts: ['page', 'frame'], title: 'menuPage' }
];

/** Welcher Text gehört zu welchem Menüeintrag? */
export function textFor(menuId, info, tab) {
  switch (menuId) {
    case 'qr-link':
      return info.linkUrl || '';
    case 'qr-selection':
      return (info.selectionText || '').trim();
    case 'qr-image':
      return info.srcUrl || '';
    default:
      return info.frameUrl || info.pageUrl || (tab && tab.url) || '';
  }
}

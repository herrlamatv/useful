const hostEl = document.getElementById('host');
const noteEl = document.getElementById('note');
const rowEl = document.getElementById('row');
const sw = document.getElementById('sw');

let tab, origin;

function note(on) {
  noteEl.textContent = on ? 'right-click and selection unlocked' : 'the page reloads when you switch';
}

// tab vorher holen, sonst ist beim klick die user-geste weg
(async () => {
  [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  try {
    const u = new URL(tab.url);
    if (u.protocol === 'http:' || u.protocol === 'https:') origin = u.origin;
  } catch (_) {}

  if (!origin) {
    hostEl.textContent = 'no website';
    rowEl.classList.add('off');
    return;
  }

  hostEl.textContent = new URL(origin).hostname;
  const res = await chrome.runtime.sendMessage({ type: 'state', origin });
  sw.checked = !!res.on;
  note(sw.checked);
})();

sw.addEventListener('change', async () => {
  const on = sw.checked;
  sw.disabled = true;

  if (on && !(await chrome.permissions.request({ origins: [`${origin}/*`] }))) {
    sw.checked = false;
    sw.disabled = false;
    noteEl.textContent = 'needs access to this site';
    return;
  }

  await chrome.runtime.sendMessage({ type: 'toggle', origin, on, tabId: tab.id });
  await chrome.tabs.reload(tab.id);
  window.close();
});

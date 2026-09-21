/* Chroofer - by herrlamatv : Optionen */

import { send, el, createProfileEditor, segmented, toggle, toggleRow, errorText } from './common.js';
import { profileLabel } from '../src/profiles.js';
import { randomProfileFor, normalizeHost } from '../src/settings.js';
import { t, tParts, setLanguage, applyStatic, languagePreference } from './i18n.js';

const $ = (id) => document.getElementById(id);

let state = null;        // { settings, randomProfile, randomUA, currentSite }
let openHost = null;     // aufgeklappte Website-Regel
const siteEditors = new Map();

applyStatic();

const globalEditor = createProfileEditor((profile) => apply(send('setGlobal', { profile })));

function settings() {
  return state.settings;
}

async function apply(promise) {
  state = await promise;
  if (setLanguage(settings().options.language)) {
    applyStatic();
    globalEditor.retranslate();
    for (const editor of siteEditors.values()) editor.retranslate();
  }
  render();
}

const label = (profile) => profileLabel(profile, t('profile.customUA'));

/* -------------------------------------------------------------------- */
/* Pro Website                                                           */
/* -------------------------------------------------------------------- */

function siteSummary(site) {
  if (!site.enabled) return t('site.summaryPaused');
  if (site.mode === 'off') return t('site.summaryOff');
  const profile = site.mode === 'random' ? randomProfileFor(site.pattern) : site.profile;
  const scope = [site.subdomains ? t('site.scopeSubdomains') : t('site.scopeExact')];
  if (site.redirects) scope.push(t('site.scopeRedirects'));
  if (!site.thirdParty) scope.push(t('site.scopeNoThirdParty'));
  return label(profile) + ' · ' + scope.join(', ');
}

function siteChip(site) {
  if (!site.enabled) return el('span', { class: 'chip off' }, t('site.chipPause'));
  if (site.mode === 'off') return el('span', { class: 'chip off' }, t('site.chipOff'));
  if (site.mode === 'random') return el('span', { class: 'chip site' }, t('site.chipRandom'));
  return el('span', { class: 'chip site' }, t('site.chipProfile'));
}

function patchSite(pattern, patch) {
  return apply(send('setSite', { host: pattern, patch }));
}

function siteCard(site) {
  const open = site.pattern === openHost;

  const top = el('div', { class: 'site-top' }, [
    toggle(site.enabled, (v) => patchSite(site.pattern, { enabled: v }), {
      title: site.enabled ? t('options.pauseRule') : t('options.resumeRule')
    }),
    el('div', { class: 'grow', style: 'min-width:0' }, [
      el('div', { class: 'site-name mono' }, site.pattern),
      el('div', { class: 'site-sub' }, siteSummary(site))
    ]),
    siteChip(site),
    el(
      'button',
      {
        class: 'btn small',
        type: 'button',
        onclick: () => {
          openHost = open ? null : site.pattern;
          renderSites();
        }
      },
      open ? t('options.done') : t('options.edit')
    ),
    el(
      'button',
      {
        class: 'icon',
        type: 'button',
        title: t('options.deleteRule'),
        onclick: () => {
          if (openHost === site.pattern) openHost = null;
          siteEditors.delete(site.pattern);
          apply(send('removeSite', { host: site.pattern }));
        }
      },
      '✕'
    )
  ]);

  const card = el('div', { class: 'site-card' + (open ? ' open' : '') + (site.enabled ? '' : ' paused') }, [top]);
  if (!open) return card;

  const detail = el('div', { class: 'site-detail' });

  detail.append(
    segmented(
      [
        ['spoof', t('site.modeProfile')],
        ['random', t('site.modeRandom')],
        ['off', t('site.modeOff')]
      ],
      site.mode,
      (mode) => patchSite(site.pattern, { mode }),
      !site.enabled
    ),
    el('div', { class: 'spacer' })
  );

  if (site.mode === 'spoof') {
    let editor = siteEditors.get(site.pattern);
    if (!editor) {
      editor = createProfileEditor((profile) => patchSite(site.pattern, { profile }));
      siteEditors.set(site.pattern, editor);
    }
    editor.set(site.profile);
    detail.append(editor.el);
  } else if (site.mode === 'random') {
    const name = el('strong', {}, label(randomProfileFor(site.pattern)));
    detail.append(el('div', { class: 'note' }, tParts('site.randomNote', 'profile', name)));
  } else {
    detail.append(el('div', { class: 'note' }, t('site.offNote')));
  }

  const note = el('input', {
    type: 'text',
    value: site.note || '',
    placeholder: t('options.notePlaceholder'),
    onchange: (ev) => patchSite(site.pattern, { note: ev.target.value })
  });

  detail.append(
    el('div', { class: 'spacer' }),
    el('div', { class: 'label', style: 'margin-bottom:8px' }, t('scope.title')),
    el('div', { class: 'list' }, [
      toggleRow(
        t('scope.subdomains'),
        t('scope.subdomainsLong', { pattern: site.pattern }),
        site.subdomains,
        (v) => patchSite(site.pattern, { subdomains: v })
      ),
      toggleRow(t('scope.redirects'), t('scope.redirectsDesc'), site.redirects, (v) =>
        patchSite(site.pattern, { redirects: v })
      ),
      toggleRow(t('scope.thirdParty'), t('scope.thirdPartyDesc'), site.thirdParty, (v) =>
        patchSite(site.pattern, { thirdParty: v })
      )
    ]),
    el('div', { class: 'spacer' }),
    el('label', { class: 'field' }, [t('options.note'), note])
  );

  card.append(detail);
  return card;
}

function renderSites() {
  const list = $('siteList');
  const sites = settings().sites.slice().sort((a, b) => a.pattern.localeCompare(b.pattern));
  list.replaceChildren();

  if (!sites.length) {
    list.append(el('div', { class: 'empty' }, t('options.emptySites')));
    return;
  }

  for (const site of sites) list.append(siteCard(site));
}

/* -------------------------------------------------------------------- */
/* Global                                                                */
/* -------------------------------------------------------------------- */

function renderGlobal() {
  const g = settings().global;

  $('globalModes').replaceChildren(
    segmented(
      [
        ['spoof', t('global.modeProfile')],
        ['random', t('global.modeRandom')],
        ['off', t('global.modeOff')]
      ],
      g.mode,
      (mode) => apply(send('setGlobal', { mode }))
    )
  );

  const detail = $('globalDetail');
  detail.replaceChildren(el('div', { class: 'spacer' }));

  if (g.mode === 'spoof') {
    globalEditor.set(g.profile);
    detail.append(globalEditor.el);
  } else if (g.mode === 'random') {
    detail.append(
      el('div', { class: 'row between' }, [
        el('span', { class: 'hint' }, t('global.randomHint')),
        el('button', { class: 'btn small', type: 'button', onclick: () => apply(send('reroll')) }, t('global.reroll'))
      ]),
      el('div', { style: 'height:8px' }),
      el('div', { class: 'mono ua-box' }, label(state.randomProfile) + '\n' + state.randomUA)
    );
  } else {
    detail.append(el('div', { class: 'note' }, t('global.offNote')));
  }
}

/* -------------------------------------------------------------------- */
/* Erweiterte Optionen                                                   */
/* -------------------------------------------------------------------- */

function saveOptions(patch) {
  const next = structuredClone(settings());
  Object.assign(next.options, patch);
  return apply(send('saveSettings', { settings: next }));
}

function numberField(labelText, value, min, max, step, onCommit) {
  return el('label', { class: 'field' }, [
    labelText,
    el('input', {
      type: 'number',
      value,
      min,
      max,
      step: step || 1,
      onchange: (ev) => onCommit(Number(ev.target.value))
    })
  ]);
}

function renderOptions() {
  const o = settings().options;

  $('optCore').replaceChildren(
    toggleRow(t('opts.headers'), t('opts.headersDesc'), o.headers, (v) => saveOptions({ headers: v })),
    toggleRow(t('opts.navigator'), t('opts.navigatorDesc'), o.navigator, (v) => saveOptions({ navigator: v })),
    toggleRow(t('opts.hideWebdriver'), t('opts.hideWebdriverDesc'), o.hideWebdriver, (v) =>
      saveOptions({ hideWebdriver: v })
    ),
    toggleRow(t('opts.highEntropy'), t('opts.highEntropyDesc'), o.highEntropyHeaders, (v) =>
      saveOptions({ highEntropyHeaders: v })
    ),
    toggleRow(t('opts.badge'), t('opts.badgeDesc'), o.badge, (v) => saveOptions({ badge: v }))
  );

  $('optExtra').replaceChildren(
    toggleRow(t('opts.languages'), t('opts.languagesDesc'), o.languages, (v) => saveOptions({ languages: v })),
    toggleRow(t('opts.timezone'), t('opts.timezoneDesc'), o.timezone, (v) => saveOptions({ timezone: v })),
    toggleRow(t('opts.screen'), t('opts.screenDesc'), o.screen, (v) => saveOptions({ screen: v })),
    toggleRow(t('opts.hardware'), t('opts.hardwareDesc'), o.hardware, (v) => saveOptions({ hardware: v })),
    toggleRow(t('opts.hideEngineTraces'), t('opts.hideEngineTracesDesc'), o.hideEngineTraces, (v) =>
      saveOptions({ hideEngineTraces: v })
    )
  );

  $('optNote').replaceChildren(...tParts('opts.note', 'navigator', el('span', { class: 'mono' }, 'navigator')));

  const values = $('optValues');
  values.replaceChildren();
  const blocks = [];

  if (o.languages) {
    blocks.push(
      el('label', { class: 'field' }, [
        t('opts.languagesLabel'),
        el('input', {
          type: 'text',
          value: o.languagesValue,
          placeholder: 'de-DE,de,en-US,en',
          onchange: (ev) => saveOptions({ languagesValue: ev.target.value.trim() })
        })
      ])
    );
  }

  if (o.timezone) {
    blocks.push(
      el('label', { class: 'field' }, [
        t('opts.timezoneLabel'),
        el('input', {
          type: 'text',
          value: o.timezoneValue,
          placeholder: 'Europe/Berlin',
          onchange: (ev) => saveOptions({ timezoneValue: ev.target.value.trim() })
        })
      ])
    );
  }

  if (o.screen) {
    const set = (key) => (value) => saveOptions({ screenValue: Object.assign({}, o.screenValue, { [key]: value }) });
    blocks.push(
      el('div', {}, [
        el('div', { class: 'label', style: 'margin-bottom:6px' }, t('opts.screenLabel')),
        el('div', { class: 'grid', style: 'grid-template-columns:repeat(4,minmax(0,1fr))' }, [
          numberField(t('opts.width'), o.screenValue.width, 320, 8192, 1, set('width')),
          numberField(t('opts.height'), o.screenValue.height, 240, 8192, 1, set('height')),
          numberField(t('opts.colorDepth'), o.screenValue.colorDepth, 8, 48, 1, set('colorDepth')),
          numberField(t('opts.dpr'), o.screenValue.dpr, 1, 4, 0.25, set('dpr'))
        ])
      ])
    );
  }

  if (o.hardware) {
    const set = (key) => (value) =>
      saveOptions({ hardwareValue: Object.assign({}, o.hardwareValue, { [key]: value }) });
    blocks.push(
      el('div', {}, [
        el('div', { class: 'label', style: 'margin-bottom:6px' }, t('opts.hardwareLabel')),
        el('div', { class: 'grid' }, [
          numberField(t('opts.cores'), o.hardwareValue.cores, 1, 64, 1, set('cores')),
          numberField(t('opts.memory'), o.hardwareValue.memory, 0.25, 64, 0.25, set('memory'))
        ])
      ])
    );
  }

  for (const block of blocks) values.append(el('div', { class: 'spacer' }), block);
}

/* -------------------------------------------------------------------- */

function renderLanguage() {
  $('langSwitch').replaceChildren(
    segmented(
      [
        ['auto', t('lang.auto'), t('lang.autoTitle')],
        ['de', 'DE', t('lang.de')],
        ['en', 'EN', t('lang.en')]
      ],
      languagePreference(),
      (value) => saveOptions({ language: value })
    )
  );
}

function render() {
  const s = settings();

  $('master').checked = s.enabled;
  $('masterLabel').textContent = s.enabled ? t('app.on') : t('app.off');
  document.body.classList.toggle('dimmed', !s.enabled);

  const current = state.currentSite;
  $('addCurrent').disabled = !current;
  $('currentHint').textContent = current ? current.host : t('options.noCurrentSite');

  renderLanguage();
  renderSites();
  renderGlobal();
  renderOptions();
}

/* ----------------------------- Verdrahten ---------------------------- */

$('master').addEventListener('change', (ev) => apply(send('setEnabled', { value: ev.target.checked })));

async function addHost(value) {
  $('addError').textContent = '';
  try {
    await apply(send('setSite', { host: value, patch: { enabled: true, mode: 'spoof' } }));
    openHost = normalizeHost(value);
    renderSites();
    $('newHost').value = '';
  } catch (err) {
    $('addError').textContent = errorText(err);
  }
}

$('addCurrent').addEventListener('click', () => {
  if (state.currentSite) addHost(state.currentSite.host);
});

$('addSite').addEventListener('click', () => {
  const value = $('newHost').value.trim();
  if (value) addHost(value);
});

$('newHost').addEventListener('keydown', (ev) => {
  if (ev.key === 'Enter') $('addSite').click();
});

$('export').addEventListener('click', () => {
  const blob = new Blob([JSON.stringify(settings(), null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = el('a', { href: url, download: 'chroofer-settings.json' });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  $('ioStatus').textContent = t('io.exported');
});

$('importBtn').addEventListener('click', () => $('importFile').click());

$('importFile').addEventListener('change', async (ev) => {
  const file = ev.target.files && ev.target.files[0];
  ev.target.value = '';
  if (!file) return;
  try {
    const parsed = JSON.parse(await file.text());
    siteEditors.clear();
    openHost = null;
    await apply(send('saveSettings', { settings: parsed }));
    $('ioStatus').textContent = t('io.imported');
  } catch (err) {
    $('ioStatus').textContent = t('io.importFailed', { error: errorText(err) });
  }
});

$('reset').addEventListener('click', async () => {
  openHost = null;
  siteEditors.clear();
  await apply(send('reset'));
  $('ioStatus').textContent = t('io.wasReset');
});

apply(send('getState')).catch((err) => {
  document.body.prepend(el('div', { class: 'note' }, t('error.generic', { message: errorText(err) })));
});

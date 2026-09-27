/* dlplist - by herrlamatv
 * Einstellungs-Panels der Export-Seite: Vorlagen, Format pro Kategorie,
 * Allgemein, Umgebung, Ausgabe, Erweiterung.
 *
 * Jede Funktion baut ihr Panel komplett neu. Textfelder melden sich mit
 * rerender=false, damit der Fokus beim Tippen nicht verloren geht.
 */

import { t } from './i18n.js';
import { el, row, toggle, toggleRow, segmented, select, input, field } from './common.js';
import { CATS } from '../src/links.js';
import {
  QUALITIES,
  TEMPLATES,
  BROWSERS,
  BUILTIN_PRESETS,
  presetMatches,
  COMPAT_SORT
} from '../src/command.js';

const CHROMIUM = new Set(['chrome', 'edge', 'brave', 'opera', 'vivaldi', 'chromium']);

function panel(title, desc, bodies, id) {
  return el('div', { class: 'panel', id: id || null }, [
    el('div', { class: 'panel-head' }, [el('h2', {}, title), desc ? el('p', {}, desc) : null]),
    ...bodies.filter(Boolean).map((b) => el('div', { class: 'panel-body' }, b))
  ]);
}

function label(text, extra) {
  return el('div', { class: 'label', style: 'margin-bottom: 8px' + (extra ? ';' + extra : '') }, text);
}

function list(rows) {
  return el('div', { class: 'list' }, rows.filter(Boolean));
}

/* ------------------------------ Vorlagen ----------------------------- */

export function presetsPanel(api) {
  const cfg = api.cfg();
  const pills = BUILTIN_PRESETS.map((p) =>
    el(
      'button',
      {
        class: 'pill',
        type: 'button',
        title: t('preset.' + p.id + 'Desc'),
        'aria-pressed': String(presetMatches(cfg, p.format)),
        onclick: () => api.applyFormat(p.format)
      },
      t('preset.' + p.id)
    )
  );
  for (const p of api.presets()) {
    const x = el('span', { class: 'x', role: 'button', title: t('presets.delete') }, '✕');
    x.addEventListener('click', (e) => {
      e.stopPropagation();
      api.deletePreset(p.id);
    });
    pills.push(
      el(
        'button',
        {
          class: 'pill',
          type: 'button',
          'aria-pressed': String(presetMatches(cfg, p.format)),
          onclick: () => api.applyFormat(p.format)
        },
        [p.name, x]
      )
    );
  }

  const name = el('input', { type: 'text', placeholder: t('presets.namePh'), maxlength: '40' });
  const save = () => {
    const value = name.value.trim();
    if (!value) return name.focus();
    api.savePreset(value);
  };
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') save();
  });

  return panel(t('presets.title'), t('presets.desc'), [
    el('div', { class: 'pills' }, pills),
    el('div', { class: 'add-row' }, [name, el('button', { class: 'btn small', type: 'button', onclick: save }, t('presets.save'))])
  ]);
}

/* ------------------------- Format pro Kategorie ---------------------- */

export function formatPanel(api) {
  const cfg = api.cfg();
  const qualityEntries = QUALITIES.map((q) => [q, t('quality.' + q)]);

  const cards = CATS.map((cat) => {
    const cc = cfg.cats[cat];
    const countChip = el('span', { class: 'chip' }, String(api.counts()[cat] || 0));
    api.countEls[cat] = countChip;

    const body = [
      el('div', { class: 'field-grid' }, [
        field(t('format.quality'), select(qualityEntries, cc.quality, (v) => api.set(`cats.${cat}.quality`, v))),
        field(
          t('format.folder'),
          input(cc.folder, (v) => api.set(`cats.${cat}.folder`, v, false), { placeholder: t('format.folderPh') })
        )
      ])
    ];

    if (cat === 'lists') {
      const limit = el('input', { type: 'number', min: '0', step: '1', style: 'width: 90px' });
      limit.value = String(cc.limit || 0);
      limit.addEventListener('change', () => api.set('cats.lists.limit', Math.max(0, parseInt(limit.value, 10) || 0), false));
      body.push(
        list([
          toggleRow(t('format.perList'), t('format.perListDesc'), cc.perList, (v) => api.set('cats.lists.perList', v)),
          row(t('format.limit'), t('format.limitDesc'), limit),
          toggleRow(t('format.breakOn'), t('format.breakOnDesc'), cc.breakOnExisting, (v) => api.set('cats.lists.breakOnExisting', v))
        ])
      );
    }

    return el('div', { class: 'cat-card' + (cc.include ? '' : ' off') }, [
      el('div', { class: 'cat-top' }, [
        el('h3', {}, t('cat.' + cat)),
        countChip,
        toggle(cc.include, (v) => api.set(`cats.${cat}.include`, v), { title: t('format.include') })
      ]),
      el('div', { class: 'hint' }, t('format.' + cat + 'Desc')),
      ...body
    ]);
  });

  return panel(t('format.title'), t('format.desc'), [el('div', { class: 'cat-grid' }, cards)]);
}

/* ------------------------------ Allgemein ---------------------------- */

export function generalPanel(api) {
  const cfg = api.cfg();
  const x = cfg.extras;

  const templateEntries = Object.keys(TEMPLATES)
    .map((k) => [k, t('template.' + k)])
    .concat([['custom', t('template.custom')]]);
  const templateHint = el('div', { class: 'mono hint', style: 'margin-top: 6px' }, cfg.template === 'custom' ? '' : TEMPLATES[cfg.template]);

  const format = [
    el('div', { class: 'field-grid' }, [
      field(
        t('general.container'),
        segmented(
          [
            ['mp4', 'MP4'],
            ['mkv', 'MKV'],
            ['original', t('general.original')]
          ],
          cfg.container,
          (v) => api.set('container', v)
        )
      ),
      el('div', {}, [
        field(t('general.template'), select(templateEntries, cfg.template, (v) => api.set('template', v))),
        cfg.template === 'custom'
          ? el('div', { style: 'margin-top: 6px' }, input(cfg.templateCustom, (v) => api.set('templateCustom', v, false), {
              placeholder: TEMPLATES.default,
              class: 'mono'
            }))
          : templateHint
      ])
    ]),
    el('div', { class: 'spacer' }),
    list([
      toggleRow(t('general.recode'), t('general.recodeDesc'), cfg.recode, (v) => api.set('recode', v), {
        disabled: cfg.container !== 'mp4'
      }),
      toggleRow(t('general.compat'), t('general.compatDesc', { sort: COMPAT_SORT }), cfg.compat, (v) => api.set('compat', v))
    ])
  ];

  const cookies = [
    label(t('general.cookies')),
    segmented(
      [
        ['none', t('general.cookiesNone')],
        ['file', t('general.cookiesFile')],
        ['browser', t('general.cookiesBrowser')]
      ],
      cfg.cookies,
      (v) => api.set('cookies', v)
    ),
    cfg.cookies === 'file'
      ? el('div', { style: 'margin-top: 10px' }, [
          field(t('general.cookieFile'), input(cfg.cookieFile, (v) => api.set('cookieFile', v, false), { placeholder: 'cookies.txt' })),
          el('div', { class: 'hint', style: 'margin-top: 6px' }, t('general.cookieFileHint'))
        ])
      : null,
    cfg.cookies === 'browser'
      ? el('div', { style: 'margin-top: 10px' }, [
          field(
            t('general.cookieBrowser'),
            select(
              BROWSERS.map((b) => [b, b[0].toUpperCase() + b.slice(1)]),
              cfg.cookieBrowser,
              (v) => api.set('cookieBrowser', v)
            )
          ),
          CHROMIUM.has(cfg.cookieBrowser) ? el('div', { class: 'note', style: 'margin-top: 8px' }, t('warn.chromiumCookies', { browser: cfg.cookieBrowser })) : null
        ])
      : el('div', { class: 'hint', style: 'margin-top: 8px' }, t('general.cookiesHint'))
  ];

  const subsRow = toggleRow(t('extras.subs'), t('extras.subsDesc'), x.subs, (v) => api.set('extras.subs', v));
  const subsDetail = x.subs
    ? el('div', { class: 'item' }, [
        el('div', { class: 'txt' }, [
          field(t('extras.subLangs'), input(x.subLangs, (v) => api.set('extras.subLangs', v, false), { placeholder: 'de,en', class: 'mono' }))
        ]),
        el('div', { class: 'row' }, [el('span', { class: 'hint' }, t('extras.autoSubs')), toggle(x.autoSubs, (v) => api.set('extras.autoSubs', v))])
      ])
    : null;

  const extras = [
    label(t('general.extras')),
    list([
      toggleRow(t('extras.thumbnail'), t('extras.thumbnailDesc'), x.thumbnail, (v) => api.set('extras.thumbnail', v)),
      toggleRow(t('extras.metadata'), t('extras.metadataDesc'), x.metadata, (v) => api.set('extras.metadata', v)),
      toggleRow(t('extras.chapters'), t(x.metadata ? 'extras.chaptersIncluded' : 'extras.chaptersDesc'), x.chapters || x.metadata, (v) => api.set('extras.chapters', v), {
        disabled: x.metadata
      }),
      subsRow,
      subsDetail,
      row(
        t('extras.sponsorblock'),
        t('extras.sponsorblockDesc'),
        segmented(
          [
            ['off', t('extras.sbOff')],
            ['mark', t('extras.sbMark')],
            ['remove', t('extras.sbRemove')]
          ],
          x.sponsorblock,
          (v) => api.set('extras.sponsorblock', v)
        )
      ),
      toggleRow(t('extras.archive'), t('extras.archiveDesc'), x.archive, (v) => api.set('extras.archive', v)),
      toggleRow(t('extras.sleep'), t('extras.sleepDesc'), x.sleep, (v) => api.set('extras.sleep', v)),
      toggleRow(t('extras.restrict'), t('extras.restrictDesc'), x.restrict, (v) => api.set('extras.restrict', v)),
      toggleRow(t('extras.simulate'), t('extras.simulateDesc'), x.simulate, (v) => api.set('extras.simulate', v))
    ]),
    el('div', { class: 'spacer' }),
    el('div', { class: 'field-grid' }, [
      field(t('extras.rate'), input(x.rate, (v) => api.set('extras.rate', v, false), { placeholder: t('extras.ratePh') })),
      field(
        t('extras.fragments'),
        (() => {
          const n = el('input', { type: 'number', min: '0', max: '32', step: '1' });
          n.value = String(x.fragments || 0);
          n.addEventListener('change', () => api.set('extras.fragments', parseInt(n.value, 10) || 0, false));
          return n;
        })()
      ),
      field(t('extras.extraArgs'), input(x.extraArgs, (v) => api.set('extras.extraArgs', v, false), { placeholder: '--no-part --live-from-start', class: 'mono' }), {
        class: 'field full'
      })
    ])
  ];

  return panel(t('general.title'), t('general.desc'), [format, cookies, extras]);
}

/* ------------------------------ Umgebung ----------------------------- */

export function envPanel(api) {
  const cfg = api.cfg();
  const env = cfg.env;
  return panel(t('env.title'), t('env.desc'), [
    [
      label(t('env.location')),
      segmented(
        [
          ['fixed', t('env.fixed')],
          ['here', t('env.here')]
        ],
        env.location,
        (v) => api.set('env.location', v)
      ),
      el('div', { class: 'hint', style: 'margin-top: 8px' }, t(env.location === 'fixed' ? 'env.fixedDesc' : 'env.hereDesc')),
      env.location === 'fixed'
        ? el('div', { style: 'margin-top: 10px' }, [
            field(t('env.path'), input(env.path, (v) => api.set('env.path', v, false), { placeholder: '%USERPROFILE%\\Desktop\\dw', class: 'mono' })),
            el('div', { class: 'hint', style: 'margin-top: 6px' }, t('env.pathHint'))
          ])
        : null
    ],
    [
      el('div', { class: 'field-grid' }, [
        field(t('env.exe'), input(env.exe, (v) => api.set('env.exe', v, false), { placeholder: t('env.exePh'), class: 'mono' })),
        field(t('env.saveTo'), input(env.saveTo, (v) => api.set('env.saveTo', v, false), { placeholder: t('env.saveToPh'), class: 'mono' })),
        field(t('env.ffmpeg'), input(env.ffmpeg, (v) => api.set('env.ffmpeg', v, false), { placeholder: t('env.ffmpegPh'), class: 'mono' })),
        field(
          t('env.js'),
          select(
            [
              ['', t('env.jsDefault')],
              ['node', 'Node.js'],
              ['quickjs', 'QuickJS'],
              ['bun', 'Bun']
            ],
            env.jsRuntime,
            (v) => api.set('env.jsRuntime', v)
          )
        )
      ]),
      el('div', { class: 'hint', style: 'margin-top: 8px' }, t('env.jsHint'))
    ]
  ]);
}

/* ------------------------------- Ausgabe ----------------------------- */

const FILE_OPTIONS = {
  bat: ['pause', 'check', 'update', 'consoleTitle', 'openFolder', 'shutdown'],
  ps1: ['pause', 'check', 'update', 'consoleTitle', 'openFolder', 'shutdown'],
  sh: ['pause', 'check', 'update'],
  txt: ['update'],
  links: []
};

export function outputPanel(api) {
  const cfg = api.cfg();
  const out = cfg.output;
  const opts = FILE_OPTIONS[out.type];

  const typeBlock = [
    label(t('output.type')),
    segmented(
      [
        ['txt', t('output.txt')],
        ['bat', '.bat'],
        ['ps1', '.ps1'],
        ['sh', '.sh'],
        ['links', t('output.links')]
      ],
      out.type,
      (v) => api.set('output.type', v)
    ),
    el('div', { class: 'hint', style: 'margin-top: 8px' }, t('output.hint_' + out.type)),
    out.type === 'txt'
      ? el('div', { style: 'margin-top: 12px' }, [
          label(t('output.shell')),
          segmented(
            [
              ['cmd', 'CMD'],
              ['ps', 'PowerShell'],
              ['sh', 'Bash']
            ],
            out.shell,
            (v) => api.set('output.shell', v)
          )
        ])
      : null,
    out.type !== 'links'
      ? el('div', { style: 'margin-top: 12px' }, [
          label(t('output.mode')),
          segmented(
            [
              ['batch', t('output.batch')],
              ['each', t('output.each')]
            ],
            out.mode,
            (v) => api.set('output.mode', v)
          ),
          el('div', { class: 'hint', style: 'margin-top: 8px' }, t(out.mode === 'each' ? 'output.eachDesc' : 'output.batchDesc'))
        ])
      : null
  ];

  const fileBlock = opts.length
    ? [
        label(t('output.fileOptions')),
        list(opts.map((k) => toggleRow(t('bat.' + k), t('bat.' + k + 'Desc'), cfg.bat[k], (v) => api.set('bat.' + k, v))))
      ]
    : null;

  const afterBlock = [
    field(t('output.name'), input(out.name, (v) => api.set('output.name', v, false), { placeholder: 'dlplist-{date}-{time}', class: 'mono' })),
    el('div', { class: 'hint', style: 'margin-top: 6px' }, t('output.nameHint')),
    el('div', { class: 'spacer' }),
    label(t('output.after')),
    segmented(
      [
        ['mark', t('output.afterMark')],
        ['clear', t('output.afterClear')],
        ['none', t('output.afterNone')]
      ],
      out.after,
      (v) => api.set('output.after', v)
    ),
    el('div', { class: 'spacer' }),
    list([
      toggleRow(t('output.onlyNew'), t('output.onlyNewDesc', { n: api.counts().exported }), cfg.onlyNew, (v) => api.set('onlyNew', v)),
      toggleRow(t('output.saveAs'), t('output.saveAsDesc'), out.saveAs, (v) => api.set('output.saveAs', v))
    ])
  ];

  return panel(t('output.title'), t('output.desc'), [typeBlock, fileBlock, afterBlock]);
}

/* ------------------------------ Erweiterung -------------------------- */

export function extensionPanel(api) {
  const s = api.settings();
  return panel(t('ext.title'), t('ext.desc'), [
    list([
      toggleRow(t('ext.toast'), t('ext.toastDesc'), s.toast, (v) => api.patchSettings({ toast: v })),
      row(
        t('ext.titles'),
        t('ext.titlesDesc'),
        el('div', { class: 'row' }, [
          el('button', { class: 'btn small', type: 'button', onclick: () => api.refetchTitles() }, t('ext.titlesRetry')),
          toggle(s.titles, (v) => api.patchSettings({ titles: v }))
        ])
      ),
      toggleRow(t('ext.badge'), t('ext.badgeDesc'), s.badge, (v) => api.patchSettings({ badge: v })),
      row(
        t('ext.shortcut'),
        t('ext.shortcutDesc'),
        el('button', { class: 'btn small', type: 'button', onclick: () => api.openShortcuts() }, t('ext.shortcutOpen'))
      )
    ])
  ]);
}

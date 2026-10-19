/* ai chat exporter - by herrlamatv  PDF-Seite
 *
 * Holt den Auftrag vom Service Worker, baut das Dokument (formatiert, Reintext
 * oder zusammengesetzte Screenshots) und macht daraus ein PDF:
 *   direkt  - chrome.debugger -> Page.printToPDF (als Datenstrom), Download, Tab zu
 *   Druck   - Chromes Druckdialog („Als PDF speichern“)
 * Klappt „direkt“ nicht, geht es automatisch mit dem Druckdialog weiter.
 */

import { send, errorText, downloadBlob } from './common.js';
import { t, setLanguage, applyStatic } from './i18n.js';
import { renderBody, documentCss } from '../src/export/html.js';

const $ = (id) => document.getElementById(id);
const jobId = new URLSearchParams(location.search).get('job');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const PAPER = {
  a4: { mm: [210, 297], inch: [8.27, 11.69], css: 'A4' },
  letter: { mm: [215.9, 279.4], inch: [8.5, 11], css: 'letter' }
};
const SHOT_MARGIN = 8; // mm

applyStatic();

function say(text, kind = '') {
  $('barText').textContent = text;
  $('barText').className = 'text' + (kind ? ' ' + kind : '');
}

function addStyle(css) {
  const style = document.createElement('style');
  style.textContent = css;
  document.head.append(style);
}

/* ------------------------------ Aufbauen ------------------------------ */

function buildDocument(job) {
  addStyle(documentCss({ mode: job.mode, fontSize: job.fontSize, paper: job.paper, print: job.method === 'print' }));
  $('content').innerHTML = renderBody(job.conv, {
    labels: job.labels,
    include: job.include,
    mode: job.mode,
    print: true,
    exportedAt: new Date(job.exportedAt)
  });
}

const loadImage = (src) =>
  new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('image'));
    img.src = src;
  });

/**
 * Screenshots zu Seiten zusammensetzen: jede Aufnahme deckt einen Ausschnitt
 * des Chats ab (top in CSS-Pixeln), jede PDF-Seite ist ein Streifen fester
 * Höhe. Pro Seite werden die passenden Teile der Aufnahmen hineingezeichnet.
 */
async function buildShots(job) {
  say(t('render.stitching'));
  const paper = PAPER[job.paper] || PAPER.a4;
  const contentW = paper.mm[0] - 2 * SHOT_MARGIN;
  const contentH = paper.mm[1] - 2 * SHOT_MARGIN;
  addStyle(
    '@page{size:' + paper.css + ';margin:' + SHOT_MARGIN + 'mm}' +
    'html{-webkit-print-color-adjust:exact;print-color-adjust:exact}' +
    'body{margin:0;background:#ebe5d7}' +
    '.shot-page{display:block;width:' + contentW + 'mm;height:' + contentH + 'mm;margin:18px auto;background:#fff;box-shadow:0 10px 26px rgba(74,62,40,.12)}' +
    '@media print{body{background:#fff}.shot-page{margin:0;box-shadow:none}.shot-page+.shot-page{break-before:page}}'
  );

  const images = [];
  for (let i = 0; i < job.shots.length; i++) images.push(await loadImage(await send('pdf:shot', { id: jobId, index: i })));
  if (!images.length) throw new Error('empty');

  const rect = job.rect;
  const scale = images[0].naturalWidth / job.viewport.width;
  const crop = { x: rect.left * scale, y: rect.top * scale, w: rect.width * scale, h: rect.height * scale };
  const base = job.shots[0].top;
  const tops = job.shots.map((s) => s.top - base);
  const total = tops[tops.length - 1] + rect.height;
  const pageH = rect.width * (contentH / contentW); // CSS-Pixel pro Seite
  const pages = Math.max(1, Math.ceil(total / pageH - 0.01));

  for (let p = 0; p < pages; p++) {
    const canvas = document.createElement('canvas');
    canvas.width = Math.round(crop.w);
    canvas.height = Math.round(pageH * scale);
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    for (let i = 0; i < images.length; i++) {
      const from = Math.max(p * pageH, tops[i]);
      const to = Math.min((p + 1) * pageH, tops[i] + rect.height);
      if (to <= from) continue;
      ctx.drawImage(images[i], crop.x, crop.y + (from - tops[i]) * scale, crop.w, (to - from) * scale, 0, (from - p * pageH) * scale, crop.w, (to - from) * scale);
    }
    const blob = await new Promise((r) => canvas.toBlob(r, 'image/jpeg', 0.9));
    const img = document.createElement('img');
    img.className = 'shot-page';
    img.alt = '';
    img.src = URL.createObjectURL(blob);
    $('content').append(img);
  }
}

/** Warten, bis Bilder und Schriften da sind (höchstens 15 s). */
async function settle() {
  const imgs = Array.from(document.images).filter((img) => !img.complete);
  if (imgs.length) say(t('render.images'));
  await Promise.race([
    Promise.all(
      imgs.map(
        (img) =>
          new Promise((r) => {
            img.addEventListener('load', r, { once: true });
            img.addEventListener('error', r, { once: true });
          })
      )
    ),
    sleep(15000)
  ]);
  if (document.fonts && document.fonts.ready) await document.fonts.ready;
  await sleep(200);
}

/* ------------------------------- PDF ---------------------------------- */

function base64Bytes(b64) {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

async function directPdf(job) {
  const tab = await chrome.tabs.getCurrent();
  const target = { tabId: tab.id };
  const paper = PAPER[job.paper] || PAPER.a4;
  const params = {
    printBackground: true,
    preferCSSPageSize: true,
    paperWidth: paper.inch[0],
    paperHeight: paper.inch[1],
    transferMode: 'ReturnAsStream'
  };
  if (job.mode !== 'shot') {
    Object.assign(params, {
      displayHeaderFooter: true,
      headerTemplate: '<span></span>',
      footerTemplate:
        '<div style="width:100%;text-align:center;font:8px system-ui,sans-serif;color:#a29a8a"><span class="pageNumber"></span> / <span class="totalPages"></span></div>'
    });
  }
  await chrome.debugger.attach(target, '1.3');
  let blob;
  try {
    const res = await chrome.debugger.sendCommand(target, 'Page.printToPDF', params);
    const chunks = [];
    for (;;) {
      const part = await chrome.debugger.sendCommand(target, 'IO.read', { handle: res.stream, size: 1 << 20 });
      if (part.data) chunks.push(part.base64Encoded ? base64Bytes(part.data) : new TextEncoder().encode(part.data));
      if (part.eof) break;
    }
    await chrome.debugger.sendCommand(target, 'IO.close', { handle: res.stream }).catch(() => {});
    blob = new Blob(chunks, { type: 'application/pdf' });
  } finally {
    await chrome.debugger.detach(target).catch(() => {});
  }
  await downloadBlob(blob, job.filename, { saveAs: job.saveAs });
}

function printDialog() {
  $('printBtn').hidden = false;
  $('printBtn').onclick = () => window.print();
  setTimeout(() => window.print(), 300);
}

async function main() {
  const job = await send('pdf:job', { id: jobId });
  if (setLanguage(job.lang)) applyStatic();
  document.title = (job.title || 'Chat') + ' – AI Chat Export';
  say(t('render.working'));

  if (job.mode === 'shot') await buildShots(job);
  else buildDocument(job);
  await settle();

  if (job.method === 'print') {
    say(t('render.printHint'));
    printDialog();
    send('pdf:done', { id: jobId }).catch(() => {});
    return;
  }
  try {
    say(t('render.working'));
    await directPdf(job);
    say(t('render.done'));
    send('pdf:done', { id: jobId }).catch(() => {});
    setTimeout(() => window.close(), 1500);
  } catch (err) {
    say(t('render.fallback', { message: err.message || String(err) }), 'error');
    /* Hintergrund-Tab nach vorn holen, sonst sieht man den Druckdialog nicht. */
    const tab = await chrome.tabs.getCurrent();
    await chrome.tabs.update(tab.id, { active: true }).catch(() => {});
    printDialog();
  }
}

main().catch((err) => say(errorText(err), 'error'));

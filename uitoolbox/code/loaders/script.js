I18N.add({
  en: {
    docTitle: 'CSS Loader Gallery', back: '← Overview', title: 'CSS Loaders', eyebrow: 'Gallery',
    intro: 'Pure CSS loading animations. Click a card to copy HTML + CSS.',
    colour: 'Colour', copied: 'Copied', copiedName: '“{name}” copied', copyFail: 'Copying not possible'
  },
  de: {
    docTitle: 'CSS Loader Galerie', back: '← Übersicht', title: 'CSS Loader', eyebrow: 'Galerie',
    intro: 'Reine CSS-Ladeanimationen. Klick auf eine Karte kopiert HTML + CSS.',
    colour: 'Farbe', copied: 'Kopiert', copiedName: '„{name}" kopiert', copyFail: 'Kopieren nicht möglich'
  }
});

// Every loader uses the CSS variable --c for its colour.
const LOADERS = [
  { name: 'Spinner', html: '<div class="ld-spinner"></div>', css: `
.ld-spinner { width: 48px; height: 48px; border: 5px solid #1f1d1818; border-top-color: var(--c); border-radius: 50%; animation: ld-rot 1s linear infinite; }
@keyframes ld-rot { to { transform: rotate(360deg); } }` },

  { name: 'Dots', html: '<div class="ld-dots"><i></i><i></i><i></i></div>', css: `
.ld-dots { display: flex; gap: 8px; }
.ld-dots i { width: 14px; height: 14px; border-radius: 50%; background: var(--c); animation: ld-dot .9s ease-in-out infinite; }
.ld-dots i:nth-child(2) { animation-delay: .15s; }
.ld-dots i:nth-child(3) { animation-delay: .3s; }
@keyframes ld-dot { 0%, 80%, 100% { transform: scale(.4); opacity: .4; } 40% { transform: scale(1); opacity: 1; } }` },

  { name: 'Bars', html: '<div class="ld-bars"><i></i><i></i><i></i><i></i><i></i></div>', css: `
.ld-bars { display: flex; gap: 5px; align-items: center; height: 50px; }
.ld-bars i { width: 7px; height: 100%; background: var(--c); border-radius: 4px; animation: ld-bar 1s ease-in-out infinite; }
.ld-bars i:nth-child(2) { animation-delay: .1s; } .ld-bars i:nth-child(3) { animation-delay: .2s; }
.ld-bars i:nth-child(4) { animation-delay: .3s; } .ld-bars i:nth-child(5) { animation-delay: .4s; }
@keyframes ld-bar { 0%, 100% { transform: scaleY(.3); } 50% { transform: scaleY(1); } }` },

  { name: 'Pulse Ring', html: '<div class="ld-pulse"></div>', css: `
.ld-pulse { width: 50px; height: 50px; border-radius: 50%; position: relative; }
.ld-pulse::before, .ld-pulse::after { content: ""; position: absolute; inset: 0; border-radius: 50%; border: 4px solid var(--c); animation: ld-pulse 1.6s ease-out infinite; }
.ld-pulse::after { animation-delay: .8s; }
@keyframes ld-pulse { from { transform: scale(.1); opacity: 1; } to { transform: scale(1.2); opacity: 0; } }` },

  { name: 'Dual Ring', html: '<div class="ld-dual"></div>', css: `
.ld-dual { width: 50px; height: 50px; border-radius: 50%; border: 5px solid; border-color: var(--c) transparent var(--c) transparent; animation: ld-rot 1.2s linear infinite; }` },

  { name: 'Flip Square', html: '<div class="ld-flip"></div>', css: `
.ld-flip { width: 44px; height: 44px; background: var(--c); animation: ld-flip 1.2s ease-in-out infinite; }
@keyframes ld-flip { 0% { transform: perspective(120px) rotateX(0) rotateY(0); } 50% { transform: perspective(120px) rotateX(-180deg) rotateY(0); } 100% { transform: perspective(120px) rotateX(-180deg) rotateY(-180deg); } }` },

  { name: 'Orbit', html: '<div class="ld-orbit"><i></i></div>', css: `
.ld-orbit { width: 56px; height: 56px; border-radius: 50%; border: 2px dashed #1f1d1833; position: relative; animation: ld-rot 1.4s linear infinite; }
.ld-orbit i { position: absolute; width: 14px; height: 14px; top: -7px; left: calc(50% - 7px); border-radius: 50%; background: var(--c); }` },

  { name: 'Wave Text', html: '<div class="ld-wave"><b>L</b><b>O</b><b>A</b><b>D</b><b>I</b><b>N</b><b>G</b></div>', css: `
.ld-wave { display: flex; gap: 2px; font: 700 20px/1 system-ui, sans-serif; color: var(--c); }
.ld-wave b { animation: ld-wave 1.2s ease-in-out infinite; }
.ld-wave b:nth-child(2) { animation-delay: .1s; } .ld-wave b:nth-child(3) { animation-delay: .2s; }
.ld-wave b:nth-child(4) { animation-delay: .3s; } .ld-wave b:nth-child(5) { animation-delay: .4s; }
.ld-wave b:nth-child(6) { animation-delay: .5s; } .ld-wave b:nth-child(7) { animation-delay: .6s; }
@keyframes ld-wave { 0%, 60%, 100% { transform: translateY(0); } 30% { transform: translateY(-12px); } }` },

  { name: 'Progress', html: '<div class="ld-progress"></div>', css: `
.ld-progress { width: 130px; height: 8px; border-radius: 4px; background: #1f1d1818; overflow: hidden; position: relative; }
.ld-progress::after { content: ""; position: absolute; inset: 0; width: 40%; border-radius: 4px; background: var(--c); animation: ld-prog 1.3s ease-in-out infinite; }
@keyframes ld-prog { from { left: -40%; } to { left: 100%; } }` },

  { name: 'Grid', html: '<div class="ld-grid">' + '<i></i>'.repeat(9) + '</div>', css: `
.ld-grid { display: grid; grid-template-columns: repeat(3, 14px); gap: 4px; }
.ld-grid i { width: 14px; height: 14px; background: var(--c); animation: ld-grid 1.3s ease-in-out infinite; }
.ld-grid i:nth-child(3n+1) { animation-delay: .2s; } .ld-grid i:nth-child(3n+2) { animation-delay: .3s; } .ld-grid i:nth-child(3n) { animation-delay: .4s; }
.ld-grid i:nth-child(n+4) { animation-delay: .1s; } .ld-grid i:nth-child(n+7) { animation-delay: 0s; }
@keyframes ld-grid { 0%, 70%, 100% { transform: scale(1); } 35% { transform: scale(0); } }` },

  { name: 'Heart', html: '<div class="ld-heart"></div>', css: `
.ld-heart { width: 30px; height: 30px; background: var(--c); transform: rotate(45deg); position: relative; animation: ld-heart 1s ease-in-out infinite; }
.ld-heart::before, .ld-heart::after { content: ""; position: absolute; width: 30px; height: 30px; border-radius: 50%; background: var(--c); }
.ld-heart::before { left: -15px; } .ld-heart::after { top: -15px; }
@keyframes ld-heart { 0%, 100% { transform: rotate(45deg) scale(.8); } 30% { transform: rotate(45deg) scale(1.1); } }` },

  { name: 'Hourglass', html: '<div class="ld-hour"></div>', css: `
.ld-hour { width: 0; height: 0; border: 24px solid var(--c); border-color: var(--c) transparent var(--c) transparent; border-radius: 50%; animation: ld-hour 1.2s infinite; }
@keyframes ld-hour { 0% { transform: rotate(0); animation-timing-function: cubic-bezier(.55,.055,.675,.19); } 50% { transform: rotate(900deg); animation-timing-function: cubic-bezier(.215,.61,.355,1); } 100% { transform: rotate(1800deg); } }` },
];

const grid = document.getElementById('grid');
const style = document.createElement('style');
style.textContent = LOADERS.map(l => l.css).join('\n');
document.head.appendChild(style);

for (const l of LOADERS) {
  const card = document.createElement('div');
  card.className = 'card';
  card.innerHTML = `<div class="stage">${l.html}</div><span>${l.name}</span>`;
  card.onclick = () => copy(l);
  grid.appendChild(card);
}

async function copy(l) {
  const color = document.getElementById('color').value;
  // @keyframes ld-rot is shared by several loaders -> include it when used
  let css = l.css.trim();
  if (css.includes('ld-rot') && !css.includes('@keyframes ld-rot')) css += '\n@keyframes ld-rot { to { transform: rotate(360deg); } }';
  const text = `<!-- HTML -->\n${l.html}\n\n<style>\n:root { --c: ${color}; }\n${css}\n</style>`;
  const toast = document.getElementById('toast');
  try {
    await navigator.clipboard.writeText(text);
    toast.textContent = I18N.t('copiedName', { name: l.name });
  } catch {
    toast.textContent = I18N.t('copyFail');
  }
  toast.classList.add('show');
  setTimeout(() => toast.classList.remove('show'), 1500);
}

document.getElementById('color').oninput = e => document.documentElement.style.setProperty('--c', e.target.value);

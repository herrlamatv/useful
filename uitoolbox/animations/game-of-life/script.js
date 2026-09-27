I18N.add({
  en: {
    title: 'Game of Life',
    back: '← Overview',
    heading: '🧬 Game of Life',
    info: 'Generation {gen} · Population {pop}',
    start: 'Start',
    stop: 'Stop',
    step: 'Step',
    random: 'Random',
    clear: 'Clear',
    speed: 'Speed',
    cellSize: 'Cell size',
    pattern: 'Pattern',
    insert: 'insert…',
    hint: 'Click & drag = draw / erase cells'
  },
  de: {
    title: 'Game of Life',
    back: '← Übersicht',
    heading: '🧬 Game of Life',
    info: 'Generation {gen} · Population {pop}',
    start: 'Start',
    stop: 'Stop',
    step: 'Schritt',
    random: 'Zufall',
    clear: 'Leeren',
    speed: 'Tempo',
    cellSize: 'Zellgröße',
    pattern: 'Muster',
    insert: 'einfügen…',
    hint: 'Klicken & ziehen = Zellen malen / löschen'
  }
});

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);

let W, H, dpr;
let cell, cols = 0, rows = 0;
let grid = new Uint16Array(0);   // 0 = dead, >0 = age in generations
let running = false, generation = 0, acc = 0;

const patterns = {
  glider: ['.O.', '..O', 'OOO'],
  lwss: ['.O..O', 'O....', 'O...O', 'OOOO.'],
  rpent: ['.OO', 'OO.', '.O.'],
  pulsar: [
    '..OOO...OOO..', '.............', 'O....O.O....O', 'O....O.O....O', 'O....O.O....O',
    '..OOO...OOO..', '.............', '..OOO...OOO..', 'O....O.O....O', 'O....O.O....O',
    'O....O.O....O', '.............', '..OOO...OOO..'
  ],
  gun: [
    '........................O...........',
    '......................O.O...........',
    '............OO......OO............OO',
    '...........O...O....OO............OO',
    'OO........O.....O...OO..............',
    'OO........O...O.OO....O.O...........',
    '..........O.....O.......O...........',
    '...........O...O....................',
    '............OO......................'
  ]
};

const idx = (x, y) => ((y + rows) % rows) * cols + ((x + cols) % cols);

// (re)build grid, keeping overlapping cells
function rebuild() {
  cell = +$('size').value;
  const nc = Math.max(1, Math.floor(W / cell)), nr = Math.max(1, Math.floor(H / cell));
  const ng = new Uint16Array(nc * nr);
  for (let y = 0; y < Math.min(rows, nr); y++)
    for (let x = 0; x < Math.min(cols, nc); x++) ng[y * nc + x] = grid[y * cols + x];
  cols = nc; rows = nr; grid = ng;
  draw();
}

function resize() {
  dpr = Math.min(devicePixelRatio || 1, 2);
  W = innerWidth; H = innerHeight;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  rebuild();
}

function step() {
  const next = new Uint16Array(cols * rows);
  for (let y = 0; y < rows; y++) {
    const ym = ((y - 1 + rows) % rows) * cols, y0 = y * cols, yp = ((y + 1) % rows) * cols;
    for (let x = 0; x < cols; x++) {
      const xm = (x - 1 + cols) % cols, xp = (x + 1) % cols;
      const n = (grid[ym + xm] > 0) + (grid[ym + x] > 0) + (grid[ym + xp] > 0)
              + (grid[y0 + xm] > 0) + (grid[y0 + xp] > 0)
              + (grid[yp + xm] > 0) + (grid[yp + x] > 0) + (grid[yp + xp] > 0);
      const age = grid[y0 + x];
      if (age ? (n === 2 || n === 3) : n === 3) next[y0 + x] = Math.min(age + 1, 60000);
    }
  }
  grid = next;
  generation++;
}

// age buckets -> colors (young = bright blue, old = violet)
const ageColors = ['#9fd0ff', '#58a6ff', '#4d7dff', '#7b61ff', '#b366ff'];
const bucket = a => a <= 1 ? 0 : a <= 4 ? 1 : a <= 15 ? 2 : a <= 50 ? 3 : 4;

function draw() {
  ctx.fillStyle = '#070b14';
  ctx.fillRect(0, 0, W, H);

  // faint grid lines for bigger cells
  if (cell >= 8) {
    ctx.strokeStyle = 'rgba(255,255,255,0.04)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = 0; x <= cols; x++) { ctx.moveTo(x * cell + 0.5, 0); ctx.lineTo(x * cell + 0.5, rows * cell); }
    for (let y = 0; y <= rows; y++) { ctx.moveTo(0, y * cell + 0.5); ctx.lineTo(cols * cell, y * cell + 0.5); }
    ctx.stroke();
  }

  const paths = ageColors.map(() => new Path2D());
  const gap = cell > 5 ? 1 : 0;
  let pop = 0;
  for (let y = 0; y < rows; y++)
    for (let x = 0; x < cols; x++) {
      const a = grid[y * cols + x];
      if (!a) continue;
      pop++;
      paths[bucket(a)].rect(x * cell + gap, y * cell + gap, cell - gap, cell - gap);
    }
  paths.forEach((p, i) => { ctx.fillStyle = ageColors[i]; ctx.fill(p); });

  $('info').textContent = I18N.t('info', { gen: generation, pop });
}

let last = performance.now();
function frame(now) {
  const dt = Math.min(now - last, 250); last = now;
  if (running) {
    acc += dt;
    const interval = 1000 / +$('speed').value;
    let n = 0;
    while (acc >= interval && n < 4) { step(); acc -= interval; n++; }
    if (n === 4) acc = 0;
    if (n) draw();
  }
  requestAnimationFrame(frame);
}

function place(name) {
  const rowsP = patterns[name];
  const ph = rowsP.length, pw = rowsP[0].length;
  const ox = Math.floor((cols - pw) / 2), oy = Math.floor((rows - ph) / 2);
  for (let y = -1; y <= ph; y++)
    for (let x = -1; x <= pw; x++)
      grid[idx(ox + x, oy + y)] = rowsP[y] && rowsP[y][x] === 'O' ? 1 : 0;
  draw();
}

function setRunning(on) {
  running = on;
  $('play').textContent = I18N.t(on ? 'stop' : 'start');
  $('play').classList.toggle('on', on);
  acc = 0;
}

// drawing with pointer (Bresenham line between samples)
let painting = false, paintVal = 1, lastCell = null;
function cellAt(e) { return [Math.floor(e.clientX / cell), Math.floor(e.clientY / cell)]; }
function paint(cx, cy) {
  if (cx < 0 || cy < 0 || cx >= cols || cy >= rows) return;
  grid[cy * cols + cx] = paintVal;
}
function paintLine([x0, y0], [x1, y1]) {
  const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0), sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
  let err = dx + dy;
  for (;;) {
    paint(x0, y0);
    if (x0 === x1 && y0 === y1) break;
    const e2 = 2 * err;
    if (e2 >= dy) { err += dy; x0 += sx; }
    if (e2 <= dx) { err += dx; y0 += sy; }
  }
}

canvas.addEventListener('pointerdown', e => {
  const [x, y] = cellAt(e);
  if (x >= cols || y >= rows) return;
  painting = true;
  canvas.setPointerCapture(e.pointerId);
  paintVal = grid[y * cols + x] ? 0 : 1;
  lastCell = [x, y];
  paint(x, y); draw();
});
canvas.addEventListener('pointermove', e => {
  if (!painting) return;
  const c = cellAt(e);
  paintLine(lastCell, c); lastCell = c; draw();
});
const stopPaint = () => { painting = false; };
canvas.addEventListener('pointerup', stopPaint);
canvas.addEventListener('pointercancel', stopPaint);

$('play').onclick = () => setRunning(!running);
$('step').onclick = () => { setRunning(false); step(); draw(); };
$('random').onclick = () => {
  for (let i = 0; i < grid.length; i++) grid[i] = Math.random() < 0.25 ? 1 : 0;
  generation = 0; draw();
};
$('clear').onclick = () => { grid.fill(0); generation = 0; setRunning(false); draw(); };
$('size').oninput = rebuild;
$('pattern').onchange = e => { if (e.target.value) place(e.target.value); e.target.value = ''; };
addEventListener('resize', resize);
addEventListener('langchange', () => {
  $('play').textContent = I18N.t(running ? 'stop' : 'start');
  draw();
});
addEventListener('keydown', e => {
  if (e.code === 'Space' && e.target === document.body) { e.preventDefault(); setRunning(!running); }
});

resize();
place('gun');
setRunning(true);
requestAnimationFrame(frame);

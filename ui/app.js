'use strict';
// Mindloom frontend. All behaviour lives in Rust; this file draws the scene
// it returns and turns pointer and keyboard input into commands.

const NS = 'http://www.w3.org/2000/svg';
const PALETTE = ['#ff6b6b', '#ffa500', '#f5c400', '#7bd44a', '#19c3c3', '#2f9bff', '#9b6bff', '#ee6fd0'];
const $ = (id) => document.getElementById(id);
const tauri = window.__TAURI__;

let S = null;            // latest state from Rust
let sel = null;          // selected node id
let view = { x: 0, y: 0, k: 1 };
let editing = null;      // { id, fresh }
let busy = false;
let selLink = null;      // selected link id
let linkMode = false;    // waiting for the second node of a new link

// ---------- Rust bridge ----------
async function call(cmd, args = {}) {
  if (tauri) return tauri.core.invoke('run_command', { name: cmd, args });
  const r = await fetch('/invoke/' + cmd, { method: 'POST', body: JSON.stringify(args) });
  const j = await r.json();
  if (!r.ok) throw new Error(j.error || 'request failed');
  return j;
}

// Commands run strictly in order, so fast typing can never reorder edits.
let queue = Promise.resolve();
function act(cmd, args = {}) {
  const run = async () => {
    try {
      const st = await call(cmd, args);
      if (st.scene) { S = st; render(); }
      return st;
    } catch (e) {
      toast(String(e.message || e));
      return null;
    }
  };
  const p = queue.then(run, run);
  queue = p.catch(() => {});
  return p;
}

function toast(msg, info = false) {
  const t = $('toast');
  t.textContent = msg; t.hidden = false;
  t.classList.toggle('info', info);
  clearTimeout(toast.h);
  toast.h = setTimeout(() => (t.hidden = true), info ? 4000 : 2600);
}

// ---------- Rendering ----------
const world = $('world');
function el(name, attrs = {}, parent) {
  const e = document.createElementNS(NS, name);
  for (const k in attrs) e.setAttribute(k, attrs[k]);
  if (parent) parent.appendChild(e);
  return e;
}
const pathOf = (pts) => pts.map((p, i) => (i ? 'L' : 'M') + p.x.toFixed(1) + ' ' + p.y.toFixed(1)).join(' ') + 'Z';
const lineOf = (pts) => pts.map((p, i) => (i ? 'L' : 'M') + p.x.toFixed(1) + ' ' + p.y.toFixed(1)).join(' ');
const nodeById = (id) => S.scene.nodes.find((n) => n.id === id);
const meta = (id) => S.nodes.find((n) => n.id === id);

function applyView() {
  world.setAttribute('transform', `translate(${view.x} ${view.y}) scale(${view.k})`);
}

function render() {
  document.body.classList.toggle('dark', S.scene.dark_mode);
  $('b-dark').classList.toggle('on', S.scene.dark_mode);
  $('b-undo').disabled = !S.can_undo;
  $('b-redo').disabled = !S.can_redo;
  $('title').textContent = S.title;
  setupFonts(S.fonts);
  if (sel !== null && !nodeById(sel)) sel = null;
  if (selLink !== null && !S.scene.links.some((l) => l.id === selLink)) selLink = null;
  world.replaceChildren();
  const bg = el('g', {}, world);
  for (const b of S.scene.branches) {
    if (b.style === 'Dotted') {
      el('path', { d: lineOf(b.centerline), fill: 'none', stroke: b.color, 'stroke-width': b.width * 0.5,
        'stroke-linecap': 'round', 'stroke-dasharray': `1 ${b.width * 1.2}` }, bg);
    } else {
      el('path', { d: pathOf(b.outline), fill: b.color, stroke: b.color, 'stroke-width': 0.6,
        'stroke-linejoin': 'round' }, bg);
    }
  }
  drawLinks();
  for (const n of S.scene.nodes) drawNode(n);
  drawSelection();
  drawLinkHandles();
  updateFormatBar();
  updateLinkBar();
}

function drawNode(n) {
  const r = n.rect;
  const g = el('g', { 'data-id': n.id }, world);
  el('rect', { x: r.cx - r.w / 2 - 4, y: r.cy - r.h / 2 - 4, width: r.w + 8, height: r.h + 8, class: 'hit',
    'data-id': n.id }, g);
  if (n.shape === 'Marker') {
    el('path', { transform: `translate(${r.cx} ${r.cy})`, d: n.frame, fill: 'none', stroke: n.color,
      'stroke-width': 8, 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'pointer-events': 'none' }, g);
  } else if (n.shape === 'RoundedBox') {
    el('rect', { x: r.cx - r.w / 2, y: r.cy - r.h / 2, width: r.w, height: r.h, rx: 14, fill: 'none',
      stroke: n.color, 'stroke-width': 3, 'pointer-events': 'none' }, g);
  } else if ((n.shape === 'Cloud' || n.shape === 'Bubble') && n.shape_path) {
    el('path', { transform: `translate(${r.cx} ${r.cy})`, d: n.shape_path, fill: 'none', stroke: n.color,
      'stroke-width': 3, 'pointer-events': 'none' }, g);
  }
  if (editing && editing.id === n.id) return; // the editor overlay shows the text
  const lines = n.text.split('\n');
  lines.forEach((line, i) => {
    const t = el('text', { x: r.cx, y: n.text_baseline - (lines.length - 1 - i) * n.font_size * 1.25,
      'text-anchor': 'middle', 'font-family': n.font_stack, 'font-size': n.font_size, 'font-weight': n.bold ? 700 : 400,
      'font-style': n.italic ? 'italic' : 'normal' }, g);
    t.textContent = line;
  });
  if (n.collapsed && n.has_children) {
    const left = n.side === 'Left';
    const fx = left ? r.cx - r.w / 2 - 14 : r.cx + r.w / 2 + 14;
    const f = el('g', { class: 'fold' }, g);
    el('circle', { cx: fx, cy: r.cy, r: 9 }, f);
    const t = el('text', { x: fx, y: r.cy + 4, 'text-anchor': 'middle' }, f);
    t.textContent = '+';
  }
}

function drawSelection() {
  if (sel === null) return;
  const n = nodeById(sel);
  if (!n) return;
  const r = n.rect;
  el('rect', { class: 'sel', x: r.cx - r.w / 2 - 6, y: r.cy - r.h / 2 - 6, width: r.w + 12, height: r.h + 12,
    rx: 12 }, world);
  // A central idea (the first or a later one) grows branches on both sides.
  const sides = n.central ? [-1, 1] : [n.side === 'Left' ? -1 : 1];
  for (const d of sides) {
    const g = el('g', { class: 'plus', 'data-plus': sel }, world);
    el('circle', { cx: r.cx + d * (r.w / 2 + 22), cy: r.cy, r: 11, 'data-plus': sel }, g);
    const t = el('text', { x: r.cx + d * (r.w / 2 + 22), y: r.cy + 5, 'text-anchor': 'middle',
      'font-size': 16 }, g);
    t.textContent = '+';
  }
}

// ---------- Fonts ----------
// The catalogue lives in Rust. Here it only becomes @font-face rules and the
// picker, so adding a font never needs a change in this file.
let fontsReady = false;
function setupFonts(fonts) {
  if (fontsReady) return;
  fontsReady = true;
  const css = fonts.flatMap((f) => f.files.map((x) =>
    `@font-face{font-family:'${f.name}';font-weight:${x.weight};src:url(fonts/${x.file}) format('woff2');font-display:swap;}`));
  const st = document.createElement('style');
  st.textContent = css.join('\n');
  document.head.appendChild(st);
  const picker = $('f-font');
  for (const f of fonts) {
    const o = document.createElement('option');
    o.value = f.name; o.textContent = f.name; o.style.fontFamily = f.stack;
    picker.appendChild(o);
  }
  picker.onchange = (e) => { if (sel !== null) act('set_format', { id: sel, font: e.target.value }); };
  // Preload so the first render and the editor never flash a fallback face.
  for (const f of fonts) document.fonts.load(`20px '${f.name}'`).catch(() => {});
}
const stackOf = (name) => (S.fonts.find((f) => f.name === name) || S.fonts[0]).stack;

// ---------- Links ----------
function drawLinks() {
  const g = el('g', {}, world);
  for (const l of S.scene.links) {
    if (l.id === selLink) {
      el('path', { class: 'linksel', d: l.path, 'stroke-width': l.width + 10 }, g);
    }
    el('path', { d: l.path, fill: 'none', stroke: l.color, 'stroke-width': l.width, 'stroke-linecap': 'round',
      'stroke-dasharray': l.style === 'Dashed' ? `${l.width * 3} ${l.width * 2.4}` : 'none',
      'pointer-events': 'none' }, g);
    for (const head of [l.arrow_end, l.arrow_start]) {
      if (!head.length) continue;
      el('polygon', { points: head.map((p) => `${p.x},${p.y}`).join(' '), fill: l.color, stroke: l.color,
        'stroke-width': 1, 'stroke-linejoin': 'round', 'pointer-events': 'none' }, g);
    }
    el('path', { class: 'linkhit', d: l.path, 'data-link': l.id }, g);
  }
}

// Where a link visibly starts and ends (the arrow tip when there is one).
function linkEnds(l) {
  const nums = l.path.split(/[ML\s]+/).filter(Boolean).map(Number);
  const first = { x: nums[0], y: nums[1] };
  const last = { x: nums[nums.length - 2], y: nums[nums.length - 1] };
  return {
    start: l.arrow_start.length ? l.arrow_start[0] : first,
    end: l.arrow_end.length ? l.arrow_end[0] : last,
  };
}
// Bow of the curve for a pointer at world position w. The curve's midpoint
// sits 0.75 * bend * length away from the straight line between the nodes.
function bendFor(l, w) {
  const dx = l.cb.x - l.ca.x, dy = l.cb.y - l.ca.y;
  const dist = Math.hypot(dx, dy) || 1;
  const nx = -dy / dist, ny = dx / dist;
  const off = (w.x - (l.ca.x + l.cb.x) / 2) * nx + (w.y - (l.ca.y + l.cb.y) / 2) * ny;
  return Math.max(-1.5, Math.min(1.5, off / (0.75 * dist)));
}
function drawLinkHandles() {
  const l = selLink === null ? null : S.scene.links.find((x) => x.id === selLink);
  if (!l || editing) return;
  const { start, end } = linkEnds(l);
  const g = el('g', { class: 'linkhandles' }, world);
  const add = (which, p, cls) => el('circle', { class: 'lh ' + cls, cx: p.x, cy: p.y, r: 7, 'data-lh': which }, g);
  add('from', start, 'lh-end');
  add('to', end, 'lh-end');
  add('bend', l.mid, 'lh-bend');
}
function nodeAt(w, skip) {
  return S.scene.nodes.find((n) => n.id !== skip && Math.abs(w.x - n.rect.cx) < n.rect.w / 2 + 6 &&
    Math.abs(w.y - n.rect.cy) < n.rect.h / 2 + 6);
}

const LINK_COLORS = ['#8a93a3', ...PALETTE];
for (const c of LINK_COLORS) {
  const i = document.createElement('i');
  i.style.background = c; i.title = c;
  i.onclick = () => selLink !== null && act('set_link', { id: selLink, color: c });
  $('link-swatches').appendChild(i);
}
for (const b of document.querySelectorAll('#link-style button')) {
  b.onclick = () => selLink !== null && act('set_link', { id: selLink, style: b.dataset.style });
}
for (const b of document.querySelectorAll('#link-arrow button')) {
  b.onclick = () => selLink !== null && act('set_link', { id: selLink, arrow: b.dataset.arrow });
}
$('link-delete').onclick = () => selLink !== null && act('delete_link', { id: selLink });

function updateLinkBar() {
  const l = selLink === null ? null : S.scene.links.find((x) => x.id === selLink);
  $('linkbar').hidden = !l || !!editing;
  if (!l) return;
  for (const b of document.querySelectorAll('#link-style button')) b.classList.toggle('on', b.dataset.style === l.style);
  for (const b of document.querySelectorAll('#link-arrow button')) b.classList.toggle('on', b.dataset.arrow === l.arrow);
}

function setLinkMode(on) {
  linkMode = on;
  svg.classList.toggle('linking', on);
  if (!on) $('toast').hidden = true;
}
function startLink() {
  if (sel === null) { toast('Select a node first, then press Link'); return; }
  setLinkMode(true);
  toast('Now click the node to link to (Esc cancels)', true);
}
async function finishLink(target) {
  const from = sel;
  setLinkMode(false);
  if (from === null || target === from) return;
  const st = await act('add_link', { from, to: target });
  if (st) { sel = null; selLink = st.new_link; render(); }
}

// ---------- Format bar ----------
for (const c of PALETTE) {
  const i = document.createElement('i');
  i.style.background = c; i.title = c;
  i.onclick = () => sel !== null && act('set_color', { id: sel, color: c });
  $('swatches').appendChild(i);
}
$('f-color').oninput = (e) => sel !== null && act('set_color', { id: sel, color: e.target.value });
for (const b of document.querySelectorAll('#shapes button')) {
  b.onclick = () => sel !== null && act('set_shape', { id: sel, shape: b.dataset.shape });
}
$('f-bold').onclick = () => { const m = meta(sel); m && act('set_format', { id: sel, bold: !m.bold }); };
$('f-italic').onclick = () => { const m = meta(sel); m && act('set_format', { id: sel, italic: !m.italic }); };
$('f-smaller').onclick = () => { const m = meta(sel); m && act('set_format', { id: sel, font_size: m.font_size - 2 }); };
$('f-bigger').onclick = () => { const m = meta(sel); m && act('set_format', { id: sel, font_size: m.font_size + 2 }); };

function updateFormatBar() {
  const bar = $('formatbar');
  bar.hidden = sel === null || !!editing;
  if (bar.hidden) return;
  const m = meta(sel), n = nodeById(sel);
  for (const b of document.querySelectorAll('#shapes button')) b.classList.toggle('on', b.dataset.shape === m.shape);
  $('f-font').value = m.font;
  $('f-bold').classList.toggle('on', m.bold);
  $('f-italic').classList.toggle('on', m.italic);
  $('f-color').value = n.color;
}

// ---------- View helpers ----------
const toWorld = (sx, sy) => ({ x: (sx - view.x) / view.k, y: (sy - view.y) / view.k });
function fit() {
  if (!S) return;
  const b = S.scene.bounds, W = innerWidth, H = innerHeight;
  const k = Math.min(2, Math.min((W - 120) / Math.max(b.w, 1), (H - 190) / Math.max(b.h, 1)));
  view = { k, x: W / 2 - b.cx * k, y: H / 2 + 10 - b.cy * k };
  applyView();
}

// ---------- Pointer interaction ----------
const svg = $('canvas');
let drag = null;
let lastDown = { id: -1, t: 0 };

svg.addEventListener('pointerdown', (e) => {
  if (editing) { commitEdit(); }
  const plus = e.target.closest('[data-plus]');
  if (plus) { e.stopPropagation(); addChild(Number(plus.getAttribute('data-plus'))); return; }
  const hit = e.target.closest('[data-id]');
  const linkHit = e.target.closest('[data-link]');
  svg.setPointerCapture(e.pointerId);
  if (linkMode) {
    if (hit) finishLink(Number(hit.getAttribute('data-id'))); else setLinkMode(false);
    return;
  }
  const lh = e.target.closest('[data-lh]');
  if (lh && selLink !== null) {
    drag = { kind: 'link', which: lh.getAttribute('data-lh'), id: selLink, moved: false, sx: e.clientX, sy: e.clientY };
    return;
  }
  if (!hit && linkHit) {
    // Pressing on the curve selects it, and dragging from there bends it.
    sel = null; selLink = Number(linkHit.getAttribute('data-link')); render();
    drag = { kind: 'link', which: 'bend', id: selLink, moved: false, sx: e.clientX, sy: e.clientY };
    return;
  }
  selLink = null;
  if (hit) {
    const id = Number(hit.getAttribute('data-id'));
    // Double click is detected by hand: redrawing on pointerdown detaches the
    // clicked element, so the browser's own dblclick event is unreliable.
    const now = performance.now();
    if (lastDown.id === id && now - lastDown.t < 400) {
      lastDown = { id: -1, t: 0 };
      sel = id; drag = null; render();
      // Deferred so the browser's own mousedown focus change cannot blur the editor.
      setTimeout(() => startEdit(id, false), 0);
      return;
    }
    lastDown = { id, t: now };
    sel = id; render();
    drag = { kind: 'node', id, sx: e.clientX, sy: e.clientY, moved: false, shift: e.shiftKey };
  } else {
    sel = null; render();
    drag = { kind: 'pan', sx: e.clientX, sy: e.clientY, vx: view.x, vy: view.y };
    svg.classList.add('panning');
  }
});

svg.addEventListener('pointermove', (e) => {
  if (!drag) return;
  const dx = e.clientX - drag.sx, dy = e.clientY - drag.sy;
  if (drag.kind === 'link') {
    if (Math.abs(dx) + Math.abs(dy) > 4) drag.moved = true;
    if (drag.moved) previewLinkDrag(drag, toWorld(e.clientX, e.clientY));
    return;
  }
  if (drag.kind === 'pan') {
    view.x = drag.vx + dx; view.y = drag.vy + dy; applyView();
  } else {
    if (Math.abs(dx) + Math.abs(dy) > 4) drag.moved = true;
    if (drag.moved) {
      const g = world.querySelector(`g[data-id="${drag.id}"]`);
      if (g) g.setAttribute('transform', `translate(${dx / view.k} ${dy / view.k})`);
    }
  }
});

function linkOf(id) { return S.scene.links.find((x) => x.id === id); }

// Live feedback while a link is dragged: nothing is sent to Rust until release,
// so a whole drag is a single undo step.
function previewLinkDrag(d, w) {
  const l = linkOf(d.id);
  if (!l) return;
  world.querySelector('.lh-preview')?.remove();
  world.querySelectorAll('.node-target').forEach((n) => n.classList.remove('node-target'));
  const { start, end } = linkEnds(l);
  const g = world.querySelector('.linkhandles');
  if (d.which === 'bend') {
    const b = bendFor(l, w);
    const dx = l.cb.x - l.ca.x, dy = l.cb.y - l.ca.y, nx = -dy, ny = dx; // bend is a fraction of length
    const c1 = { x: l.ca.x + dx * 0.25 + nx * b, y: l.ca.y + dy * 0.25 + ny * b };
    const c2 = { x: l.ca.x + dx * 0.75 + nx * b, y: l.ca.y + dy * 0.75 + ny * b };
    el('path', { class: 'lh-preview', fill: 'none',
      d: `M${start.x} ${start.y} C${c1.x} ${c1.y} ${c2.x} ${c2.y} ${end.x} ${end.y}` }, world);
    const h = g?.querySelector('[data-lh="bend"]');
    if (h) { h.setAttribute('cx', w.x); h.setAttribute('cy', w.y); }
  } else {
    const fixed = d.which === 'from' ? end : start;
    el('path', { class: 'lh-preview', fill: 'none', d: `M${fixed.x} ${fixed.y} L${w.x} ${w.y}` }, world);
    const h = g?.querySelector(`[data-lh="${d.which}"]`);
    if (h) { h.setAttribute('cx', w.x); h.setAttribute('cy', w.y); }
    const t = nodeAt(w, d.which === 'from' ? l.to : l.from);
    if (t) world.querySelector(`g[data-id="${t.id}"]`)?.classList.add('node-target');
  }
}

async function finishLinkDrag(d, w) {
  world.querySelector('.lh-preview')?.remove();
  const l = linkOf(d.id);
  if (!d.moved || !l) return;
  if (d.which === 'bend') {
    await act('set_link', { id: d.id, bend: bendFor(l, w) });
    return;
  }
  const t = nodeAt(w, d.which === 'from' ? l.to : l.from);
  if (!t) { render(); toast('Drop the end of the link on a node', true); return; }
  const next = d.which === 'from' ? { from: t.id, to: l.to } : { from: l.from, to: t.id };
  if (next.from === l.from && next.to === l.to) { render(); return; }
  const st = await act('set_link', { id: d.id, ...next });
  if (!st) render(); // refused: put the handle back
}

svg.addEventListener('pointerup', async (e) => {
  const d = drag; drag = null;
  svg.classList.remove('panning');
  if (d && d.kind === 'link') { await finishLinkDrag(d, toWorld(e.clientX, e.clientY)); return; }
  if (!d || d.kind !== 'node' || !d.moved) return;
  if (d.id === S.root) { render(); return; }
  const dx = (e.clientX - d.sx) / view.k, dy = (e.clientY - d.sy) / view.k;
  const w = toWorld(e.clientX, e.clientY);
  const target = e.shiftKey
    ? S.scene.nodes.find((n) => n.id !== d.id && Math.abs(w.x - n.rect.cx) < n.rect.w / 2 + 6 &&
        Math.abs(w.y - n.rect.cy) < n.rect.h / 2 + 6)
    : null;
  if (target) await act('reparent', { id: d.id, parent: target.id });
  else await act('move_node', { id: d.id, dx, dy });
});

svg.addEventListener('wheel', (e) => {
  e.preventDefault();
  const f = Math.exp(-e.deltaY * 0.0015);
  const k = Math.min(4, Math.max(0.15, view.k * f));
  const w = toWorld(e.clientX, e.clientY);
  view.k = k; view.x = e.clientX - w.x * k; view.y = e.clientY - w.y * k;
  applyView();
}, { passive: false });

// ---------- Editing ----------
const editor = $('editor');

function placeEditor(sx, sy, fontSize, bold, italic, stack) {
  editor.style.fontFamily = stack || stackOf('Open Sans');
  editor.style.fontSize = fontSize * view.k + 'px';
  editor.style.fontWeight = bold ? 700 : 400;
  editor.style.fontStyle = italic ? 'italic' : 'normal';
  editor.dataset.sx = sx; editor.dataset.sy = sy;
  sizeEditor(sx, sy);
}

// Pans the view just enough to bring a node inside the usable window area.
function ensureVisible(id) {
  const n = nodeById(id);
  if (!n) return;
  const r = n.rect, m = { l: 40, r: 40, t: 90, b: 120 };
  const x0 = view.x + (r.cx - r.w / 2) * view.k, x1 = view.x + (r.cx + r.w / 2) * view.k;
  const y0 = view.y + (r.cy - r.h / 2) * view.k, y1 = view.y + (r.cy + r.h / 2) * view.k;
  if (x1 > innerWidth - m.r) view.x -= x1 - (innerWidth - m.r);
  if (x0 < m.l) view.x += m.l - x0;
  if (y1 > innerHeight - m.b) view.y -= y1 - (innerHeight - m.b);
  if (y0 < m.t) view.y += m.t - y0;
  applyView();
}

function startEdit(id, fresh, keepValue = false) {
  const n = nodeById(id);
  if (!n) return;
  ensureVisible(id);
  const wasPending = editing && editing.pending;
  editing = { id, fresh };
  sel = id; render();
  const r = n.rect;
  if (!keepValue) editor.value = n.text;
  editor.hidden = false;
  placeEditor(view.x + r.cx * view.k, view.y + r.cy * view.k, n.font_size, n.bold, n.italic, n.font_stack);
  editor.focus();
  if (!keepValue && !wasPending) editor.select();
}

// Opens the editor at once, before Rust has created the node, so no
// keystrokes are lost during the round trip. `create` resolves to the state
// that holds `new_id`.
function startPendingEdit(create) {
  const base = sel !== null && nodeById(sel) ? nodeById(sel).rect : { cx: 0, cy: 0, w: 0 };
  const sx = view.x + (base.cx + (base.w / 2 + 90)) * view.k, sy = view.y + base.cy * view.k;
  editor.value = '';
  editor.hidden = false;
  const mine = { id: null, fresh: true, pending: true };
  editing = mine;
  placeEditor(sx, sy, 20, false, false);
  editor.focus();
  mine.ready = Promise.resolve().then(create).then((st) => {
    if (!st) { if (editing === mine) { editing = null; editor.hidden = true; } return null; }
    // If the editor was already closed (fast typist), do not reopen it.
    if (editing === mine) startEdit(st.new_id, true, true);
    return st.new_id;
  });
  return mine.ready;
}

function sizeEditor(sx, sy) {
  const lines = editor.value.split('\n');
  const longest = Math.max(4, ...lines.map((l) => l.length));
  const fs = parseFloat(editor.style.fontSize);
  const w = longest * fs * 0.62 + 24, h = lines.length * fs * 1.35 + 10;
  editor.style.width = w + 'px'; editor.style.height = h + 'px';
  editor.style.left = sx - w / 2 + 'px'; editor.style.top = sy - h / 2 + 'px';
}
editor.addEventListener('input', () => sizeEditor(+editor.dataset.sx, +editor.dataset.sy));

// Closes the editor synchronously and returns what it held. Everything that
// needs Rust happens afterwards in finishEdit, so typing can carry on at once.
function closeEditor() {
  const cur = editing, text = editor.value;
  editing = null; editor.hidden = true; editor.value = '';
  return { cur, text };
}

async function finishEdit({ cur, text }) {
  if (!cur) return null;
  const id = cur.ready ? await cur.ready : cur.id;
  if (id == null) return null;
  if (cur.fresh && text.trim() === '') { await act('undo'); sel = null; render(); return null; }
  const n = nodeById(id);
  if (n && n.text !== text) await act('set_text', { id, text }); else render();
  return id;
}
const commitEdit = () => finishEdit(closeEditor());

async function cancelEdit() {
  const { cur } = closeEditor();
  if (!cur) return;
  const id = cur.ready ? await cur.ready : cur.id;
  if (cur.fresh && id != null) await act('undo'); else render();
}

editor.addEventListener('keydown', (e) => {
  e.stopPropagation();
  if (e.key === 'Escape') { e.preventDefault(); cancelEdit(); }
  else if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    const done = commitEdit();
    startPendingEdit(async () => {
      const id = await done;
      if (id == null) return null;
      return isCentral(id) ? act('add_child', { parent: id, text: '' }) : act('add_sibling', { id, text: '' });
    });
  } else if (e.key === 'Tab') {
    e.preventDefault();
    const done = commitEdit();
    startPendingEdit(async () => {
      const id = await done;
      return id == null ? null : act('add_child', { parent: id, text: '' });
    });
  }
});
editor.addEventListener('blur', () => { if (editing && !editing.pending) commitEdit(); });

function addChild(parent) {
  return startPendingEdit(() => act('add_child', { parent, text: '' }));
}
const isCentral = (id) => !!nodeById(id)?.central;

// Starts another central idea: a new tree on the same map, with its own colour.
// Pending, like a new branch: Esc or an empty name removes it again.
async function addCentralIdea() {
  const st = await act('add_central_idea', { text: 'Central idea' });
  if (!st) return;
  const r = nodeById(st.new_id).rect;
  view.x = innerWidth / 2 - r.cx * view.k;
  view.y = innerHeight / 2 - r.cy * view.k;
  applyView();
  startEdit(st.new_id, true);
}

function addSibling(id) {
  if (isCentral(id)) return addChild(id);
  return startPendingEdit(() => act('add_sibling', { id, text: '' }));
}

// ---------- Files ----------
function download(name, data, type) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([data], { type }));
  a.download = name; a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
const safeName = () => (S.title || 'mindmap').replace(/[^\w ]+/g, '').trim().replace(/\s+/g, '_') || 'mindmap';

async function saveFile(name, data, filters, mime) {
  if (tauri) {
    const path = await tauri.dialog.save({ defaultPath: name, filters });
    if (path) await tauri.core.invoke('write_file', { path, contents: data });
  } else {
    download(name, data, mime);
  }
}
async function pickFile(filters, accept) {
  if (tauri) {
    const path = await tauri.dialog.open({ multiple: false, filters });
    return path ? tauri.core.invoke('read_file', { path }) : null;
  }
  return new Promise((resolve) => {
    const inp = $('file-input');
    inp.accept = accept; inp.value = '';
    inp.onchange = () => (inp.files[0] ? inp.files[0].text().then(resolve) : resolve(null));
    inp.click();
  });
}

async function doSave() {
  const r = await call('save_json');
  await saveFile(safeName() + '.mindloom', r.data, [{ name: 'Mindloom map', extensions: ['mindloom'] }], 'application/json');
}
async function doOpen() {
  const json = await pickFile([{ name: 'Mindloom map', extensions: ['mindloom'] }], '.mindloom,.json');
  if (json) { sel = null; await act('load_json', { json }); fit(); }
}
async function doImport() {
  const xml = await pickFile([{ name: 'OPML', extensions: ['opml', 'xml'] }], '.opml,.xml');
  if (xml) { sel = null; await act('import_opml', { xml }); fit(); }
}
async function doExport(format) {
  const r = await call('export', { format });
  const info = {
    svg: ['svg', 'image/svg+xml', 'SVG image'],
    markdown: ['md', 'text/markdown', 'Markdown'],
    opml: ['opml', 'text/xml', 'OPML'],
  }[format];
  await saveFile(`${safeName()}.${info[0]}`, r.data, [{ name: info[2], extensions: [info[0]] }], info[1]);
}

// ---------- Toolbar and keys ----------
$('b-new').onclick = async () => { sel = null; await act('new', { title: 'Central idea' }); fit(); startEdit(S.root, false); };
$('b-open').onclick = doOpen;
$('b-save').onclick = () => doSave().catch((e) => toast(String(e)));
$('b-import').onclick = doImport;
$('b-undo').onclick = () => act('undo');
$('b-redo').onclick = () => act('redo');
$('b-child').onclick = () => addChild(sel ?? S.root);
$('b-sibling').onclick = () => sel !== null && addSibling(sel);
$('b-central').onclick = addCentralIdea;
$('b-delete').onclick = () => sel !== null && act('delete', { id: sel });
$('b-fold').onclick = () => sel !== null && act('toggle_collapse', { id: sel });
$('b-link').onclick = startLink;
$('b-clean').onclick = () => act('clean_up');
$('b-fit').onclick = fit;
$('b-dark').onclick = () => act('set_dark', { on: !S.scene.dark_mode });
$('b-export').onchange = (e) => { const f = e.target.value; e.target.value = ''; if (f) doExport(f).catch((x) => toast(String(x))); };

addEventListener('keydown', (e) => {
  if (editing) return;
  const mod = e.ctrlKey || e.metaKey;
  if (mod && e.key.toLowerCase() === 'z') { e.preventDefault(); act(e.shiftKey ? 'redo' : 'undo'); }
  else if (mod && e.key.toLowerCase() === 'y') { e.preventDefault(); act('redo'); }
  else if (mod && e.key.toLowerCase() === 's') { e.preventDefault(); doSave(); }
  else if (mod && e.key.toLowerCase() === 'o') { e.preventDefault(); doOpen(); }
  else if (mod && e.key.toLowerCase() === 'n') { e.preventDefault(); $('b-new').click(); }
  else if (mod && e.key === '0') { e.preventDefault(); fit(); }
  else if (e.key === 'Escape') { if (linkMode) setLinkMode(false); else { sel = null; selLink = null; render(); } }
  else if (selLink !== null && (e.key === 'Delete' || e.key === 'Backspace')) { e.preventDefault(); act('delete_link', { id: selLink }); }
  else if (!mod && !e.altKey && e.key.toLowerCase() === 'c') { e.preventDefault(); addCentralIdea(); }
  else if (sel === null) return;
  else if (e.key === 'l' || e.key === 'L') { e.preventDefault(); startLink(); }
  else if (e.key === 'Tab') { e.preventDefault(); addChild(sel); }
  else if (e.key === 'Enter') { e.preventDefault(); addSibling(sel); }
  else if (e.key === 'F2') { e.preventDefault(); startEdit(sel, false); }
  else if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); act('delete', { id: sel }); }
  else if (e.key === ' ') { e.preventDefault(); act('toggle_collapse', { id: sel }); }
});
addEventListener('resize', () => applyView());

(async () => { S = await call('state'); render(); fit(); })();

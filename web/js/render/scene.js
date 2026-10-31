// The SVG scene: draws a laid-out model and animates between drawings.
// Concepts that stay glide to their new place; new ones grow out of the
// concept that needs them, nearest the goal first (the FLT video's growth
// over time); edges draw themselves in once the concepts have settled.

import { el, drawNode, dotRadius } from "./nodes.js";
import { drawSections } from "./sections.js";
import { edgePath, isCrossLink } from "./paths.js";
import { panzoom } from "./panzoom.js";

const MOVE = 650;
const STAGGER = 55;
const ease = (p) => (p < 0.5 ? 4 * p * p * p : 1 - (-2 * p + 2) ** 3 / 2);

export function createScene(svg, { onSelect, onHover }) {
  const defs = el("defs", {}, svg);
  const marker = el("marker", { id: "arrow", viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: "auto" }, defs);
  el("path", { d: "M0,1 L9,5 L0,9 z", fill: "#5f5d57" }, marker);
  const viewport = el("g", {}, svg);
  const layers = Object.fromEntries(["sections", "edges", "nodes"].map((k) => [k, el("g", { class: k }, viewport)]));
  const pz = panzoom(svg, viewport);
  let shown = new Map(); // id → { g, x, y }
  let frame = 0;
  let last = null;

  svg.addEventListener("click", (e) => {
    if (pz.dragged()) return;
    onSelect(e.target.closest(".node")?.dataset.id ?? null);
  });
  svg.addEventListener("pointerover", (e) => onHover(e.target.closest(".node")?.dataset.id ?? null, e));
  svg.addEventListener("pointerout", (e) => !e.relatedTarget?.closest?.(".node") && onHover(null, e));

  function bounds(model, layout) {
    let x1 = Infinity, y1 = Infinity, x2 = -Infinity, y2 = -Infinity;
    const grow = (a, b, c, d) => ((x1 = Math.min(x1, a)), (y1 = Math.min(y1, b)), (x2 = Math.max(x2, c)), (y2 = Math.max(y2, d)));
    for (const m of model.nodes) {
      const p = layout.pos.get(m.id);
      const [w, h] = layout.kind === "boxes" ? [m.w / 2, m.h / 2] : [m.labelled ? 90 : 12, 14];
      grow(p.x - w, p.y - h, p.x + w, p.y + h);
    }
    for (const s of layout.sections) grow(s.x, s.y, s.x + s.w, s.y + s.h);
    for (const l of layout.labels) grow(l.x - (l.anchor === "end" ? 200 : 0), l.y - 14, l.x + (l.anchor === "start" ? 200 : 0), l.y + 6);
    return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
  }

  function drawEdges(model, layout, settings, grow) {
    layers.edges.replaceChildren();
    const radius = (m) => dotRadius(m);
    for (const e of model.edges) {
      const cross = isCrossLink(e, layout);
      const hue = layout.kind === "dots" && settings.colorBy !== "state" ? model.byId.get(e.from).hue : "#5f5d57";
      const path = el("path", {
        class: `edge${e.implied ? " implied" : ""}${cross ? " cross" : ""}`,
        "data-from": e.from, "data-to": e.to,
        d: edgePath(e, layout.pos, model.byId, layout, settings.linkStyle, radius),
        stroke: hue, "marker-end": layout.kind === "boxes" ? "url(#arrow)" : "none",
      }, layers.edges);
      if (grow.has(e.id)) {
        const len = path.getTotalLength();
        path.style.strokeDasharray = `${len} ${len}`;
        path.style.strokeDashoffset = len;
        path.getBoundingClientRect();
        path.style.transition = "stroke-dashoffset 600ms ease-out";
        path.style.strokeDashoffset = 0;
        path.addEventListener("transitionend", () => (path.style.strokeDasharray = path.style.strokeDashoffset = ""), { once: true });
      }
    }
  }

  /** Draw `model` laid out by `layout`. `opts.fresh` grows everything from the goal. */
  function draw(model, layout, settings, ui, opts = {}) {
    cancelAnimationFrame(frame);
    const before = opts.fresh ? new Map() : shown;
    const oldEdges = new Set(opts.fresh || !last ? [] : last.model.edges.map((e) => e.id));
    for (const [id, s] of before) if (!model.byId.has(id)) s.g.remove();
    layers.nodes.replaceChildren();
    const order = [...model.nodes].sort((a, b) => a.n.depth - b.n.depth);
    const parentOf = (id) => model.edges.find((e) => e.from === id)?.to;
    const moves = [];
    let rank = 0;
    for (const m of order) {
      const to = layout.pos.get(m.id);
      const g = drawNode(m, layout.kind, to, ui, settings.known);
      layers.nodes.appendChild(g);
      const prev = before.get(m.id);
      const origin = prev ?? before.get(parentOf(m.id)) ?? (m.id === model.goal ? to : layout.pos.get(parentOf(m.id)) ?? to);
      moves.push({ m, g, from: { x: origin.x, y: origin.y }, to, fresh: !prev, delay: prev ? 0 : STAGGER * rank++ });
    }
    drawSections(layers.sections, layout, settings.colorBy);
    layers.sections.style.opacity = 0;
    layers.edges.style.opacity = before.size ? 0.12 : 0;
    const start = performance.now();
    const total = MOVE + (rank ? STAGGER * (rank - 1) : 0);
    const tick = (now) => {
      for (const mv of moves) {
        const p = ease(Math.min(1, Math.max(0, (now - start - mv.delay) / MOVE)));
        const x = mv.from.x + (mv.to.x - mv.from.x) * p, y = mv.from.y + (mv.to.y - mv.from.y) * p;
        const s = mv.fresh ? 0.3 + 0.7 * p : 1;
        mv.g.setAttribute("transform", `translate(${x},${y}) scale(${s})`);
        if (mv.fresh) mv.g.style.opacity = p;
      }
      if (now - start < total) frame = requestAnimationFrame(tick);
      else {
        moves.forEach((mv) => (mv.g.style.opacity = ""));
        drawEdges(model, layout, settings, new Set(model.edges.map((e) => e.id).filter((id) => !oldEdges.has(id))));
        layers.edges.style.opacity = 1;
        layers.sections.style.opacity = 1;
      }
    };
    frame = requestAnimationFrame(tick);
    shown = new Map(moves.map((mv) => [mv.m.id, { g: mv.g, x: mv.to.x, y: mv.to.y }]));
    last = { model, layout, box: bounds(model, layout) };
    if (opts.fit !== false) fit();
  }

  function fit() {
    if (!last) return;
    const narrow = svg.clientWidth < 600;
    pz.fit(last.box, {
      minK: narrow ? 0.55 : 0,
      anchor: last.layout.kind === "boxes" ? "bottom" : "center",
      focus: last.layout.pos.get(last.model.goal),
    });
  }

  return {
    draw,
    fit,
    clear() {
      layers.nodes.replaceChildren();
      layers.edges.replaceChildren();
      layers.sections.replaceChildren();
      shown = new Map();
      last = null;
    },
    center(id) {
      const s = shown.get(id);
      if (s) pz.center(s.x, s.y);
    },
    nodeEl: (id) => shown.get(id)?.g,
    highlight(id) {
      svg.classList.toggle("hovering", !!id);
      for (const p of layers.edges.children) p.classList.toggle("hl", !!id && (p.dataset.from === id || p.dataset.to === id));
    },
  };
}

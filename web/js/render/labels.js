// Labels for the dot layouts (radial, force), placed greedily so they never
// overlap each other or a dot. The most important go first: the goal, then
// section roots, then learning, then concepts many others need. A label
// with no free spot is left off; hovering its dot still names it.

import { INK, MUTED } from "../palette.js";
import { measure } from "../model.js";
import { dotRadius, el } from "./nodes.js";

const GAP = 6;

const rank = (m, goal) =>
  m.id === goal ? 0 : m.isRoot ? 1 : m.n.status === "learning" ? 2 : 3 + 1 / (1 + m.n.dependants) + m.n.depth / 100;

const overlaps = (a, b) => a.x1 < b.x2 && b.x1 < a.x2 && a.y1 < b.y2 && b.y1 < a.y2;

/** Candidate boxes around a dot at p (radius r) for a w×h label. */
function candidates(p, r, w, h, isGoal) {
  const len = Math.hypot(p.x, p.y) || 1;
  const out = isGoal ? { x: 0, y: 1 } : { x: p.x / len, y: p.y / len };
  const side = out.x >= 0 ? 1 : -1;
  const at = (cx, cy, anchor) => {
    const x1 = anchor === "start" ? cx : anchor === "end" ? cx - w : cx - w / 2;
    return { x1, x2: x1 + w, y1: cy - h / 2, y2: cy + h / 2, tx: cx, ty: cy + h * 0.32, anchor };
  };
  const sideAnchor = (s) => (s > 0 ? "start" : "end");
  const list = [
    at(p.x + side * (r + GAP), p.y, sideAnchor(side)),
    at(p.x + side * (r + GAP), p.y - h * 0.8, sideAnchor(side)),
    at(p.x + side * (r + GAP), p.y + h * 0.8, sideAnchor(side)),
    at(p.x, p.y + r + GAP + h / 2, "middle"),
    at(p.x, p.y - r - GAP - h / 2, "middle"),
    at(p.x - side * (r + GAP), p.y, sideAnchor(-side)),
  ];
  return isGoal ? [list[3], ...list] : list;
}

export function drawLabels(layer, model, layout, colorBy) {
  layer.replaceChildren();
  const dots = model.nodes.map((m) => {
    const p = layout.pos.get(m.id), r = dotRadius(m) + 2;
    return { id: m.id, x1: p.x - r, x2: p.x + r, y1: p.y - r, y2: p.y + r };
  });
  const placed = [];
  const order = model.nodes.filter((m) => m.labelled).sort((a, b) => rank(a, model.goal) - rank(b, model.goal));
  for (const m of order) {
    const text = (m.n.status === "known" ? "✓ " : "") + m.n.name;
    const px = parseInt(m.font.match(/(\d+)px/)[1], 10);
    const w = measure(text, m.font), h = px * 1.25;
    const free = (c) => !placed.some((b) => overlaps(b, c)) && !dots.some((d) => d.id !== m.id && overlaps(d, c));
    const must = m.id === model.goal || m.isRoot;
    const options = candidates(layout.pos.get(m.id), dotRadius(m), w, h, m.id === model.goal);
    const spot = options.find(free) ?? (must ? options[0] : null);
    if (!spot) continue;
    placed.push(spot);
    const color = m.isRoot && colorBy === "section" ? m.hue : m.n.status === "known" ? MUTED : INK;
    const t = el("text", {
      class: "label", "data-id": m.id, x: spot.tx, y: spot.ty, "text-anchor": spot.anchor, fill: color,
      style: `font:${m.font}`, "paint-order": "stroke", stroke: "#faf9f5", "stroke-width": 4,
    }, layer);
    t.textContent = text;
  }
}

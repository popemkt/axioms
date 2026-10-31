// One concept's SVG: a blueprint box (sans title over a monospace id, as in
// the FLT figure) or a dot with an outward label (as in the FLT video).
// State sits on top of the hue: the goal is clay, a section's key concept is
// tinted, learning gets a gold mark, known is dimmed with a ✓, and a concept
// not yet decomposed has a dashed outline.

import { CLAY, INK, KNOWN_FILL, MUTED, tint } from "../palette.js";
import { MONO } from "../model.js";

const NS = "http://www.w3.org/2000/svg";
const el = (tag, attrs = {}, parent) => {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  parent?.appendChild(e);
  return e;
};
export { el };

export const dotRadius = (m) => (m.n.status === "goal" ? 10 : 5.5 * m.scale);

function look(m) {
  const { status, decomposed } = m.n;
  const dashed = !decomposed && status !== "known" && status !== "goal";
  if (status === "goal") return { fill: CLAY, stroke: "#b65c3f", width: 2, text: "#fff", mono: "#fbe1d4", dashed };
  if (status === "known") return { fill: KNOWN_FILL, stroke: tint(m.hue, 0.35), width: 1.4, text: INK, mono: MUTED, dashed };
  if (status === "learning") return { fill: tint(m.hue, 0.8), stroke: m.hue, width: 2.2, text: INK, mono: MUTED, dashed };
  if (m.isRoot) return { fill: tint(m.hue, 0.72), stroke: m.hue, width: 2.4, text: INK, mono: MUTED, dashed };
  return { fill: "#fff", stroke: m.hue, width: 1.6, text: INK, mono: MUTED, dashed };
}

function box(g, m, L) {
  const { w, h } = m;
  el("rect", { class: "ring", x: -w / 2 - 5, y: -h / 2 - 5, width: w + 10, height: h + 10, rx: 11 }, g);
  el("rect", {
    class: "shape", x: -w / 2, y: -h / 2, width: w, height: h, rx: 8,
    fill: L.fill, stroke: L.stroke, "stroke-width": L.width, "stroke-dasharray": L.dashed ? "5 3.5" : "none",
  }, g);
  if (!m.labelled) return;
  const lh = Math.round(parseInt(m.font.match(/(\d+)px/)[1], 10) * 1.3);
  const top = -h / 2 + 10 + lh * 0.8;
  const title = el("text", { class: "title", "text-anchor": "middle", fill: L.text, style: `font:${m.font}` }, g);
  m.lines.forEach((line, i) => {
    const t = el("tspan", { x: 0, y: top + i * lh }, title);
    t.textContent = line;
  });
  const mono = el("text", { class: "mono", "text-anchor": "middle", y: top + m.lines.length * lh + 2, fill: L.mono, style: `font:${MONO}` }, g);
  mono.textContent = m.id.length > 30 ? `${m.id.slice(0, 29)}…` : m.id;
}

function dot(g, m, L, p) {
  const r = dotRadius(m);
  el("circle", { class: "ring", r: r + 5 }, g);
  el("circle", {
    class: "shape", r, fill: m.n.status === "known" ? m.hue : L.fill, stroke: L.stroke,
    "stroke-width": Math.min(L.width, 2), "stroke-dasharray": L.dashed ? "2.5 2" : "none",
  }, g);
  if (!m.labelled) return;
  const goal = m.n.status === "goal";
  const len = Math.hypot(p.x, p.y) || 1;
  const out = goal ? { x: 0, y: r + 16 } : { x: (p.x / len) * (r + 6), y: (p.y / len) * (r + 6) + 4 };
  const t = el("text", {
    class: "title", x: out.x, y: out.y, fill: INK,
    "text-anchor": goal ? "middle" : p.x >= 0 ? "start" : "end",
    style: `font:${m.font}`, "paint-order": "stroke", stroke: "#faf9f5", "stroke-width": 4,
  }, g);
  t.textContent = (m.n.status === "known" ? "✓ " : "") + m.n.name;
}

/** Build the node's SVG group for model `m` in layout `kind` at final position `p`. */
export function drawNode(m, kind, p, ui, knownMode) {
  const g = el("g", { class: "node", "data-id": m.id });
  const L = look(m);
  (kind === "boxes" ? box : dot)(g, m, L, p);
  if (m.n.status === "learning") {
    const at = kind === "boxes" ? { cx: m.w / 2 - 9, cy: -m.h / 2 + 9 } : { cx: dotRadius(m), cy: -dotRadius(m) };
    el("circle", { class: "badge", ...at, r: 4, fill: "#d4a12a", stroke: "#fff", "stroke-width": 1.5 }, g);
  }
  g.classList.toggle("known", m.n.status === "known");
  g.classList.toggle("dim", m.n.status === "known" && knownMode === "dim");
  g.classList.toggle("selected", ui.selected === m.id);
  g.classList.toggle("pending", ui.pending.has(m.id));
  return g;
}

// Section decorations: blueprint panels (a tinted panel with a coloured bar,
// a title and a subtitle, like Mazur / Ribet / Wiles in the FLT figure) and
// radial section labels (like the FLT video).

import { INK, MUTED, tint } from "../palette.js";
import { el } from "./nodes.js";

const clip = (s, n) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);

export function drawSections(layer, layout, colorBy) {
  layer.replaceChildren();
  for (const s of layout.sections) {
    const hue = colorBy === "section" ? s.hue : "#a8a49a";
    const g = el("g", { class: "section" }, layer);
    el("rect", { x: s.x, y: s.y, width: s.w, height: s.h, rx: 12, fill: tint(hue, 0.86), stroke: "none" }, g);
    el("rect", { x: s.x + 14, y: s.y + 12, width: 3, height: 30, rx: 1.5, fill: hue }, g);
    const t = el("text", { x: s.x + 24, y: s.y + 27, fill: INK, style: "font:600 15px system-ui, sans-serif" }, g);
    t.textContent = clip(s.name, Math.max(12, Math.floor(s.w / 8.5)));
    const sub = el("text", { x: s.x + 24, y: s.y + 42, fill: MUTED, style: "font:11px system-ui, sans-serif" }, g);
    sub.textContent = clip(s.subtitle, Math.max(16, Math.floor(s.w / 6)));
  }
  for (const l of layout.labels) {
    const t = el("text", {
      class: "section-label", x: l.x, y: l.y, "text-anchor": l.anchor, fill: colorBy === "section" ? l.hue : MUTED,
      style: "font:500 13px system-ui, sans-serif", "paint-order": "stroke", stroke: "#faf9f5", "stroke-width": 4,
    }, layer);
    t.textContent = clip(l.text, 34);
  }
}

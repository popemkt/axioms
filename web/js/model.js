// The drawing model: the server's view, filtered and measured for the
// current display settings. Edges point the way the blueprint reads, from a
// prerequisite to the concept that needs it.

import { hueOf, sectionHues } from "./palette.js";

const FONT = { s: 11, m: 13, l: 15 };
const BOX_W = 156;
const ctx = document.createElement("canvas").getContext("2d");

export const fontSize = (settings) => FONT[settings.labelSize];
export const titleFont = (px, bold) => `${bold ? 600 : 500} ${px}px system-ui, -apple-system, "Segoe UI", sans-serif`;
export const MONO = `10px ui-monospace, "SF Mono", Menlo, monospace`;

/** Rendered width of `text` in `font`, in pixels. */
export function measure(text, font) {
  ctx.font = font;
  return ctx.measureText(text).width;
}

/** Greedy word wrap to `width` pixels, at most `max` lines. */
export function wrap(text, font, width, max = 3) {
  ctx.font = font;
  const lines = [];
  let line = "";
  for (const word of String(text).split(/\s+/)) {
    const next = line ? `${line} ${word}` : word;
    if (line && ctx.measureText(next).width > width) {
      lines.push(line);
      line = word;
    } else line = next;
  }
  if (line) lines.push(line);
  if (lines.length > max) lines.splice(max - 1, lines.length, `${lines.slice(max - 1).join(" ").slice(0, 28)}…`);
  return lines;
}

function important(n, ui) {
  return n.status === "goal" || n.depth <= 1 || n.status === "learning" || n.dependants >= 2 || ui.selected === n.id || ui.pending.has(n.id);
}

function scaleOf(n, sizeBy, maxDependants) {
  if (sizeBy === "dependants") return 0.85 + (0.5 * n.dependants) / Math.max(1, maxDependants);
  if (sizeBy === "depth") return Math.max(0.75, 1.3 - 0.15 * n.depth);
  return 1;
}

/** `ui`: { selected, pending: Set }. */
export function buildModel(view, settings, ui) {
  const drop = (n) => settings.known === "hide" && n.status === "known";
  const keep = view.nodes.filter((n) => !drop(n));
  const ids = new Set(keep.map((n) => n.id));
  const implied = new Set(view.implied.map(([a, b]) => `${a}→${b}`));
  const edges = view.edges
    .filter(([a, b]) => ids.has(a) && ids.has(b))
    .filter(([a, b]) => settings.edges === "all" || !implied.has(`${a}→${b}`))
    .map(([dependant, prereq]) => ({ id: `${dependant}→${prereq}`, from: prereq, to: dependant, implied: implied.has(`${dependant}→${prereq}`) }));
  const hues = sectionHues(view);
  const maxDependants = Math.max(...keep.map((n) => n.dependants), 1);
  const px = fontSize(settings);
  const nodes = keep.map((n) => {
    const scale = scaleOf(n, settings.sizeBy, maxDependants);
    const labelled = settings.labels === "all" || (settings.labels === "important" && important(n, ui));
    const isRoot = n.section === n.id;
    const font = titleFont(Math.round(px * Math.min(scale, 1.2)), n.status === "goal" || isRoot);
    const w = Math.round(BOX_W * scale);
    const lines = wrap((n.status === "known" ? "✓ " : "") + n.name, font, w - 20);
    const h = labelled ? 16 + lines.length * Math.round(px * 1.3) + 14 : 16;
    return { id: n.id, n, scale, labelled, isRoot, font, lines, w: labelled ? w : 34, h, hue: hueOf(n, settings.colorBy, hues) };
  });
  const sections = [...hues].map(([id, hue]) => {
    const root = view.nodes.find((n) => n.id === id);
    return { id, hue, name: root?.name ?? id, subtitle: root?.statement ?? "" };
  });
  return { goal: view.goal, nodes, edges, sections, byId: new Map(nodes.map((m) => [m.id, m])) };
}

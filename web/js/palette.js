// Colours, after the FLT blueprint figure and video: ivory paper, clay goal,
// one muted hue per section, warm greys for shared and known concepts.

export const PAPER = "#faf9f5";
export const INK = "#141413";
export const MUTED = "#87867f";
export const CLAY = "#d97757";
export const SHARED = "#8f8b80";
export const KNOWN_FILL = "#ece9e1";

const SECTION_HUES = ["#788c5d", "#6a9bcc", "#c46686", "#d4a12a", "#8b87d6", "#4f9a92", "#b5704a", "#9a7bb0"];
const STATE = { goal: CLAY, unknown: "#5d7ba3", learning: "#d4a12a", known: "#788c5d" };
const DEPTH = ["#c46686", "#8b87d6", "#6a9bcc", "#4f9a92", "#788c5d", "#d4a12a"];

/** Section id → hue, stable for a view: sections sorted by id. */
export function sectionHues(view) {
  const roots = [...new Set(view.nodes.map((n) => n.section).filter(Boolean))].sort();
  return new Map(roots.map((r, i) => [r, SECTION_HUES[i % SECTION_HUES.length]]));
}

/** The hue a node is drawn in under the current colour-by setting. */
export function hueOf(node, colorBy, hues) {
  if (node.status === "goal") return CLAY;
  if (colorBy === "state") return STATE[node.status];
  if (colorBy === "depth") return DEPTH[Math.min(node.depth, DEPTH.length) - 1] ?? DEPTH.at(-1);
  return hues.get(node.section) ?? SHARED;
}

/** `hex` mixed toward white: t = 0 keeps it, t = 1 is white. */
export function tint(hex, t) {
  const n = parseInt(hex.slice(1), 16);
  const ch = [n >> 16, (n >> 8) & 255, n & 255].map((c) => Math.round(c + (255 - c) * t));
  return `rgb(${ch.join(",")})`;
}

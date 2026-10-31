// Edge geometry: an SVG path for one edge, from a prerequisite to the
// concept that needs it, in the layout's own idiom. "flow" is orthogonal
// in the blueprint (merging buses, as in the FLT figure) and a polar elbow
// in the radial tree (as in the FLT video).

const f = (n) => Math.round(n * 10) / 10;

function ortho(points) {
  let d = `M${f(points[0].x)},${f(points[0].y)}`;
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1], b = points[i];
    const mid = i === points.length - 1 ? b.y - 16 : (a.y + b.y) / 2;
    d += ` V${f(mid)} H${f(b.x)} V${f(b.y)}`;
  }
  return d;
}

function boxes(a, b, ma, mb, route, style) {
  const s = { x: a.x, y: a.y + ma.h / 2 };
  const t = { x: b.x, y: b.y - mb.h / 2 };
  if (s.y > t.y) return `M${f(a.x)},${f(a.y - ma.h / 2)} L${f(b.x)},${f(b.y + mb.h / 2)}`;
  if (style === "straight") return `M${f(s.x)},${f(s.y)} L${f(t.x)},${f(t.y)}`;
  if (style === "curved") {
    const k = (t.y - s.y) / 2;
    return `M${f(s.x)},${f(s.y)} C${f(s.x)},${f(s.y + k)} ${f(t.x)},${f(t.y - k)} ${f(t.x)},${f(t.y)}`;
  }
  return ortho([s, ...(route ?? []).filter((p) => p.y > s.y && p.y < t.y), t]);
}

function trim(a, b, ra, rb) {
  const dx = b.x - a.x, dy = b.y - a.y, d = Math.max(1, Math.hypot(dx, dy));
  return [{ x: a.x + (dx / d) * ra, y: a.y + (dy / d) * ra }, { x: b.x - (dx / d) * rb, y: b.y - (dy / d) * rb }];
}

function dots(a, b, ra, rb, style, tree) {
  if (style === "flow" && tree && a.r !== undefined) {
    // From the dependant (inner) out along its angle, around an arc, then
    // out to the prerequisite: the dendrogram elbow.
    const rm = (a.r + b.r) / 2 || b.r / 2;
    const p1 = { x: rm * Math.cos(b.a), y: rm * Math.sin(b.a) };
    const p2 = { x: rm * Math.cos(a.a), y: rm * Math.sin(a.a) };
    let delta = a.a - b.a;
    while (delta > Math.PI) delta -= 2 * Math.PI;
    while (delta < -Math.PI) delta += 2 * Math.PI;
    const [, end] = trim(p2, a, 0, ra);
    if (b.r === 0) return `M${f(b.x)},${f(b.y)} L${f(p2.x)},${f(p2.y)} L${f(end.x)},${f(end.y)}`;
    const arc = `A${f(rm)},${f(rm)} 0 0 ${delta > 0 ? 1 : 0} ${f(p2.x)},${f(p2.y)}`;
    return `M${f(b.x)},${f(b.y)} L${f(p1.x)},${f(p1.y)} ${arc} L${f(end.x)},${f(end.y)}`;
  }
  const [s, t] = trim(a, b, ra, rb);
  if (style === "straight") return `M${f(s.x)},${f(s.y)} L${f(t.x)},${f(t.y)}`;
  // Curved: bow to one side by a fifth of the length.
  const c = { x: (s.x + t.x) / 2 - (t.y - s.y) * 0.2, y: (s.y + t.y) / 2 + (t.x - s.x) * 0.2 };
  return `M${f(s.x)},${f(s.y)} Q${f(c.x)},${f(c.y)} ${f(t.x)},${f(t.y)}`;
}

/** The path for edge `e` given positions, models, the layout and style. */
export function edgePath(e, pos, byId, layout, style, radiusOf) {
  const a = pos.get(e.from), b = pos.get(e.to);
  const ma = byId.get(e.from), mb = byId.get(e.to);
  if (layout.kind === "boxes") return boxes(a, b, ma, mb, layout.routes.get(e.id), style);
  const tree = layout.parent?.get(e.from) === e.to;
  return dots(a, b, radiusOf(ma), radiusOf(mb), style, tree);
}

export const isCrossLink = (e, layout) => layout.kind === "dots" && layout.parent?.get(e.from) !== e.to;

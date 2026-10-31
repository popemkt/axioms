// Blueprint layout: layered top-down with dagre, premises above the
// conclusions they serve and the goal at the bottom, as in the FLT
// blueprint figure. Each section is a dagre cluster, drawn as a panel.

const HEADER = 44;
const PAD = 14;
/** At most this many leaf prerequisites of one concept share a row. */
const FAN = 3;

/** Wide fans of leaf prerequisites (no prerequisites of their own, needed
 *  by one concept only) are wrapped onto stacked rows: an invisible edge
 *  from each leaf to the one FAN places before it lifts it a rank. This
 *  keeps the blueprint closer to the frame's shape, so it can be drawn
 *  larger. Returns the invisible edges as [upper, lower] pairs. */
function wraps(model) {
  const into = new Map(), out = new Map();
  for (const e of model.edges) {
    (into.get(e.to) ?? into.set(e.to, []).get(e.to)).push(e.from);
    out.set(e.from, (out.get(e.from) ?? 0) + 1);
  }
  const pairs = [];
  for (const sources of into.values()) {
    const leaves = sources.filter((s) => !into.has(s) && out.get(s) === 1).sort();
    for (let i = FAN; i < leaves.length; i++) pairs.push([leaves[i], leaves[i - FAN]]);
  }
  return pairs;
}

export function blueprint(model) {
  const g = new dagre.graphlib.Graph({ compound: true, multigraph: false });
  g.setGraph({ rankdir: "TB", nodesep: 18, ranksep: 60, edgesep: 10, marginx: 0, marginy: 0 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const s of model.sections) g.setNode(`§${s.id}`, {});
  for (const m of model.nodes) {
    g.setNode(m.id, { width: m.w, height: m.h });
    if (m.n.section) g.setParent(m.id, `§${m.n.section}`);
  }
  for (const e of model.edges) g.setEdge(e.from, e.to);
  for (const [upper, lower] of wraps(model)) g.setEdge(upper, lower, { minlen: 1, weight: 0 });
  dagre.layout(g);

  const pos = new Map(model.nodes.map((m) => [m.id, { x: g.node(m.id).x, y: g.node(m.id).y }]));
  const routes = new Map(model.edges.map((e) => [e.id, (g.edge(e.from, e.to)?.points ?? []).slice(1, -1)]));
  const sections = [];
  for (const s of model.sections) {
    const members = model.nodes.filter((m) => m.n.section === s.id);
    if (!members.length) continue;
    const x1 = Math.min(...members.map((m) => pos.get(m.id).x - m.w / 2)) - PAD;
    const x2 = Math.max(...members.map((m) => pos.get(m.id).x + m.w / 2)) + PAD;
    const y1 = Math.min(...members.map((m) => pos.get(m.id).y - m.h / 2)) - PAD - HEADER;
    const y2 = Math.max(...members.map((m) => pos.get(m.id).y + m.h / 2)) + PAD;
    sections.push({ ...s, x: x1, y: y1, w: x2 - x1, h: y2 - y1 });
  }
  return { kind: "boxes", pos, routes, sections };
}

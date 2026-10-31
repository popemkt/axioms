// Blueprint layout: layered top-down with dagre, premises above the
// conclusions they serve and the goal at the bottom, as in the FLT
// blueprint figure. Each section is a dagre cluster, drawn as a panel.

const HEADER = 44;
const PAD = 14;

export function blueprint(model) {
  const g = new dagre.graphlib.Graph({ compound: true, multigraph: false });
  g.setGraph({ rankdir: "TB", nodesep: 22, ranksep: 64, edgesep: 12, marginx: 20, marginy: 20 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const s of model.sections) g.setNode(`§${s.id}`, {});
  for (const m of model.nodes) {
    g.setNode(m.id, { width: m.w, height: m.h });
    if (m.n.section) g.setParent(m.id, `§${m.n.section}`);
  }
  for (const e of model.edges) g.setEdge(e.from, e.to);
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
  return { kind: "boxes", pos, routes, sections, labels: [] };
}

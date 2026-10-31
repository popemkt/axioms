// Radial layout: the goal at the centre and each section fanned out as a
// wedge, like the FLT formalisation video. Positions come from a spanning
// tree (each concept hangs under its shallowest dependant); the other
// "needs" edges are drawn as cross-links.

const RING = 120;

export function spanningTree(model) {
  const deps = new Map();
  for (const e of model.edges) (deps.get(e.from) ?? deps.set(e.from, []).get(e.from)).push(e.to);
  const parent = new Map();
  for (const m of model.nodes) {
    if (m.id === model.goal) continue;
    const cands = (deps.get(m.id) ?? []).map((id) => model.byId.get(id)).filter(Boolean);
    // Prefer a dependant in the same section, so a branch keeps one colour;
    // then the shallowest; then by id.
    const other = (c) => (c.n.section === m.n.section || c.id === model.goal ? 0 : 1);
    cands.sort((a, b) => other(a) - other(b) || a.n.depth - b.n.depth || a.id.localeCompare(b.id));
    parent.set(m.id, cands[0]?.id ?? model.goal);
  }
  const children = new Map(model.nodes.map((m) => [m.id, []]));
  for (const [c, p] of parent) children.get(p)?.push(c);
  const key = (id) => `${model.byId.get(id).n.section ?? "~"} ${model.byId.get(id).n.name}`;
  for (const list of children.values()) list.sort((a, b) => key(a).localeCompare(key(b)));
  return { parent, children };
}

export function radial(model) {
  const { parent, children } = spanningTree(model);
  const leaves = new Map();
  const count = (id) => {
    const kids = children.get(id) ?? [];
    const n = kids.length ? kids.reduce((s, k) => s + count(k), 0) : 1;
    leaves.set(id, n);
    return n;
  };
  const total = count(model.goal);
  const angle = new Map();
  const place = (id, from) => {
    const span = (2 * Math.PI * leaves.get(id)) / total;
    angle.set(id, from + span / 2);
    let at = from;
    for (const k of children.get(id) ?? []) {
      place(k, at);
      at += (2 * Math.PI * leaves.get(k)) / total;
    }
  };
  place(model.goal, -Math.PI / 2);
  const level = new Map([[model.goal, 0]]);
  const lvl = (id) => level.get(id) ?? level.set(id, lvl(parent.get(id)) + 1).get(id);
  const ring = Math.max(RING, (total * 26) / (2 * Math.PI * Math.max(1, ...model.nodes.map((m) => lvl(m.id)))));
  const pos = new Map();
  for (const m of model.nodes) {
    const r = m.id === model.goal ? 0 : lvl(m.id) * ring;
    pos.set(m.id, { x: r * Math.cos(angle.get(m.id)), y: r * Math.sin(angle.get(m.id)), r, a: angle.get(m.id) });
  }
  const outer = Math.max(...[...pos.values()].map((p) => p.r)) + 70;
  const labels = model.sections
    .filter((s) => angle.has(s.id))
    .map((s) => {
      const a = angle.get(s.id);
      return { id: s.id, text: s.name, hue: s.hue, x: outer * Math.cos(a), y: outer * Math.sin(a), anchor: Math.cos(a) >= 0 ? "start" : "end" };
    });
  return { kind: "dots", pos, routes: new Map(), sections: [], labels, parent };
}

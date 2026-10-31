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
  // One leaf slot per leaf, plus a gap slot between sections (the wedges
  // of the FLT video are separated by empty sky).
  const tops = children.get(model.goal) ?? [];
  const GAP = tops.length > 1 ? 1.2 : 0;
  const slot = (2 * Math.PI) / (count(model.goal) + GAP * tops.length);
  const angle = new Map();
  const place = (id, from) => {
    angle.set(id, from + (slot * leaves.get(id)) / 2);
    let at = from;
    for (const k of children.get(id) ?? []) {
      place(k, at);
      at += slot * leaves.get(k) + (id === model.goal ? slot * GAP : 0);
    }
  };
  place(model.goal, -Math.PI / 2 + (slot * GAP) / 2);
  angle.set(model.goal, 0);
  const total = count(model.goal);
  const level = new Map([[model.goal, 0]]);
  const lvl = (id) => level.get(id) ?? level.set(id, lvl(parent.get(id)) + 1).get(id);
  const ring = Math.max(RING, (total * 26) / (2 * Math.PI * Math.max(1, ...model.nodes.map((m) => lvl(m.id)))));
  const pos = new Map();
  for (const m of model.nodes) {
    const r = m.id === model.goal ? 0 : lvl(m.id) * ring;
    pos.set(m.id, { x: r * Math.cos(angle.get(m.id)), y: r * Math.sin(angle.get(m.id)), r, a: angle.get(m.id) });
  }
  return { kind: "dots", pos, routes: new Map(), sections: [], parent };
}

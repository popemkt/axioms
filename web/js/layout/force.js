// Force layout: start from the radial positions, then relax with springs on
// "needs" edges, repulsion between concepts, and a pull toward the centre.
// The goal stays pinned at the centre. Deterministic: no randomness.

import { radial } from "./radial.js";

export function force(model) {
  const base = radial(model);
  const ids = model.nodes.map((m) => m.id);
  const p = new Map(ids.map((id) => [id, { ...base.pos.get(id) }]));
  const n = ids.length;
  const k = 90;
  let temp = 40;
  for (let it = 0; it < 260; it++, temp *= 0.985) {
    const d = new Map(ids.map((id) => [id, { x: 0, y: 0 }]));
    for (let i = 0; i < n; i++)
      for (let j = i + 1; j < n; j++) {
        const a = p.get(ids[i]), b = p.get(ids[j]);
        let dx = a.x - b.x, dy = a.y - b.y;
        const dist = Math.max(1, Math.hypot(dx, dy));
        const f = (k * k) / dist;
        dx = (dx / dist) * f; dy = (dy / dist) * f;
        d.get(ids[i]).x += dx; d.get(ids[i]).y += dy;
        d.get(ids[j]).x -= dx; d.get(ids[j]).y -= dy;
      }
    for (const e of model.edges) {
      const a = p.get(e.from), b = p.get(e.to);
      const dx = a.x - b.x, dy = a.y - b.y;
      const dist = Math.max(1, Math.hypot(dx, dy));
      const f = (dist * dist) / k;
      d.get(e.from).x -= (dx / dist) * f; d.get(e.from).y -= (dy / dist) * f;
      d.get(e.to).x += (dx / dist) * f; d.get(e.to).y += (dy / dist) * f;
    }
    for (const id of ids) {
      if (id === model.goal) continue;
      const q = p.get(id), v = d.get(id);
      v.x -= q.x * 0.05; v.y -= q.y * 0.05;
      const len = Math.max(1, Math.hypot(v.x, v.y));
      q.x += (v.x / len) * Math.min(len, temp);
      q.y += (v.y / len) * Math.min(len, temp);
    }
  }
  for (const q of p.values()) delete q.r, delete q.a; // polar coordinates no longer hold
  return { kind: "dots", pos: p, routes: new Map(), sections: [], labels: [], parent: base.parent };
}

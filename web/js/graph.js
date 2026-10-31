// The blueprint graph: Cytoscape.js with a top-down dagre layout. `render`
// diffs the new view against what is drawn, so expand and collapse animate.

const COLORS = {
  goal: { bg: "#5b4bdb", border: "#3d2fb0", text: "#fff" },
  unknown: { bg: "#ffffff", border: "#5a7bb5", text: "#1d2433" },
  learning: { bg: "#ffe9a8", border: "#d79a00", text: "#3b2c00" },
  known: { bg: "#cfeccf", border: "#3f9a4a", text: "#1f4d24" },
};

const style = [
  {
    selector: "node",
    style: {
      shape: "round-rectangle",
      width: 168,
      height: 50,
      label: "data(label)",
      "text-wrap": "wrap",
      "text-max-width": 152,
      "text-valign": "center",
      "font-size": 12,
      "font-family": "system-ui, sans-serif",
      color: (n) => COLORS[n.data("status")].text,
      "background-color": (n) => COLORS[n.data("status")].bg,
      "border-color": (n) => COLORS[n.data("status")].border,
      "border-width": 2,
      opacity: 1,
      "transition-property": "background-color, border-color, opacity, border-width",
      "transition-duration": 300,
    },
  },
  { selector: "node[status = 'goal']", style: { "font-weight": 700, "border-width": 3, width: 190 } },
  { selector: "node[status = 'known']", style: { opacity: 0.6 } },
  { selector: "node[?leaf]", style: { "border-style": "dashed" } },
  { selector: "node:selected", style: { "border-width": 4, "border-color": "#ff6a3d", opacity: 1 } },
  { selector: "node.pending", style: { "border-color": "#ff6a3d", "border-style": "dotted" } },
  {
    selector: "edge",
    style: {
      width: 1.6,
      "curve-style": "bezier",
      "line-color": "#9aa8c2",
      "target-arrow-color": "#9aa8c2",
      "target-arrow-shape": "triangle",
      "arrow-scale": 0.9,
    },
  },
];

const MIN_READABLE_ZOOM = 0.6;

const layoutOptions = {
  name: "dagre",
  rankDir: "TB",
  nodeSep: 24,
  rankSep: 56,
  animate: true,
  animationDuration: 450,
  animationEasing: "ease-in-out-cubic",
  fit: true,
  padding: 24,
};

function label(n) {
  return n.hidden_below > 0 ? `${n.name}\n＋${n.hidden_below} below` : n.name;
}

export function createGraph(container, { onSelect }) {
  const cy = cytoscape({
    container,
    style,
    elements: [],
    minZoom: 0.15,
    maxZoom: 2.5,
    wheelSensitivity: 0.3,
    boxSelectionEnabled: false,
    autoungrabify: true,
  });
  cy.on("tap", "node", (e) => onSelect(e.target.id()));
  cy.on("tap", (e) => e.target === cy && onSelect(null));

  let current = new Map();

  function render(view) {
    const wanted = new Map();
    for (const n of view.nodes) {
      const data = {
        id: n.id,
        label: label(n),
        status: n.status,
        leaf: !n.decomposed && n.status !== "known",
      };
      wanted.set(n.id, data);
    }
    for (const [from, to] of view.edges) {
      const id = `${from}→${to}`;
      wanted.set(id, { id, source: from, target: to });
    }
    current = wanted;
    // Fade out what left; remove it only if a later render has not wanted it back.
    const leaving = cy.elements().filter((el) => !wanted.has(el.id()));
    leaving.animate(
      { style: { opacity: 0 } },
      { duration: 250, complete: () => leaving.filter((el) => !current.has(el.id())).remove() },
    );
    const added = [];
    for (const [id, data] of wanted) {
      const el = cy.getElementById(id);
      if (el.nonempty()) el.stop(true).removeStyle("opacity").data(data);
      else added.push({ group: data.source ? "edges" : "nodes", data });
    }
    const fresh = cy.add(added);
    // New nodes grow out of the concept that needs them.
    fresh.nodes().forEach((n) => {
      const parent = n.incomers("node").first();
      if (parent.nonempty()) n.position({ ...parent.position() });
    });
    fresh.style("opacity", 0);
    cy.elements()
      .difference(leaving)
      .layout({ ...layoutOptions, stop: readable })
      .run();
    fresh.animate({ style: { opacity: 1 } }, { duration: 450 });
    setTimeout(() => fresh.removeStyle("opacity"), 500);
  }

  // A fitted wide graph can be too small to read on a phone: zoom in to a
  // readable floor, keeping the goal at the top centre. Pan to see the rest.
  function readable() {
    if (cy.zoom() >= MIN_READABLE_ZOOM) return;
    const goal = cy.nodes("[status = 'goal']").first();
    if (goal.empty()) return;
    const p = goal.position();
    cy.animate(
      { zoom: MIN_READABLE_ZOOM, pan: { x: cy.width() / 2 - p.x * MIN_READABLE_ZOOM, y: 40 - p.y * MIN_READABLE_ZOOM + 24 } },
      { duration: 300 },
    );
  }

  return {
    cy,
    render,
    clear: () => cy.elements().remove(),
    fit: () => cy.animate({ fit: { eles: cy.elements(), padding: 24 } }, { duration: 350 }),
    select(id) {
      cy.$(":selected").unselect();
      if (!id) return;
      const n = cy.getElementById(id);
      n.select();
      cy.animate({ center: { eles: n } }, { duration: 300 });
    },
    pending(id, on) {
      cy.getElementById(id).toggleClass("pending", on);
    },
  };
}

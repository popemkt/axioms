// The blueprint graph: view → drawing model → layout → SVG scene, redrawn
// when the view or the display settings change. Rendering is our own SVG
// (see docs/DESIGN.md → Rendering) so it can look like the FLT blueprint.

import { buildModel } from "./model.js";
import { layout } from "./layout/index.js";
import { createScene } from "./render/scene.js";

export function createGraph(container, { settings, onSelect }) {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.classList.add("scene");
  const tip = Object.assign(document.createElement("div"), { className: "tip", hidden: true });
  container.append(svg, tip);
  const ui = { selected: null, pending: new Set() };
  let view = null;
  let model = null;

  const scene = createScene(svg, {
    onSelect,
    onHover(id, e) {
      scene.highlight(id);
      const m = id && model?.byId.get(id);
      if (!m || (m.labelled && settings.get().layout === "blueprint")) return (tip.hidden = true);
      const r = container.getBoundingClientRect();
      tip.innerHTML = "";
      tip.append(Object.assign(document.createElement("b"), { textContent: m.n.name }), Object.assign(document.createElement("code"), { textContent: m.id }));
      tip.style.left = `${e.clientX - r.left + 12}px`;
      tip.style.top = `${e.clientY - r.top + 12}px`;
      tip.hidden = false;
    },
  });

  function render(opts = {}) {
    if (!view) return;
    const s = settings.get();
    model = buildModel(view, s, ui);
    scene.draw(model, layout(s.layout, model), s, ui, opts);
  }

  settings.onChange(() => render());

  return {
    render(next, opts) {
      view = next;
      render(opts);
    },
    replay: () => render({ fresh: true }),
    clear() {
      view = null;
      scene.clear();
    },
    fit: () => scene.fit(),
    select(id, center = false) {
      ui.selected = id;
      svg.querySelectorAll(".node.selected").forEach((g) => g.classList.remove("selected"));
      if (!id) return;
      scene.nodeEl(id)?.classList.add("selected");
      if (center) scene.center(id);
    },
    pending(id, on) {
      if (on) ui.pending.add(id);
      else ui.pending.delete(id);
      scene.nodeEl(id)?.classList.toggle("pending", on);
    },
  };
}

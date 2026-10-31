// The Display panel: one segmented control per setting in the schema.

import { SCHEMA } from "./settings.js";

export function createDisplay(root, settings, { onReplay }) {
  function render() {
    const v = settings.get();
    root.innerHTML = "";
    const head = document.createElement("header");
    head.innerHTML = `<b>Display</b>`;
    const close = Object.assign(document.createElement("button"), { className: "close", textContent: "×", ariaLabel: "Close" });
    close.onclick = () => (root.hidden = true);
    head.append(close);
    root.append(head);
    for (const s of SCHEMA) {
      const row = document.createElement("div");
      row.className = "row";
      row.append(Object.assign(document.createElement("span"), { textContent: s.label }));
      const seg = document.createElement("div");
      seg.className = "seg";
      for (const [value, label] of s.options) {
        const b = Object.assign(document.createElement("button"), { textContent: label, type: "button" });
        b.classList.toggle("on", v[s.key] === value);
        b.onclick = () => settings.set(s.key, value);
        seg.append(b);
      }
      row.append(seg);
      root.append(row);
    }
    const foot = document.createElement("div");
    foot.className = "foot";
    const replay = Object.assign(document.createElement("button"), { textContent: "▶ Replay growth", type: "button" });
    replay.onclick = onReplay;
    const reset = Object.assign(document.createElement("button"), { textContent: "Reset", type: "button" });
    reset.onclick = () => settings.reset();
    foot.append(replay, reset);
    root.append(foot);
  }
  settings.onChange(render);
  render();
  return { toggle: () => (root.hidden = !root.hidden) };
}

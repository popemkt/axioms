// The side panel: the selected concept's card and the roadmap. Rendering
// only: actions are reported through the callbacks given to `createPanel`.

const esc = (s) =>
  String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

const STATUS_TEXT = { goal: "goal", unknown: "unknown", learning: "learning", known: "known · one of your axioms" };

export function createPanel({ card, roadmap }, { onKnowledge, onDecompose, onSelect, onClose }) {
  card.addEventListener("click", (e) => {
    const b = e.target.closest("button[data-act]");
    if (!b) return;
    const { act, id } = b.dataset;
    if (act === "decompose") onDecompose(id);
    else if (act === "close") onClose();
    else onKnowledge(id, act);
  });
  roadmap.addEventListener("click", (e) => {
    const box = e.target.closest("input[data-known]");
    if (box) return onKnowledge(box.dataset.known, box.checked ? "known" : "unknown");
    const step = e.target.closest("[data-step]");
    if (step && !e.target.closest("summary, details[open] *")) onSelect(step.dataset.step);
  });

  function showCard(node, { busy, names }) {
    if (!node) {
      card.hidden = true;
      return;
    }
    const c = node;
    const k = c.knowledge;
    const btn = (act, text, on) =>
      `<button data-act="${act}" data-id="${esc(c.id)}" class="${on ? "on" : ""}" ${busy ? "disabled" : ""}>${text}</button>`;
    const below = c.hidden_below ? `<p class="muted">${c.hidden_below} concept(s) below are hidden because you know this. Mark it unknown to expand.</p>` : "";
    card.innerHTML = `
      <header><span class="dot ${esc(c.status)}"></span><span class="status">${esc(STATUS_TEXT[c.status])}</span>
        <button data-act="close" class="close" aria-label="Close">×</button></header>
      <h2>${esc(c.name)}</h2>
      <p class="statement">${esc(c.statement) || '<span class="muted">No statement yet.</span>'}</p>
      ${c.explanation ? `<p>${esc(c.explanation)}</p>` : ""}
      ${c.check ? `<p class="check"><b>Check:</b> ${esc(c.check)}</p>` : ""}
      ${below}
      <div class="actions">
        ${btn("known", "✓ I know this", k === "known")}
        ${btn("learning", "Learning", k === "learning")}
        ${btn("unknown", "Unknown", k === "unknown")}
        ${btn("decompose", busy ? "Decomposing…" : c.decomposed ? "Decompose more" : "Don't get it: decompose")}
      </div>
      ${names.length ? `<p class="muted">Needed for: ${names.map(esc).join(", ")}</p>` : ""}`;
    card.hidden = false;
  }

  function showRoadmap(r, nodes) {
    const name = (id) => nodes.get(id)?.name ?? id;
    if (!r.steps.length) {
      roadmap.innerHTML = `<h2>Roadmap</h2><p class="muted">Nothing left: you know the goal.</p>`;
      return;
    }
    const items = r.steps
      .map((s, i) => {
        const builds = s.builds_on.length
          ? `<p class="builds">builds on ${s.builds_on.map((b) => `<span class="chip">${esc(name(b))}</span>`).join(" ")}</p>`
          : "";
        return `<li data-step="${esc(s.id)}" class="${esc(s.knowledge)}">
          <label class="tick" title="I know this"><input type="checkbox" data-known="${esc(s.id)}" ${s.knowledge === "known" ? "checked" : ""}></label>
          <div><h3><span class="n">${i + 1}</span>${esc(s.name)}</h3>
            <p class="statement">${esc(s.statement)}</p>${builds}
            <details><summary>Explain &amp; check</summary><p>${esc(s.explanation)}</p><p class="check"><b>Check:</b> ${esc(s.check)}</p></details>
          </div></li>`;
      })
      .join("");
    roadmap.innerHTML = `<h2>Roadmap <span class="muted">${r.steps.length} steps, from what you know up to the goal</span></h2><ol>${items}</ol>`;
  }

  return { showCard, showRoadmap };
}

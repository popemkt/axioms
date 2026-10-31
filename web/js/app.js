// Wiring: the current goal, its blueprint, the selection, and the calls
// that change them. The URL hash (#/topic/<id>) names the goal.

import * as api from "./api.js";
import { createGraph } from "./graph.js";
import { createPanel } from "./panel.js";
import { createSettings } from "./settings.js";
import { createDisplay } from "./display.js";
import { connectLive } from "./live.js";

const $ = (id) => document.getElementById(id);
const state = { goal: null, bp: null, nodes: new Map(), selected: null, pending: new Set(), provider: "" };
// Where reads and writes go: the JSON API, or SpacetimeDB in live mode
// (same shapes, see live.js).
let backend = {
  blueprint: (id) => api.blueprint(id),
  setKnowledge: api.setKnowledge,
  decompose: api.decompose,
  learn: (topic) => api.learn(topic),
};

const settings = createSettings();
const graph = createGraph($("graph"), { settings, onSelect: select });
const display = createDisplay($("display"), settings, { onReplay: () => graph.replay() });
const panel = createPanel(
  { card: $("card"), roadmap: $("roadmap") },
  {
    onKnowledge: (id, k) => run(() => backend.setKnowledge(id, k, state.goal)),
    onDecompose: decompose,
    onSelect: (id) => select(id, true),
    onClose: () => select(null),
  },
);

function toast(msg) {
  const t = $("toast");
  t.textContent = msg;
  t.hidden = false;
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => (t.hidden = true), 6000);
}

function busy(text) {
  $("busy").hidden = !text;
  $("busy-text").textContent = text || "";
  document.body.classList.toggle("busy", !!text);
}

function show(bp) {
  state.bp = bp;
  state.goal = bp.view.goal;
  state.nodes = new Map(bp.view.nodes.map((n) => [n.id, n]));
  $("empty").hidden = true;
  graph.render(bp.view, { fit: true });
  panel.showRoadmap(bp.roadmap, state.nodes);
  if (state.selected && !state.nodes.has(state.selected)) state.selected = null;
  showCard();
  const want = `#/topic/${encodeURIComponent(state.goal)}`;
  if (location.hash !== want) history.replaceState(null, "", want);
  $("topics").value = state.goal;
}

function showCard() {
  const n = state.nodes.get(state.selected);
  const names = n ? state.bp.view.edges.filter(([, to]) => to === n.id).map(([from]) => state.nodes.get(from)?.name) : [];
  panel.showCard(n, { busy: n && state.pending.has(n.id), names });
}

function select(id, center = false) {
  state.selected = id;
  graph.select(id, center);
  showCard();
}

// Answers for a goal the user has since left are dropped; on failure the
// last good blueprint is redrawn so optimistic inputs (checkboxes) resync.
async function run(fn) {
  const goal = state.goal;
  try {
    const bp = await fn();
    if (state.goal === goal) show(bp);
  } catch (e) {
    toast(e.message);
    if (state.bp) show(state.bp);
  }
}

async function decompose(id) {
  if (state.pending.has(id)) return;
  state.pending.add(id);
  graph.pending(id, true);
  showCard();
  busy(`Decomposing “${state.nodes.get(id)?.name}” with ${state.provider}…`);
  const goal = state.goal;
  try {
    const bp = await backend.decompose(id, goal);
    if (state.goal === goal) show(bp);
  } catch (e) {
    toast(e.message);
  } finally {
    state.pending.delete(id);
    graph.pending(id, false);
    busy(state.pending.size ? `Decomposing ${state.pending.size} concept(s)…` : "");
    showCard();
  }
}

async function learn(topic) {
  busy(`Decomposing “${topic}” two levels deep with ${state.provider}… (about 30–60 s)`);
  $("topic").value = topic;
  try {
    const bp = await backend.learn(topic, (id) => open(id));
    state.selected = null;
    graph.clear();
    show(bp);
    refreshTopics();
  } catch (e) {
    toast(e.message);
  } finally {
    busy(state.pending.size ? `Decomposing ${state.pending.size} concept(s)…` : "");
  }
}

async function open(id) {
  const ticket = (open.ticket = (open.ticket || 0) + 1);
  try {
    const bp = await backend.blueprint(id);
    if (ticket !== open.ticket) return;
    if (id !== state.goal) {
      graph.clear();
      state.selected = null;
    }
    show(bp);
    $("topic").value = bp.view.nodes.find((n) => n.id === id)?.name ?? "";
  } catch (e) {
    toast(e.message);
  }
}

async function refreshTopics() {
  const list = await api.topics();
  const sel = $("topics");
  sel.replaceChildren(new Option(`Your topics (${list.length})…`, ""), ...list.map((t) => new Option(t.name, t.id)));
  sel.value = state.goal ?? "";
}

$("learn").addEventListener("submit", (e) => {
  e.preventDefault();
  const t = $("topic").value.trim();
  if (t) learn(t);
});
$("topics").addEventListener("change", (e) => e.target.value && open(e.target.value));
$("fit").addEventListener("click", () => graph.fit());
$("display-toggle").addEventListener("click", () => display.toggle());
document.querySelectorAll("[data-topic]").forEach((b) => b.addEventListener("click", () => learn(b.dataset.topic)));
window.addEventListener("hashchange", route);

function route() {
  const m = location.hash.match(/^#\/topic\/(.+)$/);
  let id = null;
  try {
    id = m && decodeURIComponent(m[1]);
  } catch {
    return;
  }
  if (id && id !== state.goal) open(id);
}

// Live: redraw on any change, and show every device's pending requests.
async function goLive(info) {
  const badge = $("live");
  backend = await connectLive(info, {
    onChange: () => {
      if (state.goal) run(() => backend.blueprint(state.goal));
      refreshTopics();
    },
    onPending: (ids) => {
      for (const id of state.nodes.keys()) graph.pending(id, ids.has(id) || state.pending.has(id));
      if (!state.pending.size) busy(ids.size ? `Decomposing ${ids.size} concept(s), live…` : "");
    },
    // Reconnect by reloading (the hash keeps the goal); a failed reconnect
    // shows its error rather than looping.
    onLost: () => {
      badge.classList.add("lost");
      badge.textContent = "reconnecting…";
      setTimeout(() => location.reload(), 5000);
    },
  });
  badge.hidden = false;
  badge.title = `Live as ${backend.identity().slice(0, 12)}… Click to copy a link that makes another device you.`;
  badge.onclick = () =>
    navigator.clipboard
      .writeText(backend.deviceLink())
      .then(() => toast("Device link copied: open it on your other device to be you there."))
      .catch(() => prompt("Open this link on your other device:", backend.deviceLink()));
}

(async () => {
  try {
    const meta = await api.meta();
    state.provider = meta.provider;
    if (meta.live) await goLive(meta.live);
  } catch (e) {
    toast(e.message);
  }
  await refreshTopics();
  route();
})();

// Wiring: the current goal, its blueprint, the selection, and the calls
// that change them. The URL hash (#/topic/<id>) names the goal.

import * as api from "./api.js";
import { createGraph } from "./graph.js";
import { createPanel } from "./panel.js";

const $ = (id) => document.getElementById(id);
const state = { goal: null, bp: null, nodes: new Map(), selected: null, pending: new Set(), provider: "" };

const graph = createGraph($("graph"), { onSelect: select });
const panel = createPanel(
  { card: $("card"), roadmap: $("roadmap") },
  {
    onKnowledge: (id, k) => run(() => api.setKnowledge(id, k, state.goal)),
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
  graph.render(bp.view);
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
  if (center || !id) graph.select(id);
  showCard();
}

async function run(fn) {
  try {
    show(await fn());
  } catch (e) {
    toast(e.message);
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
    const bp = await api.decompose(id, goal);
    if (state.goal === goal) show(bp);
  } catch (e) {
    toast(e.message);
  } finally {
    state.pending.delete(id);
    graph.pending(id, false);
    busy(state.pending.size ? $("busy-text").textContent : "");
    showCard();
  }
}

async function learn(topic) {
  busy(`Decomposing “${topic}” two levels deep with ${state.provider}… (about 30–60 s)`);
  $("topic").value = topic;
  try {
    const bp = await api.learn(topic);
    state.selected = null;
    graph.clear();
    show(bp);
    refreshTopics();
  } catch (e) {
    toast(e.message);
  } finally {
    busy("");
  }
}

async function open(id) {
  try {
    const bp = await api.blueprint(id);
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
  $("topics").innerHTML =
    `<option value="">Your topics (${list.length})…</option>` +
    list.map((t) => `<option value="${t.id}">${t.name.replace(/</g, "&lt;")}</option>`).join("");
  $("topics").value = state.goal ?? "";
}

$("learn").addEventListener("submit", (e) => {
  e.preventDefault();
  const t = $("topic").value.trim();
  if (t) learn(t);
});
$("topics").addEventListener("change", (e) => e.target.value && open(e.target.value));
$("fit").addEventListener("click", () => graph.fit());
document.querySelectorAll("[data-topic]").forEach((b) => b.addEventListener("click", () => learn(b.dataset.topic)));
window.addEventListener("hashchange", route);

function route() {
  const m = location.hash.match(/^#\/topic\/(.+)$/);
  if (m && decodeURIComponent(m[1]) !== state.goal) open(decodeURIComponent(m[1]));
}

(async () => {
  try {
    state.provider = (await api.meta()).provider;
  } catch (e) {
    toast(e.message);
  }
  await refreshTopics();
  route();
})();

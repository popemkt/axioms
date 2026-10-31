// Live mode: the browser is a SpacetimeDB client. It subscribes to the
// shared graph, so a change from any device (a phone marking known, the
// worker writing a decomposition step) arrives here at once, and it writes
// through reducers as its own identity. Views and roadmaps still come from
// the server, computed by axioms-core for this identity (`?as=`).

import * as api from "./api.js";

const TABLES = ["concept", "need", "topic", "knowledge", "request"];
const open = (r) => r.status === "pending" || r.status === "working";

export async function connectLive({ uri, db }, { onChange, onPending, onLost }) {
  const { DbConnection } = await import("../vendor/spacetimedb.js");
  const key = `axioms.stdb.${uri}.${db}.token`;
  // A device link (#/device/<token>) makes this browser the same person as
  // the one that made it. The token is a bearer secret: tailnet only.
  const link = location.hash.match(/^#\/device\/([^/]+)$/);
  if (link) {
    localStorage.setItem(key, decodeURIComponent(link[1]));
    history.replaceState(null, "", location.pathname);
  }
  const waits = new Map(); // request id → { resolve, reject }
  let me = "";

  const conn = await new Promise((resolve, reject) => {
    const c = DbConnection.builder()
      .withUri(uri)
      .withDatabaseName(db)
      .withToken(localStorage.getItem(key) || undefined)
      .onConnect((conn, identity, token) => {
        localStorage.setItem(key, token);
        me = identity.toHexString();
        conn
          .subscriptionBuilder()
          .onApplied(() => resolve(conn))
          .onError((_, e) => reject(new Error(`subscribe: ${e?.message ?? e}`)))
          .subscribe(TABLES.map((t) => `SELECT * FROM ${t}`));
      })
      .onConnectError((_, e) => reject(new Error(`SpacetimeDB: ${e?.message ?? e}`)))
      .onDisconnect(() => onLost?.())
      .build();
    return c;
  });

  const requests = () => [...conn.db.request.iter()];
  const pending = () => {
    const ids = new Map();
    for (const r of requests().filter(open)) ids.set(r.kind === "topic" ? r.concept : r.text, r);
    ids.delete("");
    return ids;
  };
  const settle = (r) => {
    const w = waits.get(r.id);
    if (!w) return;
    if (r.kind === "topic" && r.concept && !w.opened) {
      w.opened = true;
      w.onTopic?.(r.concept);
    }
    if (open(r)) return;
    waits.delete(r.id);
    r.status === "done" ? w.resolve(r) : w.reject(new Error(r.note || "decomposition failed"));
  };
  let timer = 0;
  const changed = (table) => () => {
    clearTimeout(timer);
    timer = setTimeout(() => onChange(), 120);
    if (table === "request") onPending(pending());
  };
  for (const t of TABLES) {
    conn.db[t].onInsert(changed(t));
    conn.db[t].onDelete(changed(t));
    conn.db[t].onUpdate?.(changed(t));
  }
  conn.db.request.onInsert((_, r) => settle(r));
  conn.db.request.onUpdate((_, __, r) => settle(r));

  // A request I just opened: the newest of mine matching kind and text.
  const mine = (kind, text) =>
    requests()
      .filter((r) => r.by.toHexString() === me && r.kind === kind && r.text.toLowerCase() === text.toLowerCase())
      .sort((a, b) => (a.id < b.id ? 1 : -1))[0];
  const until = (r, onTopic) =>
    new Promise((resolve, reject) => {
      waits.set(r.id, { resolve, reject, onTopic });
      settle(r);
    });

  return {
    identity: () => me,
    deviceLink: () =>
      `${location.origin}${location.pathname}#/device/${encodeURIComponent(localStorage.getItem(key))}`,
    pending,
    blueprint: (id) => api.blueprint(id, me),
    async setKnowledge(id, knowledge, goal) {
      await conn.reducers.mark({ concept: id, state: knowledge });
      return api.blueprint(goal, me);
    },
    // Resolves when the worker is done; the steps stream in before that.
    async decompose(id, goal) {
      await conn.reducers.requestDecompose({ concept: id, goal });
      await until(mine("decompose", id));
      return api.blueprint(goal, me);
    },
    async learn(topic, onTopic) {
      await conn.reducers.requestTopic({ name: topic, depth: 2 });
      const r = await until(mine("topic", topic.trim()), onTopic);
      return api.blueprint(r.concept, me);
    },
  };
}

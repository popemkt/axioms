// The JSON API, one function per endpoint. Each mutating call answers with
// the goal's blueprint: { view, roadmap }.

async function call(method, path, body) {
  const res = await fetch(path, {
    method,
    headers: body ? { "content-type": "application/json" } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error || `${res.status} ${res.statusText}`);
  return data;
}

export const meta = () => call("GET", "/api/meta");
export const topics = () => call("GET", "/api/topics");
// `as`: in live mode, the identity whose knowledge the view shows.
export const blueprint = (id, as) =>
  call("GET", `/api/topics/${encodeURIComponent(id)}${as ? `?as=${as}` : ""}`);
export const learn = (topic) => call("POST", "/api/topics", { topic });
export const setKnowledge = (id, knowledge, goal) =>
  call("POST", `/api/concepts/${encodeURIComponent(id)}/knowledge`, { knowledge, goal });
export const decompose = (id, goal) =>
  call("POST", `/api/concepts/${encodeURIComponent(id)}/decompose`, { goal });

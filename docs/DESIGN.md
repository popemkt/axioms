# axioms: design

## Graph model

- A **concept** has an `id` (its normalised name, suffixed `-2`, `-3` … when
  distinct concepts share a name), a `name`, a one-line canonical
  `statement`, an `explanation`, a `check` question, and `knowledge`
  (`unknown | learning | known`). `known` makes it one of the learner's
  **axioms**. Knowledge belongs to the concept, not to a topic, which is why
  axioms carry over between topics.
- An **edge** `A → B` means *A needs B*. The graph is one global DAG
  (`core::ConceptGraph`, on petgraph). `add_need` refuses self-loops and any
  edge that would close a cycle. Merges count those edges as `dropped_edges`
  rather than failing.
- A **topic** is just a concept that the learner stated. `Library.topics` keeps
  their order, most recent first.
- The **view** of a goal (`core::view`) is derived: a walk down from the goal
  that does not descend below a known concept. Each drawn node carries a
  `status` (`goal | unknown | learning | known`), `hidden_below` (how many
  concepts its being known hides), and `decomposed` (whether it has any
  prerequisites). The UI draws exactly this, and nothing in the view is stored.

## Display structure

`core::view` also gives each drawn concept the facts that drawing needs,
so the UI never re-derives graph facts (`core::structure`):

- `depth`: the fewest "needs" steps from the goal.
- `dependants`: how many drawn concepts need it.
- `section`: the section it belongs to.
  - Section roots are the goal's direct prerequisites in the **transitive
    reduction**.
  - A concept joins its nearest root. A tie goes first to the section most of
    its dependants are in, so a branch keeps one colour; then to the smaller
    section, so panels stay balanced; then by id.
  - Every concept but the goal has a section.
- `implied` (on the view): edges that a longer path already implies.
  Dropping them gives the transitive reduction, which is what the FLT
  landmark graph draws and what "Edges: reduced" shows.

## Rendering

**Choice: a small SVG renderer of our own, with dagre for the layered
layout.**

The target look is Anthropic's FLT blueprint figure and video, studied from
the captures in `docs/media/reference/`. Its features:

- an ivory page;
- tinted section panels, each with a coloured bar, a title and a subtitle;
- rounded boxes with a sans-serif title over a monospace identifier;
- dark orthogonal edges that merge into buses;
- a clay goal at the bottom;
- in the video, a radial dendrogram that grows outward from the goal, with
  one hue per section.

Why not the alternatives:

- **Cytoscape** (v0's renderer) cannot draw two type styles in one label,
  polar elbow edges, or panel headers.
- **Graphviz in WASM** (which the landmark graph uses) gets the static look
  but not smooth growth or interaction, and it costs a large WASM download.
- **sigma.js / WebGL** is built for thousands of nodes, and our graphs have
  tens.

The pipeline, `web/js/`:

1. `model.js`: view + settings → a drawing model. It filters known
   concepts, reduces edges, measures and wraps labels, and assigns hues.
2. `layout/`: each layout takes the model and returns positions, plus panels
   or labels.
   - `blueprint`: dagre top-down. Each section is a dagre cluster, drawn as
     a panel.
   - `radial`: a spanning tree that prefers a parent in the same section,
     with leaf-proportional wedges and a gap between sections.
   - `force`: relaxes from the radial positions; deterministic.
3. `render/`: SVG nodes, sections and edge paths, plus pan, zoom and pinch.
   - "flow" edges are orthogonal buses in the blueprint and polar elbows in
     the radial layout.
4. `render/scene.js` animates between drawings:
   - concepts glide to their new places;
   - new concepts grow out of the concept that needs them, nearest the goal
     first;
   - edges draw themselves in afterwards;
   - **Replay growth** redraws from nothing.

**State over hue.**
- The section hue colours the outline.
- The goal is clay, and a section's key concept is filled with its tint.
- Learning carries a gold mark.
- Known is dimmed (or shown, or hidden) and marked ✓.
- Not decomposed is a dashed outline.

**Settings** (`settings.js`) are one schema that the Display panel renders.
They are stored in `localStorage`, so they are per device, and they mirror
kb's graph lens: layout, label density, link style, colour by and size by.

## Decomposition prompt contract

One call per decomposition (`agent::prompts`). The input is the target's name
and statement, the goal it serves, the concepts that need it, the names of
known concepts (axioms), the names of other existing concepts, and a depth.

- **Depth 2** is used when a topic is stated: 3–6 direct prerequisites, each
  with 2–4 of its own.
- **Depth 1** is used when "decompose" is clicked: direct prerequisites only.

The output is JSON constrained by a JSON schema (`--json-schema` for
`claude -p`, or `output_schema` plus prompt instructions through Rig). Its
shape is exactly `core::Decomposition`:

```json
{
  "target":   {"name", "statement", "explanation", "check", "needs": ["<name>", …]},
  "concepts": [{"name", "statement", "explanation", "check", "needs": ["<name>", …]}]
}
```

`needs` entries are names. They resolve first against the proposals in the
same answer, then against an existing concept with that exact normalised name.
That second path is how a decomposition links to your axioms without
re-describing them: the prompt lists the known names and asks the model to
spell them exactly. Unresolvable names are ignored. A proposal that is the
target itself is dropped.

## Roadmap

`core::roadmap(view)` is derived from the view and never stored. It is a
depth-first **post-order** from the goal over the concepts that are not yet
known:

- Each concept is placed right after its own unknown prerequisites, which
  makes it a valid topological order.
- A branch is finished before the next begins, so it reads as a path, not as
  layers.
- Siblings go simplest first: by height (the longest unknown chain below
  them), then by name.

Each step carries `builds_on` (its known prerequisites: the axioms it starts
from) and `after` (its unknown prerequisites, all earlier steps).

## Dedupe (v0)

Two layers:

1. **Name normalisation** (`core::normalize`): lowercase, drop apostrophes and
   punctuation, drop a leading article, and join words with `-`. Equal normal
   forms mean a **collision**.
2. **"Same concept?" judge.** All collisions from one decomposition go to the
   model in **one** batched call. It gets each proposal's statement against the
   candidates' statements, and answers with a candidate id or `null`. Only
   offered ids are accepted.
   - `Same(id)`: link to the existing concept. Its text is kept, because a
     statement has one home.
   - `Distinct`: create a new concept with a suffixed id ("ring" in algebra
     vs. a ring buffer).

The prompt also lists the existing concept names and asks for exact reuse,
which avoids most collisions in the first place. Not done yet: synonyms with
different names ("Softmax" vs. "Softmax function") are not merged, and there is
no manual merge and no alias list. That is the "proper merging" left for later.

## Concurrency

The `Learner` never holds the library lock across a model call. It snapshots
the request, calls the model, re-locks to find collisions, calls the judge,
and re-locks to apply. The server bounds concurrent model calls to 2, and
every mutating API call saves and answers with the fresh blueprint.

## Persistence: kb-shaped nodes

`store` writes the library as kb's node shape
(<https://github.com/popemkt/kb/blob/main/DESIGN.md>, *Data model*), one node
per line:
`{id, text, props: {<field-node-id>: [{t, v}]}, children, createdAt, updatedAt}`.

- Concepts are nodes tagged `ax.tag.concept` (`sys.f.type`). Their id is the
  concept id, and `text` is the name.
- Props: `ax.f.statement`, `ax.f.explanation` and `ax.f.check` (str);
  `ax.f.needs` (ref, one per edge); `ax.f.knowledge` (ref to an option node).
- `ax.f.knowledge` declares its options as its own children
  (`ax.k.unknown | learning | known`), which is kb's option-set shape.
- The schema (fields, the tag, the options) is written as nodes too. The
  topics are the ordered children of `ax.topics`.
- `sys.*` ids are kb's seeded nodes: referenced, never written.
- A save keeps each node's `createdAt`, and moves `updatedAt` only when its
  content changed.

## kb integration

`AXIOMS_STORE=kb AXIOMS_KB_ROOT=<dir holding .kb/>` keeps the library in a
kb store instead of the JSONL file. `KbStore` (`crates/store/src/kb/`) is a
second implementation of `Store` behind the same codec, so both stores write
the same nodes. There is no shared code with kb and no rewrite of it.

**Interface.** `KbStore` talks to `kb mcp`, kb's MCP server on stdio. kb's
DESIGN names one action registry served by the CLI, MCP and HTTP alike, so
these are the same actions as `kb action-invoke`. A session is one process
per load or save: dozens of calls cost one start-up (about 0.5 s) instead of
one each, and nothing stays running. axioms never reads `.kb/` files, so a
sqlite-backed kb works the same. `AXIOMS_KB_BIN` names the binary (default
`kb`).

**Mapping.**

| axioms | kb |
| --- | --- |
| concept | a node with id = the concept slug, text = its name, tagged `#concept` (`ax.tag.concept`, `sys.f.type`) |
| statement, explanation, check | text fields `ax.f.statement`, `ax.f.explanation`, `ax.f.check` (`sys.ft.text`, cardinality one) |
| needs edge | ref prop on the field `ax.f.needs` (`sys.ft.ref`, `sys.f.targetTag` → `#concept`) |
| knowledge state | ref prop on `ax.f.knowledge` (cardinality one) to one of its option children `unknown`/`learning`/`known` |
| topics | the ordered children of an `ax.topics` node |

The `#concept` tag templates all five fields, so kb shows and offers them on
every concept. Props are keyed by field node, as kb keys them. The
roadmap and view are still derived and never stored.

**Writes.** A save diffs the codec's nodes against what kb holds and
writes only the difference. It runs in four passes, so every ref has its
target before it is written:

1. **Mint.** Missing fields are minted with `field.define` and the tag with
   `tag.define`, with explicit `ax.*` ids. Missing concepts are minted with
   `node.add`.
2. **Place.** Children are put in order with `node.update` (parent and
   position).
3. **Props.** Props are set and unset with `node.update`.
4. **Delete.** Concepts that axioms no longer has are removed with
   `node.delete`, reparenting their children.

kb validates every write: a value must fit its field's type, and a ref must
name a stored node.

**Shared ownership.** axioms owns only its `ax.*` fields on a concept. It
only ever adds to `sys.f.type` and `sys.f.fields`, and it ignores children it
did not place. A person can tag a concept, give it their own fields, or
nest notes under it, and saves keep all of that.

**Seeing it in kb.** Concepts show in the outline with their fields. To see
the blueprint in kb's graph view, tick Graph options → Relationships →
`needs`. `kb query` reads the DAG directly:

```sh
kb --root "$AXIOMS_KB_ROOT" query \
  '[:find ?from ?to :where [?a :f/ax.f.needs ?b] [?a :node/text ?from] [?b :node/text ?to]]'
```

**Known limits.**

- **Last writer wins on axioms' fields.** The server loads the library once
  at start. If a concept's `ax.*` fields are edited in kb while the server
  runs, the next save puts axioms' copy back. Edits made in kb are picked
  up on the next start.
- **`targetTag` is not enforced.** kb refuses a dangling `needs` ref, but
  not one that points at a non-concept node. kb uses `targetTag` to scope
  its pickers, not to check writes. On load, axioms skips a `needs` edge
  whose target is not a concept rather than failing.
- **Ids are slugs, not ULIDs.** That is legal in kb. A field the person
  already has with the same name (say "statement") is not reused: axioms
  mints its own with an `ax.*` id, so the two never mix.
- **Every save is a full diff.** Each save reads all concepts first. That
  is fine at hundreds of concepts; at many thousands it would want
  `graph.changes`.
- **kb records opened roots.** `kb init` and `kb ui` add the root to the
  person's location list in their home store. `kb mcp`, which axioms uses,
  does not. Tests and demos set `KB_HOME` to a throwaway home.
  `KbStore::with_home` does the same, so they never touch the person's own
  kb.

Tests: `crates/store/src/kb/plan.rs` tests the diff without kb. Run
`cargo test -p axioms-store -- --ignored` for the round trip through a real
kb in a temp store and temp home.

| kb outline: a concept with its fields | kb graph: the `needs` relation | axioms, reloaded from kb |
| --- | --- | --- |
| ![](media/kb/kb-concept-fields.jpg) | ![](media/kb/kb-graph-needs.jpg) | ![](media/kb/axioms-from-kb.jpg) |

## SpacetimeDB

With `AXIOMS_STORE=spacetime`, the library lives in a SpacetimeDB 2.14.0
database and every open view is live. A third backend sits beside JSONL and
kb; both of those keep working unchanged.

**Shape.** The module (`spacetime/`, Rust compiled to wasm32) stores and
validates. It holds:

| table | rows | key |
| --- | --- | --- |
| `concept` | the shared graph's concepts: name, statement, explanation, check, last writer | the core slug |
| `need` | `from_id` needs `to_id` | `from>to` |
| `topic` | stated topics, ordered by an auto-increment `seq` (newest highest) | concept |
| `knowledge` | one person's state of one concept; no row means unknown | `<identity>:<concept>` |
| `request` | a decomposition asked for: `topic` or `decompose`, then `pending` → `working` → `done`/`failed` | auto id |

It has these reducers:

- `put(concepts, needs, topics)`: an upsert; a need on a missing concept is refused.
- `mark(concept, state)`: the caller's own knowledge.
- `request_topic` and `request_decompose`: these dedupe an open request for the same thing.
- `claim`, `resolve` and `finish`: for workers.

The graph is shared. Knowledge is per identity, so each person has their
own axioms.

**The maths stays in core.**

- The module does no graph work. It does not even check for cycles; a client
  reading a cycle-closing need skips it.
- The *server* is a read model. It mirrors every table (`crates/live`,
  `Mirror`, self-healing across SpacetimeDB restarts). For
  `GET /api/topics/{id}?as=<identity>` it builds a `Library` with that
  person's knowledge and answers with core's view and roadmap, as before.
- The *worker* runs the learner (merge, dedupe) on the requester's library.
- The *CLI* uses `SpacetimeStore`.

**Why a worker.** Reducers are deterministic transactions and cannot call
a model. 2.x does have procedures that may make HTTP calls, but the
SpacetimeDB docs themselves recommend a sidecar for model calls. A sidecar
also keeps the provider (here `claude -p` with the host's own sign-in) out
of the database. The worker runs inside the live server unless
`AXIOMS_WORKER=0`; `axioms-worker` runs more of them elsewhere. Each worker:

1. watches `request`, claims a pending row (atomic: two workers never both
   win), and for a topic writes the topic concept at once (`resolve`), so
   the asker's view opens on it;
2. decomposes, then writes core's `diff(before, after)` split by
   `Diff::steps(target)`: one new concept per `put`, with only the needs
   whose ends exist, growing outward from the target;
3. calls `finish`.

Every subscribed view redraws per step, so a decomposition visibly grows
node by node (`AXIOMS_STREAM_MS`, 500 ms on the host, paces the steps).

**Browser.** `web/js/live.js` connects with the SpacetimeDB TypeScript SDK.
The SDK is bundled once with the generated bindings by esbuild
(`scripts/web-bundle.sh` → `web/vendor/spacetimedb.js`, committed like dagre;
the UI itself still has no build step).

- **Reads and writes.** It subscribes to all five tables, writes through
  reducers as its own identity, and on any change (debounced 120 ms) refetches
  the blueprint, so the existing renderer and Display options are untouched.
- **Waiting.** `decompose` and `learn` return when their request finishes,
  in the same shape as the JSON API; the steps arrive before that.
- **Pending.** Requests from other devices show as pending nodes.
- **Identity.** The token is kept in localStorage. The **live** badge copies
  a device link (`#/device/<token>`) that makes another browser the same
  person.
- **Reconnecting.** A dropped connection reloads the page after 5 s.

**`SpacetimeStore`.** The `Store` contract for the CLI.

- `load` is the shared graph with this client's knowledge.
- `save` writes core's diff: additions and changes only. It never deletes,
  because one person's older copy must not shrink a shared graph.
- `axioms import <nodes.jsonl>` brings a JSONL library in.
- It connects per call on a plain thread, so the caller's async runtime is
  never blocked in.

**Running it** (shared-server; `scripts/serve-live.sh`, which is also the
after-reboot restart):

- `axioms-spacetimedb`: a transient systemd unit, standalone on
  127.0.0.1:4420, with data in `~/.local/share/axioms/spacetime/data`, a
  512 MiB page pool, `MemoryMax=1G` and `CPUQuota=200%`. It uses about
  70 MB of RAM idle. It is never exposed itself.
- `axioms-live`: axioms-server on 127.0.0.1:4401, exposed on :10001 (it replaced the JSONL server there).
- **One origin.** Browsers reach SpacetimeDB through axioms-server
  (`crates/server/src/relay.rs`), which relays only what a client of the
  `axioms` database needs: `POST /v1/identity/websocket-token` and that
  database's `/subscribe` websocket. Publishing, deleting and SQL stay on
  loopback. `AXIOMS_STDB_PUBLIC_URI` points browsers elsewhere instead.
- The binaries are the official 2.14.0 release in user space
  (`scripts/spacetime-install.sh`). They are glibc-dynamic and run through
  nix-ld. nixpkgs has only 2.10.1, which is unfree and would build from
  source on this host.
- `scripts/spacetime` is the pinned CLI with its own config file.
- Module, SDK crate and npm package are pinned to the same 2.14.0.
- Systemd units, not `setsid`, because a bb daemon restart kills processes
  started from a thread.

**Verified** (screenshots below):

- **Sync.** A phone-sized page marking a concept known redrew the desktop
  page at once.
- **Pending.** A decomposition asked for on the phone showed as pending on
  the desktop.
- **Streaming.** A topic stated on the phone streamed into both views about
  every 500 ms.
- **Persistence.** Restarting SpacetimeDB kept every concept and mark. The
  server's mirror reconnected in 2 s, and the pages reloaded themselves.

**Trade-offs against JSONL and kb.**

| | JSONL | kb | SpacetimeDB |
| --- | --- | --- | --- |
| Live across devices | no (reload) | no (axioms reads kb at start) | yes, pushed to every view |
| People | one | one | many: shared graph, knowledge per identity |
| Writes | the whole file, atomically | a diff through kb's actions | a diff through reducers, per transaction |
| Running cost | none | the `kb` binary per save | a server process (~70 MB) and a port |
| Where the data can be seen and edited | any editor, git | kb's outline and graph, with your own notes | the axioms UI, `spacetime sql` |
| Deletes | yes | yes (reparenting) | no (the graph only grows) |
| Schema change | free | free | `spacetime publish`; a breaking change needs `-c` (wipes) or a migration |
| Licence | ours | kb's | BSL 1.1 (server; the SDK's npm metadata says ISC) |

JSONL stays the simplest and most portable; kb is where the concepts meet
the owner's own notes; SpacetimeDB is the one that makes axioms
multiplayer and live, at the price of a running server and a young,
fast-moving API (weekly minor releases, 2.x renamed core APIs).

**Known limits.**

- **No authentication beyond the tailnet.** Anyone who can reach the page
  can call reducers. Identities are anonymous tokens, and a device link carries
  one (it is a bearer secret).
- **Knowledge is a public table.** Each person sees only their own in the
  UI, but the rows are readable. A private table plus a per-sender view
  would hide them.
- **Imported knowledge belongs to the CLI identity.** Knowledge brought in
  with `import` is the CLI identity's (`axioms-cli`), not a browser's.
- **Re-stating a topic does not move it to the top.** Topics are inserted
  once.
- **Views refetch in full.** A view refetches its whole blueprint per
  (debounced) change. That is fine at hundreds of concepts; at many
  thousands, core would want to run in the browser (it is pure, so it
  could be compiled to wasm).

| Phone marks a concept known… | …and the desktop redraws, no reload |
| --- | --- |
| ![](media/spacetime/phone-marked.jpg) | ![](media/spacetime/desktop-synced.jpg) |

| A topic stated on the phone streams into the desktop… | …and the phone… | …step by step… | …to the finished blueprint |
| --- | --- | --- | --- |
| ![](media/spacetime/stream-1-desktop.jpg) | ![](media/spacetime/stream-2-phone.jpg) | ![](media/spacetime/stream-3-desktop.jpg) | ![](media/spacetime/stream-4-done.jpg) |

Also: [a decomposition requested on the phone, pending on the desktop](media/spacetime/desktop-pending-from-phone.jpg),
and [the desktop after a SpacetimeDB restart](media/spacetime/after-restart.jpg).

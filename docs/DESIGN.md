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

## Future link to kb (owner to decide)

Nothing couples axioms to kb today. The file is merely *shaped* like kb's
store. There are three options:

1. **Stay separate.** Keep the shape for familiarity only.
2. **Export/import.** A `kb` command reads or writes `ax.*` nodes into a kb
   store. Concept ids would need to become ULIDs, keeping the slug as a prop
   or alias.
3. **kb as the store.** A `KbStore` implements `Store` over kb's CLI or MCP.
   Axioms become kb nodes the owner can link from notes, and the blueprint
   becomes another kb projection (a graph view over `ax.f.needs`).

Option 3 fits kb's "everything is a node; views are projections" best. It is
one new `Store` implementation, because the codec is the only place that
knows the node shape.

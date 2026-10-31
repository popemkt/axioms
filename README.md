# axioms

Learn any topic starting from what you already know.

State a topic. axioms works backwards from it to what you already know, the
way the [Lean blueprint for Fermat's Last Theorem](https://www.anthropic.com/research/formalizing-fermats-last-theorem)
runs from the theorem down to Mathlib and the axioms. An agent decomposes the
topic into prerequisite concepts. Each has a one-line canonical statement, a
short explanation and a check question, and each edge means *needs*. Mark what
you know: those concepts become your **axioms**, and everything below them
collapses. If you don't get a concept, **decompose** it, and it expands another
level, as deep as you like. The **roadmap** is the path from your known
frontier up to the goal.

![FLT blueprint with a known concept collapsed](docs/media/flt-known-collapsed.jpg)

| Decomposing a leaf | Attention in transformers | Phone |
|---|---|---|
| ![](docs/media/flt-decomposed.jpg) | ![](docs/media/attention.jpg) | ![](docs/media/phone-card.jpg) |

## Use it

Open **https://shared-server.taild98079.ts.net:10001** (tailnet only).

- **State a topic** in the top bar ("Fermat's Last Theorem", "attention in
  transformers") and press **Learn**. The first decomposition goes two levels
  deep and takes about 20–60 s.
- **Click a node** to open its card: statement, explanation and check
  question. Then choose **I know this**, **Learning**, **Unknown**, or
  **Don't get it: decompose**.
- **Colours** follow a blueprint legend: purple is the goal, white is unknown,
  amber is learning, and green (dimmed) is known, one of your axioms. A dashed
  border means the concept has not been decomposed yet. "＋N below" counts the
  concepts a known node hides.
- **Roadmap** (right panel, or below the graph on a phone): the steps in
  order. Tick a step to mark it known. Click a step to find it in the graph.
- **Your topics** (top right) lists every topic you have stated. Axioms carry
  over, so the next topic starts from what you already marked known, and a
  concept that reappears is reused rather than duplicated.

CLI, sharing the same library:

```sh
nix develop
cargo run -p axioms-cli -- learn "Fermat's Last Theorem"   # tree + roadmap
cargo run -p axioms-cli -- mark known "Modular forms" "Elliptic curves"
cargo run -p axioms-cli -- decompose "Galois theory" "Fermat's Last Theorem"
cargo run -p axioms-cli -- show "Fermat's Last Theorem"
```

## Run

```sh
nix develop                 # toolchain (rustc, cargo, clippy, rustfmt); sets the git hook
scripts/serve.sh            # build, start detached on 127.0.0.1:4401, tailscale serve :10001
scripts/check.sh            # fmt --check, clippy -D warnings, tests (also the pre-commit hook)
```

`scripts/serve.sh` starts the server with `setsid nohup` and a **clean
environment**, so `claude -p` runs with the host's own signed-in Claude Code.
The log is at `~/.local/state/axioms/server.log`.

**Not persistent across reboots.** This host has unexplained hardware resets.
After a reboot, run `scripts/serve.sh` again. The tailscale serve config
survives, but the process does not.

Data: `~/.local/share/axioms/nodes.jsonl` (override with `AXIOMS_DATA`).

### Model provider

| `AXIOMS_PROVIDER` | What | Settings |
|---|---|---|
| `claude-cli` (default) | `claude -p --output-format json --json-schema …` as a subprocess, with tools, settings and MCP off | `AXIOMS_MODEL` (default `sonnet`), `AXIOMS_CLAUDE_BIN` |
| `openai` | any OpenAI-compatible endpoint through **Rig** (e.g. CLIProxyAPI) | `AXIOMS_OPENAI_BASE_URL`, `AXIOMS_MODEL`, key in the env var named by `AXIOMS_OPENAI_KEY_ENV` (default `OPENAI_API_KEY`) |

No key is ever stored in the repo or the library. The owner's CLIProxyAPI on
popemkt-work was not reachable over the tailnet when v0 was built (nothing
answered on :8317), so the default is the `claude -p` subprocess. To point
axioms at it later:
`AXIOMS_PROVIDER=openai AXIOMS_OPENAI_BASE_URL=http://popemkt-work:8317/v1 AXIOMS_MODEL=… scripts/serve.sh`.

## Architecture

A Cargo workspace of small crates. Each file has one purpose and stays under
about 200 lines.

| crate | role |
|---|---|
| `core` | concepts, the "needs" DAG (`petgraph`), the view projection, the roadmap, and the merge of a decomposition. Pure: no IO, no async. |
| `store` | the `Store` trait. `JsonlStore` and `MemoryStore` both go through one codec into kb-style nodes (everything is a node, edges are ref props). |
| `agent` | the `ModelProvider` trait (`ClaudeCli`, `RigOpenAi`), the `ConceptSource` trait (`LlmSource`, `ScriptedSource`), the prompt contracts, and the `Learner` loop. |
| `server` | `axum`: the JSON API under `/api`, and the static UI from `web/`. |
| `cli` | `clap`: `axioms learn/show/decompose/mark/topics`. |
| `web/` | vanilla ES modules with no build step. Cytoscape.js and cytoscape-dagre (which bundles dagre) are vendored in `web/vendor/` (3.34.4 / 4.0.1, MIT). |

The design is in [docs/DESIGN.md](docs/DESIGN.md): the graph model, the
decomposition prompt contract, the roadmap algorithm, dedupe, and the open
question of a link to kb.

### Deviations from the brief

- **Rig is used for the HTTP provider path, not for the `claude -p` path.**
  Rig 0.45 has no subprocess transport. Wrapping `claude -p` as a Rig model
  would mean implementing its wire and transport traits for one call, which is
  ceremony that buys nothing. Both providers implement one small trait
  (`ModelProvider`: a system prompt, a user prompt and a JSON schema in, JSON
  out), so the agent logic is identical either way.
- **"Explanation" is part of the decomposition answer**, not a separate call.
  One call returns names, statements, explanations and check questions, which
  keeps the first view at a single model round trip.

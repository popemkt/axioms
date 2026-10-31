# axioms — agent notes

axioms turns a stated topic into a blueprint: a "needs" DAG from the topic
down to what the learner already knows (their axioms), plus a roadmap. Read
[README.md](README.md) to run it and [docs/DESIGN.md](docs/DESIGN.md) for the
model.

## Rules

This repo adopts kb's engineering rules. Their home is kb's AGENTS.md, so
they are linked here rather than restated:

- [Rule 1: Abstraction before addition](https://github.com/popemkt/kb/blob/main/AGENTS.md#rule-1--abstraction-before-addition)
- [One contract, every implementation](https://github.com/popemkt/kb/blob/main/AGENTS.md#one-contract-every-implementation):
  here that means `ModelProvider`, `ConceptSource` and `Store`.
- [Canonical statements](https://github.com/popemkt/kb/blob/main/AGENTS.md#canonical-statements):
  a concept's statement lives once, on its node; the roadmap and view derive
  from the graph and are never stored.
- [Drift markers and gaps](https://github.com/popemkt/kb/blob/main/AGENTS.md#drift-markers-and-gaps)

## Local conventions

- `core` stays pure: no IO, no async, no model calls. Anything that can be a
  function of the graph belongs there, with tests.
- Small files with one purpose each, under about 200 lines.
- The gate is `scripts/check.sh` (fmt, clippy `-D warnings`, tests). It is the
  pre-commit hook (`core.hooksPath=.githooks`, set by `nix develop`).
- Commits: `<type>: <description>`, small, pushed often, on `main`.
- Host (shared-server): build with `CARGO_BUILD_JOBS=4`. Never restart app-bb
  or app-bb-daemon, and never touch the kb trial on :10000. axioms serves on
  :10001.
- Live model tests are `#[ignore]`d. Run them with
  `cargo test -p axioms-agent -- --ignored`.

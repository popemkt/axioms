#!/usr/bin/env sh
# The gate: format, lint, test. Runs inside the flake devShell when not already in one.
set -eu
if [ -z "${IN_NIX_SHELL:-}" ] && command -v nix >/dev/null 2>&1; then
  exec nix develop --quiet -c "$0" "$@"
fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --quiet
# The SpacetimeDB module is its own crate (wasm32), outside the workspace.
cargo fmt --manifest-path spacetime/Cargo.toml -- --check
cargo clippy --manifest-path spacetime/Cargo.toml --target wasm32-unknown-unknown -- -D warnings

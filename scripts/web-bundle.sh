#!/usr/bin/env sh
# Regenerate the TypeScript bindings from the module and bundle them with
# the pinned SpacetimeDB SDK into web/vendor/spacetimedb.js (committed, like
# dagre). Run after changing the module's tables or reducers.
set -eu
if [ -z "${IN_NIX_SHELL:-}" ] && command -v nix >/dev/null 2>&1; then
  exec nix develop --quiet "$(dirname "$0")/.." -c "$0" "$@"
fi
cd "$(dirname "$0")/.."
scripts/spacetime generate --lang typescript --out-dir spacetime/web/bindings \
  --module-path spacetime -y >/dev/null
(cd spacetime/web && npm install --no-audit --no-fund --silent)
esbuild spacetime/web/entry.ts --bundle --format=esm --minify --platform=browser \
  --target=es2022 --legal-comments=eof --outfile=web/vendor/spacetimedb.js

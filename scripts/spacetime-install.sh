#!/usr/bin/env sh
# Install the pinned SpacetimeDB 2.14.0 (CLI + standalone server) in user
# space, from the official release tarball. The binaries are glibc-dynamic:
# on NixOS they run through nix-ld. Also mints the server's JWT key pair.
set -eu
V=2.14.0
ST="${AXIOMS_STDB_HOME:-$HOME/.local/share/axioms/spacetime}"
mkdir -p "$ST/$V"
if [ ! -x "$ST/$V/spacetimedb-standalone" ]; then
  curl -fsSL "https://github.com/clockworklabs/SpacetimeDB/releases/download/v$V/spacetime-x86_64-unknown-linux-gnu.tar.gz" \
    | tar xz -C "$ST/$V"
fi
if [ ! -f "$ST/id_ecdsa" ]; then
  (umask 077 && cd "$ST" \
    && nix shell nixpkgs#openssl -c sh -c \
      'openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt -out id_ecdsa && openssl ec -in id_ecdsa -pubout -out id_ecdsa.pub')
fi
"$ST/$V/spacetimedb-cli" --config-path "$ST/cli.toml" server add axioms \
  --url http://127.0.0.1:4420 --default --no-fingerprint 2>/dev/null || true
"$ST/$V/spacetimedb-cli" --version

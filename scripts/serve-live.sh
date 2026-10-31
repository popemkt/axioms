#!/usr/bin/env sh
# Live mode on shared-server: SpacetimeDB and an axioms-server reading it,
# each a transient systemd unit (so a bb daemon restart does not kill them),
# exposed on the tailnet. Run again after a reboot; it replaces old units.
#
#   axioms-spacetimedb  127.0.0.1:4420 → https://<host>:10005 (websocket)
#   axioms-live         127.0.0.1:4401 → https://<host>:10001 (UI + API)
#
# It replaces the JSONL server (unit trial-axioms) on 4401. The permanent
# home is the dotfiles service (my.services.axioms); this is the stopgap.
#
# Data: ~/.local/share/axioms/spacetime/data (survives restarts).
# Logs: journalctl -u axioms-spacetimedb / -u axioms-live
set -eu
cd "$(dirname "$0")/.."
ST="${AXIOMS_STDB_HOME:-$HOME/.local/share/axioms/spacetime}"
HOST="${AXIOMS_TAILNET_HOST:-$(hostname).taild98079.ts.net}"
nix develop --quiet -c env CARGO_BUILD_JOBS=4 cargo build --release -p axioms-server
P="$HOME/.local/bin:/etc/profiles/per-user/$USER/bin:/run/wrappers/bin:/run/current-system/sw/bin"
run() { # unit, then the command; gentle limits on a host with hardware resets
  sudo -n systemctl stop "$1" 2>/dev/null || true
  sudo -n systemctl reset-failed "$1" 2>/dev/null || true
  u="$1"; shift
  sudo -n systemd-run --unit "$u" --uid "$USER" --gid users \
    --property=MemoryMax=1G --property=CPUQuota=200% --working-directory="$PWD" \
    --setenv=HOME="$HOME" --setenv=PATH="$P" --setenv=USER="$USER" --setenv=LANG=C.UTF-8 "$@"
}
sudo -n systemctl stop trial-axioms 2>/dev/null || true
run axioms-spacetimedb "$ST/2.14.0/spacetimedb-standalone" start --listen-addr 127.0.0.1:4420 \
  --data-dir "$ST/data" --jwt-priv-key-path "$ST/id_ecdsa" --jwt-pub-key-path "$ST/id_ecdsa.pub" \
  --page_pool_max_size 536870912 --non-interactive
sleep 3
run axioms-live --setenv=AXIOMS_ADDR=127.0.0.1:4401 --setenv=AXIOMS_STORE=spacetime \
  --setenv=AXIOMS_STREAM_MS="${AXIOMS_STREAM_MS:-500}" \
  --setenv=AXIOMS_STDB_PUBLIC_URI="wss://$HOST:10005" "$PWD/target/release/axioms-server"
sleep 2
curl -fsS http://127.0.0.1:4401/api/meta && echo
for map in "10001 http://127.0.0.1:4401" "10005 http://127.0.0.1:4420"; do
  set -- $map
  tailscale serve --bg --https="$1" "$2" 2>/dev/null || sudo -n tailscale serve --bg --https="$1" "$2"
done
echo "live: https://$HOST:10001"

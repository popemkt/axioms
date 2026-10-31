#!/usr/bin/env sh
# Build and start axioms-server detached on 127.0.0.1:$AXIOMS_PORT (default
# 4401), then expose it on the tailnet at https://<host>:10001.
#
# The server starts from a clean environment (HOME, PATH, USER, LANG and any
# AXIOMS_* variables only), so `claude -p` uses the host's own signed-in
# Claude Code, never a calling agent session's proxy or tokens.
# It does not survive a reboot: run this again after one.
set -eu
cd "$(dirname "$0")/.."
PORT="${AXIOMS_PORT:-4401}"
LOG="${AXIOMS_LOG:-$HOME/.local/state/axioms/server.log}"
BIN="$PWD/target/release/axioms-server"

if [ -z "${IN_NIX_SHELL:-}" ] && command -v nix >/dev/null 2>&1; then
  nix develop --quiet -c env CARGO_BUILD_JOBS=4 cargo build --release -p axioms-server
else
  CARGO_BUILD_JOBS=4 cargo build --release -p axioms-server
fi

mkdir -p "$(dirname "$LOG")"
pkill -f "^$BIN" 2>/dev/null && sleep 1 || true
AX_VARS=$(env | grep '^AXIOMS_' || true)
# shellcheck disable=SC2086
env -i HOME="$HOME" PATH="$PATH" USER="${USER:-}" LANG="${LANG:-C.UTF-8}" $AX_VARS \
  AXIOMS_ADDR="127.0.0.1:$PORT" setsid nohup "$BIN" >>"$LOG" 2>&1 </dev/null &
sleep 1
curl -fsS "http://127.0.0.1:$PORT/api/meta" && echo
# Operator rights alone are refused on this host; fall back to sudo.
tailscale serve --bg --https=10001 "http://127.0.0.1:$PORT" 2>/dev/null \
  || sudo -n tailscale serve --bg --https=10001 "http://127.0.0.1:$PORT"
echo "log: $LOG"

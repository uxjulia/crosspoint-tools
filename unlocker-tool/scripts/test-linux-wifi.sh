#!/usr/bin/env bash
# test-linux-wifi.sh - run the Linux Wi-Fi integration tests with the required
# privileges.
#
# Usage:
#   ./scripts/test-linux-wifi.sh [extra test args...]
#
# Examples:
#   ./scripts/test-linux-wifi.sh
#   ./scripts/test-linux-wifi.sh test_hotspot_lifecycle   # run one test only
#   ./scripts/test-linux-wifi.sh --nocapture              # show println output
#
# Requirements:
#   - A Wi-Fi adapter that NetworkManager can put into AP mode.
#   - sudo access.
#   - NetworkManager running.
#
# If a test leaves a stale hotspot behind (e.g. killed mid-test), clean up with:
#   nmcli connection delete xteink-unlocker-hotspot
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$WORKSPACE_ROOT"

# ── Phase 1: build (runs as the current user, no root needed) ──
# (and shouldn't be run under root in any case)

echo "==> Building test binary..."
TEST_BIN=$(
  cargo test -p unlocker-helper --no-run --message-format=json 2>/dev/null \
    | grep -o '"executable":"[^"]*"' \
    | cut -d'"' -f4 \
    | tail -1
)

if [[ -z "$TEST_BIN" ]]; then
  echo "ERROR: could not find test binary - did the build succeed?" >&2
  exit 1
fi

echo "==> Test binary: $TEST_BIN"

# ── Phase 2: run (re-invoke under sudo with the env var set) ─────────────────

# Pass any extra args (test name filter, --nocapture, etc.) through to the
# test binary.  The 'integration_tests' filter is always prepended so that
# unit tests don't run under root unnecessarily.
FILTER="integration_tests"
EXTRA_ARGS=("$@")

echo "==> Running integration tests (elevated)..."
sudo env UNLOCKER_WIFI_INTEGRATION=1 \
  "$TEST_BIN" \
  "$FILTER" \
  --ignored \
  --test-threads=1 \
  "${EXTRA_ARGS[@]}"

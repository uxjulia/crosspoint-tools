#!/usr/bin/env bash
# Linux release build for Xteink Unlocker (x86_64).
#
#  1. cargo build       — produces unlocker-helper for x86_64-unknown-linux-gnu
#  2. tauri build       — produces .deb, .rpm, and .AppImage. Helper is
#                         included via tauri.linux.conf.json bundle.resources
#                         so it lands at resource_dir()/unlocker-helper.
#  3. updater signing   — Tauri auto-signs the .AppImage with
#                         TAURI_SIGNING_PRIVATE_KEY, producing a sibling
#                         .AppImage.sig that the auto-updater verifies.
#
# Linux auto-update only works for users who run the .AppImage.
# Users on .deb / .rpm need to upgrade through their package manager.
#
# Usage:
#   ./scripts/build-linux.sh [major|minor|patch]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${REPO_ROOT}"

# ── Load .env.local if present ──
if [[ -f .env.local ]]; then
    # shellcheck disable=SC2046
    export $(grep -v '^#' .env.local | xargs)
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
    echo "warning: TAURI_SIGNING_PRIVATE_KEY not set — auto-update bundle won't be signed" >&2
fi

# ── Optional: bump version first ──
if [[ -n "${1:-}" ]]; then
    echo "==> Bumping version (${1})"
    if [[ -x "${REPO_ROOT}/scripts/bump-version.sh" ]]; then
        ./scripts/bump-version.sh "$1"
    else
        echo "warning: scripts/bump-version.sh not found; skipping bump" >&2
    fi
fi

# ── Check for required system packages ──
echo "==> Checking system dependencies"
MISSING=()
for pkg in pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf; do
    if ! dpkg -s "$pkg" >/dev/null 2>&1; then
        MISSING+=("$pkg")
    fi
done
if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "Missing required packages:" >&2
    printf '  %s\n' "${MISSING[@]}" >&2
    echo "Install with: sudo apt install ${MISSING[*]}" >&2
    exit 1
fi

# ── Ensure rust target ──
echo "==> Ensuring rust target x86_64-unknown-linux-gnu"
rustup target add x86_64-unknown-linux-gnu >/dev/null

# ── Build the helper ──
echo "==> Building unlocker-helper (release, x86_64-unknown-linux-gnu)"
cargo build --release --target x86_64-unknown-linux-gnu -p unlocker-helper

HELPER_BIN_SRC="${REPO_ROOT}/target/x86_64-unknown-linux-gnu/release/unlocker-helper"
[[ -x "${HELPER_BIN_SRC}" ]] || { echo "helper binary missing: ${HELPER_BIN_SRC}" >&2; exit 1; }
file "${HELPER_BIN_SRC}"

# ── Build the Tauri app ──
# Pulls in tauri.linux.conf.json which references the helper binary as a
# bundle resource so it ends up at resource_dir()/unlocker-helper inside
# the AppImage / deb / rpm.
echo "==> Building Tauri app (x86_64-unknown-linux-gnu)"
( cd app && npm run tauri -- build \
    --target x86_64-unknown-linux-gnu \
    --config src-tauri/tauri.linux.conf.json )

# ── Locate produced artifacts ──
APPIMAGE=$(find target/x86_64-unknown-linux-gnu/release/bundle/appimage -name "*.AppImage" -type f 2>/dev/null | head -1)
APPIMAGE_SIG=$(find target/x86_64-unknown-linux-gnu/release/bundle/appimage -name "*.AppImage.sig" -type f 2>/dev/null | head -1)
DEB=$(find target/x86_64-unknown-linux-gnu/release/bundle/deb -name "*.deb" -type f 2>/dev/null | head -1)
RPM=$(find target/x86_64-unknown-linux-gnu/release/bundle/rpm -name "*.rpm" -type f 2>/dev/null | head -1)

[[ -f "${APPIMAGE}" ]] || { echo "no AppImage produced" >&2; exit 1; }

echo
echo "Linux build complete."
echo "  AppImage:        ${APPIMAGE}"
[[ -n "${APPIMAGE_SIG}" ]] && echo "  Updater sig:     ${APPIMAGE_SIG}"
[[ -n "${DEB}" ]] && echo "  Debian package:  ${DEB}"
[[ -n "${RPM}" ]] && echo "  RPM package:     ${RPM}"
echo
echo "  Note: auto-update only fires for AppImage installs. .deb / .rpm users"
echo "        upgrade via apt/dnf or by re-running the installer."

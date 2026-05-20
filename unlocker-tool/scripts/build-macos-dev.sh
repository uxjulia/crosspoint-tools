#!/usr/bin/env bash
# Dev build — signs but skips notarization for faster iteration.
# The resulting .app works locally (right-click > Open to bypass Gatekeeper).
#
# Usage:
#   ./scripts/build-macos-dev.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${REPO_ROOT}"

# ── Load .env.local if present ──
if [[ -f .env.local ]]; then
    # shellcheck disable=SC2046
    export $(grep -v '^#' .env.local | xargs)
fi

# ── Required env ──
: "${APPLE_TEAM_ID:?APPLE_TEAM_ID not set (put it in .env.local)}"

APPLE_CERTIFICATE_IDENTITY="${APPLE_CERTIFICATE_IDENTITY:-Developer ID Application: SoFriendly LLC (${APPLE_TEAM_ID})}"
export APPLE_CERTIFICATE_IDENTITY
export APPLE_SIGNING_IDENTITY="${APPLE_CERTIFICATE_IDENTITY}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

echo "==> Identity: ${APPLE_CERTIFICATE_IDENTITY}"
echo "==> macOS deployment target: ${MACOSX_DEPLOYMENT_TARGET}"

# ── Ensure both Apple Silicon + Intel rust targets are installed ──
echo "==> Ensuring rust targets for universal build"
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null

# ── Build helper as a universal binary so we can inject it post-bundle ──
echo "==> Building unlocker-helper (release, universal)"
cargo build --release --target aarch64-apple-darwin -p unlocker-helper
cargo build --release --target x86_64-apple-darwin -p unlocker-helper

HELPER_BIN_SRC="${REPO_ROOT}/target/universal-apple-darwin/release/unlocker-helper"
mkdir -p "$(dirname "${HELPER_BIN_SRC}")"
lipo -create \
    "${REPO_ROOT}/target/aarch64-apple-darwin/release/unlocker-helper" \
    "${REPO_ROOT}/target/x86_64-apple-darwin/release/unlocker-helper" \
    -output "${HELPER_BIN_SRC}"
[[ -x "${HELPER_BIN_SRC}" ]] || { echo "helper binary missing: ${HELPER_BIN_SRC}" >&2; exit 1; }
file "${HELPER_BIN_SRC}"

# ── Build the Tauri app, universal (unset notarization env so Tauri skips it) ──
echo "==> Building Tauri app (universal, no notarization)"
( cd app && unset APPLE_ID APPLE_PASSWORD APPLE_API_KEY APPLE_API_KEY_PATH APPLE_API_ISSUER && npm run tauri build -- --target universal-apple-darwin )

# Locate the produced .app.
APP_PATH=$(find target/universal-apple-darwin/release/bundle/macos -name "*.app" -type d 2>/dev/null | head -1)
DMG_PATH=$(find target/universal-apple-darwin/release/bundle/dmg -name "*.dmg" 2>/dev/null | head -1)
[[ -d "${APP_PATH}" ]] || { echo "no .app produced by tauri build" >&2; exit 1; }
echo "==> App bundle: ${APP_PATH}"

# ── Inject helper binary ──
echo "==> Injecting helper into app bundle"
HELPER_BIN_DST="${APP_PATH}/Contents/MacOS/unlocker-helper"

cp -f "${HELPER_BIN_SRC}" "${HELPER_BIN_DST}"
chmod 0755 "${HELPER_BIN_DST}"

echo "==> Signing helper binary"
codesign --force \
    --options runtime \
    --timestamp \
    --entitlements app/src-tauri/helper-entitlements.plist \
    --sign "${APPLE_CERTIFICATE_IDENTITY}" \
    "${HELPER_BIN_DST}"

echo "==> Re-signing app bundle"
# No --deep: it re-applies the *app*'s entitlements to the helper, clobbering
# the helper-entitlements signing above. See build-macos.sh for the long form.
codesign --remove-signature "${APP_PATH}" || true
codesign --force \
    --options runtime \
    --timestamp \
    --entitlements app/src-tauri/entitlements.plist \
    --sign "${APPLE_CERTIFICATE_IDENTITY}" \
    "${APP_PATH}"
codesign --verify --strict --deep --verbose=2 "${APP_PATH}"

# Tauri creates the DMG before we inject the helper. Rebuild it from the
# now-correct .app so local installs include Contents/MacOS/unlocker-helper.
if [[ -n "${DMG_PATH}" && -f "${DMG_PATH}" ]]; then
    echo "==> Rebuilding DMG with helper-injected .app"
    DMG_STAGING="$(mktemp -d)"
    cp -R "${APP_PATH}" "${DMG_STAGING}/"
    ln -s /Applications "${DMG_STAGING}/Applications"
    rm -f "${DMG_PATH}"
    hdiutil create \
        -volname "$(basename "${APP_PATH}" .app)" \
        -srcfolder "${DMG_STAGING}" \
        -ov \
        -format UDZO \
        "${DMG_PATH}"
    rm -rf "${DMG_STAGING}"

    echo "==> Verifying DMG contains helper"
    DMG_MOUNT="$(mktemp -d)/mnt"
    mkdir -p "${DMG_MOUNT}"
    hdiutil attach "${DMG_PATH}" -nobrowse -readonly -mountpoint "${DMG_MOUNT}" >/dev/null
    if [[ ! -x "${DMG_MOUNT}/$(basename "${APP_PATH}")/Contents/MacOS/unlocker-helper" ]]; then
        hdiutil detach "${DMG_MOUNT}" -force >/dev/null || true
        echo "ERROR: rebuilt DMG is missing the helper" >&2
        exit 1
    fi
    hdiutil detach "${DMG_MOUNT}" -force >/dev/null

    echo "==> Signing rebuilt DMG"
    codesign --force --sign "${APPLE_CERTIFICATE_IDENTITY}" "${DMG_PATH}"
fi

echo
echo "Dev build complete (not notarized)."
echo "  App: ${APP_PATH}"
if [[ -n "${DMG_PATH}" ]]; then
    echo "  DMG: ${DMG_PATH}"
fi
echo "  Tip: right-click > Open to bypass Gatekeeper on first launch."

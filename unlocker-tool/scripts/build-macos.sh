#!/usr/bin/env bash
# Full macOS release build for Xteink Unlocker.
#
#  1. tauri build      — produces the .app and .dmg
#  2. inject helper    — copies unlocker-helper into Contents/MacOS/
#  3. re-sign          — codesigns the helper with helper-entitlements,
#                        then re-signs the bundle WITHOUT --deep so the
#                        helper's entitlements survive
#  4. notarize         — submits .app and .dmg to Apple, staples on success
#  5. update bundle    — produces a signed tar.gz for Tauri auto-update
#
# Usage:
#   ./scripts/build-macos.sh [major|minor|patch]
#
# If a bump argument is provided, version is bumped first via cargo + tauri.conf.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${REPO_ROOT}"

# ── Load .env.local if present ──
if [[ -f .env.local ]]; then
    # shellcheck disable=SC2046
    export $(grep -v '^#' .env.local | xargs)
fi

# ── Required env ──
: "${APPLE_ID:?APPLE_ID not set (put it in .env.local)}"
: "${APPLE_PASSWORD:?APPLE_PASSWORD not set (app-specific password)}"
: "${APPLE_TEAM_ID:?APPLE_TEAM_ID not set}"

APPLE_CERTIFICATE_IDENTITY="${APPLE_CERTIFICATE_IDENTITY:-Developer ID Application: SoFriendly LLC (${APPLE_TEAM_ID})}"
export APPLE_CERTIFICATE_IDENTITY
export APPLE_SIGNING_IDENTITY="${APPLE_CERTIFICATE_IDENTITY}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
    echo "warning: TAURI_SIGNING_PRIVATE_KEY not set — auto-update bundle won't be signed"
fi

echo "==> Identity: ${APPLE_CERTIFICATE_IDENTITY}"
echo "==> macOS deployment target: ${MACOSX_DEPLOYMENT_TARGET}"

# ── Optional: bump version first ──
if [[ -n "${1:-}" ]]; then
    echo "==> Bumping version (${1})"
    if [[ -x "${REPO_ROOT}/scripts/bump-version.sh" ]]; then
        ./scripts/bump-version.sh "$1"
    else
        echo "warning: scripts/bump-version.sh not found; skipping bump" >&2
    fi
fi

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

# ── Build the Tauri app (universal) ──
echo "==> Building Tauri app (universal)"
( cd app && npm run tauri build -- --target universal-apple-darwin )

# Locate the produced .app / .dmg.
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
# Important: NO --deep here. The helper at Contents/MacOS/unlocker-helper was
# just signed with helper-entitlements.plist. --deep would re-sign it with the
# *app*'s entitlements, which produces a hardened-runtime helper that fails to
# launch under `osascript ... with administrator privileges`. Apple deprecated
# --deep for this exact reason. Without --deep, codesign only updates the
# bundle's Code Resources catalog and leaves the helper's signature alone.
codesign --remove-signature "${APP_PATH}" || true
codesign --force \
    --options runtime \
    --timestamp \
    --entitlements app/src-tauri/entitlements.plist \
    --sign "${APPLE_CERTIFICATE_IDENTITY}" \
    "${APP_PATH}"
codesign --verify --strict --deep --verbose=2 "${APP_PATH}"

# Belt-and-braces: confirm the helper still carries helper-entitlements
# (no com.apple.security.network.client leaked in from the app entitlements).
echo "==> Verifying helper entitlements"
codesign -d --entitlements - "${HELPER_BIN_DST}" 2>&1 | grep -q "com.apple.security.network.client" \
    && { echo "ERROR: helper has app entitlements — re-sign clobbered it" >&2; exit 1; } \
    || echo "  helper entitlements look correct"

# ── Notarize the .app ──
echo "==> Notarizing app"
APP_ZIP="target/universal-apple-darwin/release/bundle/Unlocker.zip"
ditto -c -k --keepParent "${APP_PATH}" "${APP_ZIP}"
xcrun notarytool submit "${APP_ZIP}" \
    --apple-id "${APPLE_ID}" \
    --password "${APPLE_PASSWORD}" \
    --team-id "${APPLE_TEAM_ID}" \
    --wait
xcrun stapler staple "${APP_PATH}"
rm -f "${APP_ZIP}"

# ── Rebuild the DMG with the helper-injected .app ──
# Tauri's `tauri build` produces the .dmg in the same step as the .app, *before*
# we inject the helper. The DMG it writes therefore wraps a helper-less .app.
# Rebuild the DMG from the now-correct .app on disk before signing/notarizing.
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

    echo "==> Signing + notarizing DMG"
    codesign --force --sign "${APPLE_CERTIFICATE_IDENTITY}" "${DMG_PATH}"
    xcrun notarytool submit "${DMG_PATH}" \
        --apple-id "${APPLE_ID}" \
        --password "${APPLE_PASSWORD}" \
        --team-id "${APPLE_TEAM_ID}" \
        --wait
    xcrun stapler staple "${DMG_PATH}"
fi

# ── Update bundle for Tauri auto-update ──
VERSION=$(grep '"version"' app/src-tauri/tauri.conf.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
TAR_FILE="target/universal-apple-darwin/release/bundle/XteinkUnlocker_${VERSION}_darwin-universal.app.tar.gz"
echo "==> Creating update bundle ${TAR_FILE}"
COPYFILE_DISABLE=1 tar -czf "${TAR_FILE}" -C "$(dirname "${APP_PATH}")" "$(basename "${APP_PATH}")"

if [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
    echo "==> Signing update bundle"
    if [[ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]]; then
        ( cd app && npx tauri signer sign --private-key "${TAURI_SIGNING_PRIVATE_KEY}" --password "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD}" "../${TAR_FILE}" )
    else
        ( cd app && npx tauri signer sign --private-key "${TAURI_SIGNING_PRIVATE_KEY}" "../${TAR_FILE}" )
    fi
fi

echo
echo "Build complete."
echo "  App: ${APP_PATH}"
[[ -n "${DMG_PATH}" ]] && echo "  DMG: ${DMG_PATH}"
echo "  Update bundle: ${TAR_FILE}"

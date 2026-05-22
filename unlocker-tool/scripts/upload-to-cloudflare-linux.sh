#!/usr/bin/env bash
# Push the latest Linux build artifacts to R2 and write the linux-<arch>
# update manifest so the Tauri updater (running inside the AppImage) picks it
# up. Run after build-linux.sh.
#
# Required env (loaded from .env.local if present):
#   CLOUDFLARE_ACCOUNT_ID
#   CLOUDFLARE_R2_ACCESS_KEY  (or AWS_ACCESS_KEY_ID)
#   CLOUDFLARE_R2_SECRET_KEY  (or AWS_SECRET_ACCESS_KEY)
#
# Optional:
#   CLOUDFLARE_R2_BUCKET   defaults to "unlocker-releases"

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${REPO_ROOT}"

if [[ -f .env.local ]]; then
    # shellcheck disable=SC2046
    export $(grep -v '^#' .env.local | xargs)
fi

[[ -z "${CLOUDFLARE_R2_ACCESS_KEY:-}" ]] && CLOUDFLARE_R2_ACCESS_KEY="${AWS_ACCESS_KEY_ID:-}"
[[ -z "${CLOUDFLARE_R2_SECRET_KEY:-}" ]] && CLOUDFLARE_R2_SECRET_KEY="${AWS_SECRET_ACCESS_KEY:-}"

: "${CLOUDFLARE_ACCOUNT_ID:?CLOUDFLARE_ACCOUNT_ID not set}"
: "${CLOUDFLARE_R2_ACCESS_KEY:?CLOUDFLARE_R2_ACCESS_KEY not set}"
: "${CLOUDFLARE_R2_SECRET_KEY:?CLOUDFLARE_R2_SECRET_KEY not set}"

CLOUDFLARE_R2_BUCKET="${CLOUDFLARE_R2_BUCKET:-unlocker-releases}"
R2_ENDPOINT="https://${CLOUDFLARE_ACCOUNT_ID}.r2.cloudflarestorage.com"

VERSION=$(grep '"version"' app/src-tauri/tauri.conf.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
echo "Uploading version: $VERSION"

HOST_ARCH="$(uname -m)"
DEFAULT_TARGET="x86_64-unknown-linux-gnu"
case "$HOST_ARCH" in
  x86_64|amd64) DEFAULT_TARGET="x86_64-unknown-linux-gnu" ;;
  aarch64|arm64) DEFAULT_TARGET="aarch64-unknown-linux-gnu" ;;
esac
LINUX_TARGET="${LINUX_TARGET:-$DEFAULT_TARGET}"
case "$LINUX_TARGET" in
  x86_64-unknown-linux-gnu) RELEASE_ARCH="x86_64" ;;
  aarch64-unknown-linux-gnu) RELEASE_ARCH="aarch64" ;;
  *) echo "Unsupported LINUX_TARGET: $LINUX_TARGET" >&2; exit 1 ;;
esac
PLATFORM_KEY="linux-${RELEASE_ARCH}"

# ── Extract release notes from CHANGELOG.md (same logic as macOS upload) ──
extract_changelog() {
  local version=$1
  local changelog_file="CHANGELOG.md"
  [[ -f "$changelog_file" ]] || { echo "Update to version ${version}"; return; }

  local notes
  notes=$(awk -v ver="$version" '
    /^## \[/ {
      if (found) exit
      if ($0 ~ "\\[" ver "\\]") found=1
      next
    }
    found && !/^## / { print }
  ' "$changelog_file" | sed '/^$/d' | sed 's/^- /• /')

  if [[ -z "$notes" ]]; then
    notes=$(awk '
      /^## \[/ {
        if (found) exit
        found=1
        next
      }
      found && !/^## / { print }
    ' "$changelog_file" | sed '/^$/d' | sed 's/^- /• /')
  fi
  echo "$notes"
}

CHANGELOG_NOTES=$(extract_changelog "$VERSION")
[[ -z "$CHANGELOG_NOTES" ]] && CHANGELOG_NOTES="Update to version ${VERSION}"
echo "Changelog notes:"
echo "$CHANGELOG_NOTES"

upload_file() {
  local file=$1
  local key=$2
  if [[ -f "$file" ]]; then
    echo "Uploading: $key"
    AWS_ACCESS_KEY_ID="$CLOUDFLARE_R2_ACCESS_KEY" \
    AWS_SECRET_ACCESS_KEY="$CLOUDFLARE_R2_SECRET_KEY" \
    aws s3 cp "$file" "s3://${CLOUDFLARE_R2_BUCKET}/${key}" \
      --endpoint-url "$R2_ENDPOINT" \
      --no-progress
  else
    echo "Skipping (not found): $file"
  fi
}

echo
echo "=== Uploading Linux artifacts (${PLATFORM_KEY}) ==="

BUNDLE_DIR="target/${LINUX_TARGET}/release/bundle"

# Tauri 2 signs the AppImage directly, producing `*.AppImage` and a sibling
# `*.AppImage.sig`. (v1 produced a `.AppImage.tar.gz` updater bundle — that
# format is gone in v2.) The updater manifest URL points at the AppImage itself.
APPIMAGE=$(find "${BUNDLE_DIR}/appimage" -name "*.AppImage" -type f 2>/dev/null | head -1)
APPIMAGE_SIG=$(find "${BUNDLE_DIR}/appimage" -name "*.AppImage.sig" -type f 2>/dev/null | head -1)
DEB=$(find "${BUNDLE_DIR}/deb" -name "*.deb" -type f 2>/dev/null | head -1)
RPM=$(find "${BUNDLE_DIR}/rpm" -name "*.rpm" -type f 2>/dev/null | head -1)

# Upload installers under stable, version-pinned names so the URL pattern
# matches the macOS / Windows convention.
APPIMAGE_KEY="v${VERSION}/XteinkUnlocker_${VERSION}_${PLATFORM_KEY}.AppImage"
[[ -n "${APPIMAGE}" ]] && upload_file "$APPIMAGE" "$APPIMAGE_KEY"
[[ -n "${APPIMAGE_SIG}" ]] && upload_file "$APPIMAGE_SIG" "${APPIMAGE_KEY}.sig"
[[ -n "${DEB}" ]] && upload_file "$DEB" "v${VERSION}/XteinkUnlocker_${VERSION}_${PLATFORM_KEY}.deb"
[[ -n "${RPM}" ]] && upload_file "$RPM" "v${VERSION}/XteinkUnlocker_${VERSION}_${PLATFORM_KEY}.rpm"

echo
echo "=== Writing ${PLATFORM_KEY} update manifest ==="

LINUX_SIG=""
[[ -n "${APPIMAGE_SIG:-}" && -f "${APPIMAGE_SIG}" ]] && LINUX_SIG=$(cat "${APPIMAGE_SIG}")

if [[ -z "$LINUX_SIG" ]]; then
  echo "No Linux signature (TAURI_SIGNING_PRIVATE_KEY unset?). Manifest not written." >&2
else
  PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  APPIMAGE_URL="https://unlocker-releases.crosspointreader.com/${APPIMAGE_KEY}"

  OUT="${BUNDLE_DIR}/latest-${PLATFORM_KEY}.json"
  cat > "$OUT" <<JSON
{
  "version": "${VERSION}",
  "notes": $(jq -Rs . <<<"$CHANGELOG_NOTES"),
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "${PLATFORM_KEY}": {
      "signature": $(jq -Rs . <<<"$LINUX_SIG"),
      "url": "${APPIMAGE_URL}"
    }
  }
}
JSON
  upload_file "$OUT" "latest-${PLATFORM_KEY}.json"
fi

echo
echo "=== Upload complete ==="
echo "Linux update endpoint:"
echo "  https://unlocker-releases.crosspointreader.com/latest-${PLATFORM_KEY}.json"
echo "(Auto-update only fires for AppImage installs; .deb / .rpm users upgrade via apt/dnf.)"

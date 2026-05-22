# Building & Releasing

End-to-end build, signing, notarization, and auto-update for the Xteink Unlocker. The `scripts/` directory wraps each step; this doc explains what each does and the env it expects.

## Bundle identifiers

- App: `com.sofriendly.crosspoint.unlocker`
- Helper: `com.sofriendly.crosspoint.unlocker.helper`

## macOS

### Setup

Copy `.env.local.example` to `.env.local` and fill in:

```bash
APPLE_ID=you@sofriendly.com
APPLE_PASSWORD=@keychain:AC_PASSWORD       # app-specific password, or @keychain:NAME
APPLE_TEAM_ID=2H66PPM438

# Optional, for auto-update bundle signing
# TAURI_SIGNING_PRIVATE_KEY=
# TAURI_SIGNING_PRIVATE_KEY_PASSWORD=
```

`APPLE_SIGNING_IDENTITY` defaults to `Developer ID Application: SoFriendly LLC (2H66PPM438)`. Override via env if needed.

Apple Team: **SoFriendly LLC (`2H66PPM438`)** — already wired into `tauri.conf.json` (`providerShortName`) and the build script's default identity.

### Signed build pipeline

`scripts/build-macos.sh` runs:

1. `tauri build` — produces the `.app` and `.dmg`.
2. Inject the helper binary into `Contents/MacOS/unlocker-helper`.
3. Sign the helper, re-sign the bundle for a consistent signature.
4. Notarize `.app` and `.dmg` with `xcrun notarytool`, staple.
5. Produce a signed `.tar.gz` for Tauri auto-update.

### One-shot build

```bash
npm run bundle
# or, with version bump:
./scripts/build-macos.sh patch
```

Output:
- Signed + notarized app at `target/release/bundle/macos/Xteink Unlocker.app`
- Signed + notarized DMG at `target/release/bundle/dmg/`
- Auto-update tarball at `target/release/bundle/XteinkUnlocker_<version>_darwin-aarch64.app.tar.gz` (signed if `TAURI_SIGNING_PRIVATE_KEY` is set)

### Dev build (faster local iteration)

```bash
./scripts/build-macos-dev.sh
```

Same pipeline as `build-macos.sh` but **skips Apple notarization** — the slow step that uploads to Apple and waits for staple approval. Useful when you're iterating on something that can only be exercised through the bundled app (signed helper launch, admin prompt, hotspot, real OTA flow against a device).

What it still does:

- Builds `unlocker-helper` for both `aarch64-apple-darwin` and `x86_64-apple-darwin`, lipos them into a universal binary.
- Runs `tauri build --target universal-apple-darwin`.
- Injects the helper into `Contents/MacOS/unlocker-helper`.
- Signs the helper with `helper-entitlements.plist`, then re-signs the bundle with `entitlements.plist`.
- Rebuilds the DMG so it contains the helper-injected `.app` (Tauri creates the DMG before injection, so the original is stale).
- Signs the rebuilt DMG.

What it does **not** do:

- No notarization.
- No version bump.
- No upload to Cloudflare.

Requirements:

- `.env.local` with `APPLE_TEAM_ID` set (everything else is optional for dev — `APPLE_ID` / `APPLE_PASSWORD` are unset inside the script's `tauri build` call so Tauri skips its own notarization step).
- A valid `Developer ID Application` cert in your keychain (override identity via `APPLE_CERTIFICATE_IDENTITY` env if you don't have SoFriendly's).
- Both rust targets installed — the script auto-runs `rustup target add aarch64-apple-darwin x86_64-apple-darwin` if missing.

Output:
- Signed (not notarized) `.app` at `target/universal-apple-darwin/release/bundle/macos/Xteink Unlocker.app`
- Signed `.dmg` at `target/universal-apple-darwin/release/bundle/dmg/`

Because the build isn't notarized, Gatekeeper blocks the first launch. **Right-click → Open** to bypass it once; subsequent launches work normally.

### Cutting a release (full pipeline)

```bash
./scripts/release.sh patch
```

Bumps the version, builds + signs + notarizes, commits the version files, tags `vX.Y.Z`, pushes, and uploads to Cloudflare R2. After this completes, existing installs see the update on next launch (within 3s of opening the app) or when the user clicks **Check for updates** in the footer.

## Windows

```powershell
# bumps version (optional), builds helper + app, signs both installers
.\scripts\build-windows.ps1 patch
.\scripts\upload-to-cloudflare.ps1
```

The PowerShell scripts mirror the macOS pipeline: bump version, `cargo build --release -p unlocker-helper`, `npm run tauri -- build` (NSIS + MSI, picks up `app/src-tauri/tauri.windows.conf.json`), `signtool` for both installers using the Sectigo USB token, then push to R2 and merge a `windows-x86_64` entry into `latest.json` while preserving `darwin-aarch64`.

## Linux (x86_64 + aarch64)

### Setup

System packages (Debian/Ubuntu — adjust for your distro):

```bash
sudo apt install \
  pkg-config \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  rpm
```

`.env.local` only needs `TAURI_SIGNING_PRIVATE_KEY` (and the Cloudflare R2 vars for upload). No OS-side signing or notarization is required.

### Build

In practice you won't run this on a Mac — Tauri's Linux build links against `libwebkit2gtk` / `libgtk-3` which aren't available on macOS. Use the CI pipeline (below) for releases, or run on a Linux box / VM / container if you need to iterate locally:

```bash
./scripts/build-linux.sh           # or pass major|minor|patch to bump first
./scripts/upload-to-cloudflare-linux.sh
```

### CI (GitHub Actions)

`.github/workflows/build-unlocker-linux.yml` runs the same scripts on `ubuntu-22.04` and `ubuntu-22.04-arm` whenever a `vX.Y.Z` tag is pushed (matches `scripts/release.sh`'s tag step), or on manual dispatch via the Actions tab. It:

1. Installs system deps + Node + Rust.
2. Runs `scripts/build-linux.sh` (helper → tauri build → AppImage / deb / rpm + signed updater bundle).
3. Uploads the bundle as a GH Actions artifact for inspection.
4. Runs `scripts/upload-to-cloudflare-linux.sh` to push to R2 and write `latest-linux-x86_64.json` / `latest-linux-aarch64.json`.
5. Creates a GitHub Release with the installers attached.

Required repository secrets (Settings → Secrets and variables → Actions):

| Secret | Notes |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | minisign private key, single line. Same key used for macOS / Windows. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | optional, only if the key is password-protected |
| `CLOUDFLARE_ACCOUNT_ID` | |
| `CLOUDFLARE_R2_ACCESS_KEY` | R2 API token scoped to the `unlocker-releases` bucket |
| `CLOUDFLARE_R2_SECRET_KEY` | |
| `CLOUDFLARE_R2_BUCKET` | optional, defaults to `unlocker-releases` |
| `UNLOCKER_TLS_PRIVKEY_PEM` | full contents of `crates/unlocker-helper/certs/privkey.pem` (the Let's Encrypt private key for `unlocker.crosspointreader.com`). Compiled into the helper via `include_str!` and gitignored, so CI materializes it from this secret before building. |

`build-linux.sh`:

1. Builds `unlocker-helper` for the native Linux runner architecture.
2. Runs `tauri build --target $LINUX_TARGET --config src-tauri/tauri.linux.conf.json` — produces `.AppImage`, `.deb`, `.rpm`. CI sets `LINUX_TARGET` to `x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-gnu`; local builds infer it from `uname -m`.
3. The Linux config bundles the helper as a Tauri resource so it lands at `resource_dir()/unlocker-helper` inside the AppImage / deb / rpm — same path the Tauri shell already reads.
4. Tauri auto-signs the `.AppImage` (producing a sibling `.AppImage.sig`) with `TAURI_SIGNING_PRIVATE_KEY` if set. The Tauri 2 auto-updater downloads the AppImage directly and verifies the sig — there is no `.AppImage.tar.gz` updater wrapper anymore (that was a v1 thing).

Output:
- AppImage: `target/<linux-target>/release/bundle/appimage/*.AppImage` (+ `.sig`)
- Debian package: `target/<linux-target>/release/bundle/deb/*.deb`
- RPM package: `target/<linux-target>/release/bundle/rpm/*.rpm`

### Privileged helper at runtime

The Debian package depends on `pkexec` and `polkitd`, not the obsolete transitional `policykit-1` package. The Tauri shell launches a short root shell via `pkexec`, which gives the user the standard PolicyKit graphical password prompt — the Linux equivalent of the macOS osascript / Windows UAC prompt. That shell starts `unlocker-helper` in the background and exits so the app can continue to the RPC readiness check. No polkit `.policy` file required (pkexec falls back to `auth_admin_keep`).

### Auto-update caveat

Tauri's auto-updater only fires for users running the **AppImage** (it detects via `$APPIMAGE`). Users who installed via `.deb` or `.rpm` need to upgrade through their package manager — re-download the `.deb`/`.rpm` from the releases bucket or rerun the installer. Worth noting in any UI that prompts for updates so package-manager users don't get a broken auto-update flow.

## Auto-update infrastructure

- **Endpoint:** `https://unlocker-releases.crosspointreader.com/latest.json`
- **Bucket:** `unlocker-releases` on Cloudflare R2
- **Worker:** `workers/releases/` — deploy with `cd workers/releases && npx wrangler deploy`
- **Public key** (paste into the worker route or whatever consumes `latest.json` if you ever need to verify outside Tauri):
  ```
  dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEY1RDVFOTA2OTQzN0NCMTkKUldRWnl6ZVVCdW5WOVpyaHR0anNZRm5jNlBEaFk3WWJvbkpSdDdxbC9XSmQ5N0pZcitGK1d5YUMK
  ```

### First-time setup

1. Create the `unlocker-releases` R2 bucket in the SoFriendly Cloudflare account.
2. Create an R2 API token scoped to that bucket; paste keys into `.env.local`.
3. Add the DNS record for `unlocker-releases.crosspointreader.com` → Cloudflare Workers route.
4. `cd workers/releases && npm install && npx wrangler deploy`.
5. `./scripts/release.sh patch` to cut the first release.

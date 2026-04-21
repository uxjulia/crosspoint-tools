# CrossPoint Tools

Web-based firmware flasher and build system for [CrossPoint Reader](https://github.com/crosspoint-reader/crosspoint-reader) devices. Hosted at [crosspointreader.com](https://crosspointreader.com).

## What it does

- **Stable firmware flashing** — Flash the latest CrossPoint release or stock Xteink firmware to X3 and X4 devices directly from the browser using WebSerial
- **Early access builds** — Nightly firmware builds compiled automatically from the master branch, gated behind a [Royalty.dev](https://royalty.dev) subscription
- **Stock firmware** — Restore original Xteink firmware (English or Chinese) for both X3 and X4
- **Full flash backup/restore** — Save and restore the entire 16MB flash contents
- **Admin dashboard** — Manually trigger builds and view build logs

## How it works

The project runs on [Cloudflare Workers](https://workers.cloudflare.com/) with the following infrastructure:

- **Worker** (`src/index.ts`) — Handles API routes, firmware proxying, early access gating via Royalty.dev, and serves static assets
- **Cloudflare Durable Objects + Containers** — Runs PlatformIO builds inside sandboxed Docker containers to compile firmware from source
- **R2** — Stores compiled firmware binaries
- **KV** — Caches build metadata for fast reads
- **Static assets** (`public/`) — HTML pages, the WebSerial flasher module, and bundled X3 firmware

### Build pipeline

1. A GitHub webhook or daily cron job detects new commits on the upstream CrossPoint repo
2. A Durable Object spins up a Docker container with PlatformIO pre-installed
3. The container clones the repo, compiles firmware, and uploads the binary to R2
4. Build metadata (version, commit, changelog) is stored in KV

### Flashing

The browser-based flasher (`public/js/flasher.js`) uses [esptool-js](https://github.com/nicholasgasior/nicholasgasior.github.io) via WebSerial to perform OTA flashing:

1. Connects to the ESP32-C3 over USB serial
2. Reads and validates the partition table
3. Writes firmware to the backup OTA partition
4. Updates the OTA boot selector to swap partitions on next boot

## Development

```bash
npm install
npm run dev      # Start local dev server (wrangler dev)
npm run deploy   # Deploy to Cloudflare
```

Secrets are managed via `wrangler secret put`:
- `GITHUB_WEBHOOK_SECRET` — Webhook signature verification and admin auth
- `GITHUB_TOKEN` (optional) — For private repo access

## Acknowledgments

The WebSerial flasher is based on [xteink-flasher](https://github.com/crosspoint-reader/xteink-flasher), licensed under the MIT License. The partition table handling, OTA flashing logic, and device model support were adapted from that project.

## License

MIT

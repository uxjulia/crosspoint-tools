/**
 * Releases worker for Xteink Unlocker.
 * Routed at unlocker-releases.crosspointreader.com.
 *
 * Serves objects from the `unlocker-releases` R2 bucket. Two notable paths:
 *
 *   GET /latest.json
 *     The Tauri updater endpoint. Pushed by scripts/upload-to-cloudflare.sh.
 *
 *   GET /unlocker-latest.dmg
 *   GET /unlocker-latest.msi
 *   GET /unlocker-latest.tar.gz
 *   GET /unlocker-latest.AppImage
 *   GET /unlocker-latest.deb
 *   GET /unlocker-latest.rpm
 *   GET /unlocker-latest-arm64.AppImage
 *   GET /unlocker-latest-arm64.deb
 *   GET /unlocker-latest-arm64.rpm
 *     Convenience redirects to the most-recent versioned artifact.
 *
 * Anything else is a passthrough to the bucket.
 */

export interface Env {
  BUCKET: R2Bucket;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const key = url.pathname.slice(1);

    if (request.method === "OPTIONS") {
      return new Response(null, {
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
          "Access-Control-Allow-Headers": "*",
        },
      });
    }

    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method Not Allowed", { status: 405 });
    }

    if (!key) {
      return new Response("Xteink Unlocker Releases", {
        headers: { "Content-Type": "text/plain" },
      });
    }

    // /unlocker-latest[-arch].(dmg|tar.gz|msi|AppImage|deb|rpm)
    const latestMatch = key.match(
      /^unlocker-latest(?:-(arm64|amd64))?\.(dmg|tar\.gz|msi|AppImage|deb|rpm)$/,
    );
    if (latestMatch) {
      try {
        const arch = latestMatch[1];
        const ext = latestMatch[2];
        const linuxPlatform =
          arch === "arm64" ? "linux-aarch64" : "linux-x86_64";
        const manifestForExt: Record<string, string> = {
          dmg: "latest.json",
          "tar.gz": "latest.json",
          msi: "latest-windows-x86_64.json",
          AppImage: `latest-${linuxPlatform}.json`,
          deb: `latest-${linuxPlatform}.json`,
          rpm: `latest-${linuxPlatform}.json`,
        };
        const manifestKey = manifestForExt[ext] ?? "latest.json";
        const manifestObj = await env.BUCKET.get(manifestKey);
        if (manifestObj) {
          const latest = await manifestObj.json<{
            version: string;
            platforms: Record<string, { url: string; signature: string }>;
          }>();

          const extractVersion = (platformKey: string): string | null => {
            const platformUrl = latest.platforms[platformKey]?.url;
            if (!platformUrl) return null;
            const m = platformUrl.match(/XteinkUnlocker_([\d.]+)/);
            return m ? m[1] : null;
          };

          const platformForExt: Record<string, string> = {
            dmg: "darwin-aarch64",
            "tar.gz": "darwin-aarch64",
            msi: "windows-x86_64",
            AppImage: linuxPlatform,
            deb: linuxPlatform,
            rpm: linuxPlatform,
          };
          const platformKey = platformForExt[ext];
          const version =
            (platformKey && extractVersion(platformKey)) || latest.version;

          const fileMap: Record<string, string> = {
            dmg: `v${version}/XteinkUnlocker_${version}_universal.dmg`,
            "tar.gz": `v${version}/XteinkUnlocker_${version}_darwin-universal.app.tar.gz`,
            msi: `v${version}/XteinkUnlocker_${version}_x64.msi`,
            AppImage: `v${version}/XteinkUnlocker_${version}_${linuxPlatform}.AppImage`,
            deb: `v${version}/XteinkUnlocker_${version}_${linuxPlatform}.deb`,
            rpm: `v${version}/XteinkUnlocker_${version}_${linuxPlatform}.rpm`,
          };
          const targetKey = fileMap[ext];
          if (targetKey) {
            return Response.redirect(`${url.origin}/${targetKey}`, 302);
          }
        }
      } catch (e) {
        return new Response(`Could not determine latest version: ${e}`, {
          status: 500,
        });
      }
    }

    try {
      const object = await env.BUCKET.get(key);
      if (!object) return new Response("Not Found", { status: 404 });

      const headers = new Headers();
      headers.set("Access-Control-Allow-Origin", "*");
      headers.set("etag", object.httpEtag);

      if (key.endsWith(".json")) {
        headers.set("Content-Type", "application/json");
      } else if (key.endsWith(".dmg")) {
        headers.set("Content-Type", "application/x-apple-diskimage");
      } else if (key.endsWith(".tar.gz")) {
        headers.set("Content-Type", "application/gzip");
      } else if (key.endsWith(".msi")) {
        headers.set("Content-Type", "application/x-msi");
      } else if (key.endsWith(".AppImage")) {
        headers.set("Content-Type", "application/vnd.appimage");
      } else if (key.endsWith(".deb")) {
        headers.set("Content-Type", "application/vnd.debian.binary-package");
      } else if (key.endsWith(".rpm")) {
        headers.set("Content-Type", "application/x-rpm");
      } else if (key.endsWith(".sig")) {
        headers.set("Content-Type", "text/plain");
      } else {
        headers.set("Content-Type", "application/octet-stream");
      }

      if (
        !key.endsWith(".json") &&
        !key.endsWith(".sig") &&
        !key.endsWith(".pem")
      ) {
        const filename = key.split("/").pop();
        headers.set("Content-Disposition", `attachment; filename="${filename}"`);
      }

      if (object.size) headers.set("Content-Length", object.size.toString());

      return new Response(object.body, { headers });
    } catch {
      return new Response("Internal Error", { status: 500 });
    }
  },
};

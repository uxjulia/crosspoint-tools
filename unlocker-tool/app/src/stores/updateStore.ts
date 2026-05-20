import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { isMac } from "../platform";

interface UpdateInfo {
  version: string;
  body?: string;
}

interface UpdateState {
  updateAvailable: UpdateInfo | null;
  isChecking: boolean;
  isDownloading: boolean;
  isInstalling: boolean;
  downloadProgress: number;
  updateRef: Update | null;
  upToDateAt: number | null;
  installError: string | null;

  checkForUpdates: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  dismiss: () => void;
}

async function restartAfterUpdate() {
  if (isMac()) {
    await invoke<void>("restart_after_update");
    return;
  }

  await relaunch();
}

export const useUpdateStore = create<UpdateState>((set, get) => ({
  updateAvailable: null,
  isChecking: false,
  isDownloading: false,
  isInstalling: false,
  downloadProgress: 0,
  updateRef: null,
  upToDateAt: null,
  installError: null,

  checkForUpdates: async () => {
    set({ isChecking: true });
    try {
      const update = await check();
      if (update) {
        let body = update.body;
        // If the update payload doesn't include notes, fall back to fetching
        // them from latest.json — the worker keeps human-readable changelog
        // there.
        if (!body) {
          try {
            const response = await fetch(
              "https://unlocker-releases.crosspointreader.com/latest.json",
            );
            const data = await response.json();
            body = data.notes;
          } catch (e) {
            console.error("Failed to fetch release notes:", e);
          }
        }
        set({
          updateAvailable: { version: update.version, body },
          updateRef: update,
          upToDateAt: null,
        });
      } else {
        set({ upToDateAt: Date.now() });
      }
    } catch (error) {
      console.error("Failed to check for updates:", error);
      throw error;
    } finally {
      set({ isChecking: false });
    }
  },

  downloadAndInstall: async () => {
    const { updateRef } = get();
    if (!updateRef) return;

    set({ isDownloading: true, downloadProgress: 0, installError: null });
    try {
      let contentLength = 0;
      let downloaded = 0;

      await updateRef.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            downloaded = 0;
            set({ downloadProgress: 0 });
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            const progress =
              contentLength > 0
                ? Math.round((downloaded / contentLength) * 100)
                : 0;
            set({ downloadProgress: progress });
            break;
          case "Finished":
            set({
              downloadProgress: 100,
              isDownloading: false,
              isInstalling: true,
            });
            break;
        }
      });

      await restartAfterUpdate();
    } catch (error) {
      console.error("Failed to download/install update:", error);
      const message =
        error instanceof Error
          ? `${error.name}: ${error.message}`
          : String(error);
      set({
        isDownloading: false,
        isInstalling: false,
        installError: message,
      });
    }
  },

  dismiss: () => {
    set({
      updateAvailable: null,
      updateRef: null,
      downloadProgress: 0,
      installError: null,
    });
  },
}));

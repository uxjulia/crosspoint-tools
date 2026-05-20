import { invoke } from "@tauri-apps/api/core";

export type Platform = "macos" | "windows" | "linux";

let cached: Platform = "macos";

export async function loadPlatform(): Promise<Platform> {
  cached = await invoke<Platform>("get_platform");
  return cached;
}

export function platform(): Platform {
  return cached;
}

export const isWindows = () => cached === "windows";
export const isMac = () => cached === "macos";
export const isLinux = () => cached === "linux";



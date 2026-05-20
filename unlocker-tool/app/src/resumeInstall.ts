import type { Locale, Model } from "./types";

const STORAGE_KEY = "xteink-unlocker-resume-install";

export type ResumeInstall =
  | {
      kind: "catalog";
      model: Model;
      locale: Locale;
      releaseId: string;
    }
  | {
      kind: "local";
      model: Model;
      locale: Locale;
      path: string;
    };

export function saveResumeInstall(value: ResumeInstall) {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
}

export function takeResumeInstall(): ResumeInstall | null {
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  window.localStorage.removeItem(STORAGE_KEY);

  try {
    const parsed = JSON.parse(raw) as ResumeInstall;
    if (
      (parsed.kind === "catalog" && parsed.releaseId) ||
      (parsed.kind === "local" && parsed.path)
    ) {
      return parsed;
    }
  } catch {
    return null;
  }

  return null;
}

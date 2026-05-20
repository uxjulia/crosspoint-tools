import { useEffect } from "react";
import { useUpdateStore } from "../stores/updateStore";
import { PrimaryButton, SecondaryButton } from "./ui";

/**
 * Mounts at the root. Auto-checks for updates 3s after launch and renders
 * a modal when one is available. Failures are swallowed (offline = silent).
 */
export function UpdateChecker() {
  const {
    updateAvailable,
    isDownloading,
    isInstalling,
    downloadProgress,
    installError,
    checkForUpdates,
    downloadAndInstall,
    dismiss,
  } = useUpdateStore();

  useEffect(() => {
    const timer = setTimeout(() => {
      checkForUpdates().catch(() => {
        /* silent */
      });
    }, 3000);
    return () => clearTimeout(timer);
  }, [checkForUpdates]);

  if (!updateAvailable) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-stone-950/40 px-6"
      role="dialog"
      aria-modal="true"
    >
      <div className="w-full max-w-md rounded-2xl border border-stone-200 bg-white p-6 shadow-xl">
        <div className="flex items-center gap-2">
          <span className="inline-flex items-center rounded-full bg-brand-50 px-2.5 py-0.5 text-xs font-semibold text-brand-500">
            Update available
          </span>
        </div>
        <h2 className="mt-3 font-serif text-xl font-medium text-stone-900">
          Xteink Unlocker {updateAvailable.version}
        </h2>

        {updateAvailable.body && (
          <div className="mt-3 max-h-48 overflow-auto rounded-md bg-stone-50 p-3 text-sm/6 text-stone-600 whitespace-pre-wrap">
            {updateAvailable.body}
          </div>
        )}

        {isDownloading && (
          <div className="mt-4 space-y-1.5">
            <div className="h-2 w-full overflow-hidden rounded-full bg-stone-100">
              <div
                className="h-full bg-brand-500 transition-all duration-300"
                style={{ width: `${downloadProgress}%` }}
              />
            </div>
            <p className="text-xs text-stone-500">
              Downloading… {downloadProgress}%
            </p>
          </div>
        )}

        {isInstalling && (
          <p className="mt-4 text-sm text-stone-500">
            Installing update… the app will restart in a moment.
          </p>
        )}

        {installError && (
          <div className="mt-4 rounded-md border border-red-200 bg-red-50 p-3 text-xs/5 text-red-700">
            <p className="font-semibold">Update failed</p>
            <p className="mt-1 break-words whitespace-pre-wrap font-mono">
              {installError}
            </p>
          </div>
        )}

        <div className="mt-5 flex justify-end gap-2">
          <SecondaryButton
            onClick={dismiss}
            disabled={isDownloading || isInstalling}
          >
            Later
          </SecondaryButton>
          <PrimaryButton
            onClick={() => {
              downloadAndInstall().catch(() => {
                /* logged in store */
              });
            }}
            disabled={isDownloading || isInstalling}
          >
            {isDownloading ? "Downloading…" : "Update now"}
          </PrimaryButton>
        </div>
      </div>
    </div>
  );
}

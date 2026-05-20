import { useUpdateStore } from "../stores/updateStore";

/** Footer link. Manually triggers an update check; the modal renders separately. */
export function CheckForUpdatesLink() {
  const { isChecking, checkForUpdates, upToDateAt, updateAvailable } =
    useUpdateStore();

  const recentlyChecked = upToDateAt && Date.now() - upToDateAt < 8000;

  const label = isChecking
    ? "Checking…"
    : recentlyChecked && !updateAvailable
      ? "Up to date"
      : "Check for updates";

  return (
    <button
      type="button"
      onClick={() => {
        if (!isChecking) {
          checkForUpdates().catch(() => {
            /* silent — see store */
          });
        }
      }}
      disabled={isChecking}
      className="text-xs text-stone-400 hover:text-stone-600 disabled:cursor-not-allowed"
    >
      {label}
    </button>
  );
}

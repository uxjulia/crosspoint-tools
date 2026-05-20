import { useEffect, useState } from "react";
import { api } from "../api";
import type { HelperLogTail } from "../types";

export function HelperDebugLog() {
  const [open, setOpen] = useState(false);
  const [log, setLog] = useState<HelperLogTail | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;

    let cancelled = false;
    const refresh = async () => {
      try {
        const next = await api.getHelperLogTail();
        if (!cancelled) {
          setLog(next);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    };

    refresh();
    const id = window.setInterval(refresh, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [open]);

  return (
    <details
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
      className="rounded-xl border border-stone-200 bg-stone-950 p-4 text-stone-300"
    >
      <summary className="cursor-pointer select-none text-sm font-medium text-stone-100">
        Helper debug log
      </summary>
      <div className="mt-3 space-y-2">
        {log?.path && (
          <div className="break-all font-mono text-[11px] text-stone-500">
            {log.path}
          </div>
        )}
        {error ? (
          <p className="text-xs text-red-300">{error}</p>
        ) : log?.available ? (
          <pre className="max-h-72 overflow-auto whitespace-pre-wrap rounded-md bg-black/30 p-3 font-mono text-xs leading-5 text-stone-200">
            {log.content || "log file is empty"}
          </pre>
        ) : (
          <p className="text-xs text-stone-500">No helper log yet.</p>
        )}
      </div>
    </details>
  );
}

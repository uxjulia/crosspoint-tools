import { ReactNode, useState } from "react";
import { api } from "../api";
import { CheckForUpdatesLink } from "./CheckForUpdatesLink";
import { SettingsModal } from "./SettingsModal";
import type { StateKind } from "../types";

const STEPS: { id: string; label: string; states: StateKind[] }[] = [
  { id: "consent", label: "Consent", states: ["consenting"] },
  {
    id: "device",
    label: "Device",
    states: ["selecting_device_and_region", "selecting_firmware"],
  },
  {
    id: "connect",
    label: "Connect",
    states: [
      "downloading_firmware",
      "setting_up_hotspot",
      "waiting_for_internet_sharing",
      "awaiting_client",
      "awaiting_device_request",
    ],
  },
  {
    id: "install",
    label: "Install",
    states: ["armed", "serving", "flashing"],
  },
  { id: "verify", label: "Verify", states: ["verifying", "done"] },
];

function activeIndex(state: StateKind): number {
  const idx = STEPS.findIndex((s) => s.states.includes(state));
  return idx === -1 ? 0 : idx;
}

export function Shell({
  state,
  children,
}: {
  state: StateKind;
  children: ReactNode;
}) {
  const idx = activeIndex(state);
  const isTerminal = state === "done" || state === "failed" || state === "idle";
  const [settingsOpen, setSettingsOpen] = useState(false);
  return (
    <div className="mx-auto flex min-h-full max-w-3xl flex-col px-6 py-8">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <img src="/logo.png" alt="" className="size-7 rounded-md" />
          <span className="text-sm/6 font-medium tracking-tight text-stone-900">
            Xteink Unlocker
          </span>
          <span className="ml-1 inline-flex items-center rounded-full bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-800">
            Beta
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings and recovery"
            title="Settings and recovery"
            className="rounded-md p-2 text-stone-500 hover:bg-stone-100 hover:text-stone-700"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="size-4"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
        </div>
      </header>
      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}

      <nav className="mt-6">
        <ol className="flex items-center gap-3 text-xs">
          {STEPS.map((step, i) => {
            const isActive = i === idx;
            const isDone = i < idx;
            return (
              <li key={step.id} className="flex items-center gap-3">
                <span
                  className={`flex size-6 items-center justify-center rounded-full font-mono text-[11px] font-semibold ${
                    isActive
                      ? "bg-brand-500 text-white"
                      : isDone
                        ? "bg-brand-100 text-brand-700"
                        : "bg-stone-200 text-stone-500"
                  }`}
                >
                  {isDone ? "✓" : i + 1}
                </span>
                <span
                  className={
                    isActive
                      ? "font-medium text-stone-900"
                      : isDone
                        ? "text-stone-500"
                        : "text-stone-400"
                  }
                >
                  {step.label}
                </span>
                {i < STEPS.length - 1 && (
                  <span className="h-px w-6 bg-stone-200" />
                )}
              </li>
            );
          })}
        </ol>
      </nav>

      <main className="mt-8 flex-1">{children}</main>

      <footer className="mt-10 flex items-center justify-between text-xs text-stone-400">
        <span>CrossPoint Reader · MIT licensed</span>
        <div className="flex-1 text-center">
          {!isTerminal && (
            <button
              type="button"
              onClick={() => api.cancel()}
              className="text-xs text-stone-400 underline-offset-2 hover:text-stone-600 hover:underline"
            >
              Cancel and clean up
            </button>
          )}
        </div>
        <div className="flex items-center gap-4">
          <CheckForUpdatesLink />
          <a
            href="https://crosspointreader.com"
            className="hover:text-stone-600"
            target="_blank"
            rel="noreferrer"
          >
            crosspointreader.com
          </a>
        </div>
      </footer>
    </div>
  );
}

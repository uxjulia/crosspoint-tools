import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LogEntry, StateEvent, StateKind } from "./types";

export function useStateMachine(): {
  state: StateKind;
  message: string | null;
  error: string | null;
} {
  const [state, setState] = useState<StateKind>("idle");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unlisten = listen<StateEvent>("state-changed", (e) => {
      setState(e.payload.state.kind);
      setMessage(e.payload.message);
      setError(e.payload.error);
    });
    return () => {
      unlisten.then((u) => u());
    };
  }, []);

  return { state, message, error };
}

export function useSessionLog(max = 200): LogEntry[] {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  useEffect(() => {
    const unlisten = listen<LogEntry>("log", (e) => {
      setEntries((prev) => {
        const next = [...prev, e.payload];
        return next.length > max ? next.slice(next.length - max) : next;
      });
    });
    return () => {
      unlisten.then((u) => u());
    };
  }, [max]);
  return entries;
}

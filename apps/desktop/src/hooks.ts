import { useCallback, useEffect, useState } from "react";
import { api, errorMessage } from "./api";
import type { Status } from "./protocol";

/** How often the open window re-reads the service; the countdown ticks locally. */
const POLL_MS = 5_000;

export interface LiveStatus {
  status: Status | null;
  /** Set while the service cannot be reached. */
  error: string | null;
  /** Seconds left in the session, interpolated between polls. */
  remaining: number;
  refresh: () => Promise<void>;
}

export function useStatus(): LiveStatus {
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fetchedAt, setFetchedAt] = useState(0);
  const [now, setNow] = useState(() => Date.now());

  const refresh = useCallback(async () => {
    try {
      const next = await api.status();
      setStatus(next);
      setFetchedAt(Date.now());
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const poll = setInterval(() => void refresh(), POLL_MS);
    const tick = setInterval(() => setNow(Date.now()), 1_000);
    const onVisible = () => {
      if (!document.hidden) void refresh();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      clearInterval(poll);
      clearInterval(tick);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [refresh]);

  const session = status?.session ?? null;
  const remaining = session ? Math.max(0, session.remaining - (now - fetchedAt) / 1_000) : 0;
  const ended = session !== null && remaining === 0;
  useEffect(() => {
    if (ended) void refresh();
  }, [ended, now, refresh]);

  return { status, error, remaining, refresh };
}

/** Runs one async action at a time and keeps its error for display. */
export function useAction() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(async (action: () => Promise<unknown>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      await action();
      return true;
    } catch (e) {
      setError(errorMessage(e));
      return false;
    } finally {
      setBusy(false);
    }
  }, []);
  return { busy, error, run, clearError: () => setError(null) };
}

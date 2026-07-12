"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";
import type { AccountUser } from "@/lib/types";

interface SyncInfo {
  finishedAt: number | null;
  newCount: number;
  status: string;
}

interface SessionState {
  authenticated: boolean;
  user: AccountUser | null;
  historyCount: number;
  lastSync: SyncInfo | null;
  syncing: boolean;
  loading: boolean;
  refresh: () => Promise<void>;
}

const SessionContext = createContext<SessionState | null>(null);

export function SessionProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<Omit<SessionState, "refresh">>({
    authenticated: false,
    user: null,
    historyCount: 0,
    lastSync: null,
    syncing: false,
    loading: true,
  });

  const refresh = useCallback(async () => {
    try {
      const res = await fetch("/api/session", { cache: "no-store" });
      const data = await res.json();
      setState({
        authenticated: !!data.authenticated,
        user: data.user ?? null,
        historyCount: data.historyCount ?? 0,
        lastSync: data.lastSync ?? null,
        syncing: !!data.syncing,
        loading: false,
      });
    } catch {
      setState((s) => ({ ...s, loading: false }));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <SessionContext.Provider value={{ ...state, refresh }}>
      {children}
    </SessionContext.Provider>
  );
}

export function useSession(): SessionState {
  const ctx = useContext(SessionContext);
  if (!ctx) throw new Error("useSession must be used within SessionProvider");
  return ctx;
}

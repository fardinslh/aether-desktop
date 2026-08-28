import { useState, useEffect, useCallback, useRef } from "react";
import { AppSettings, ApplicationRule, ConnectionState, HealthStatus, LogEntry, RouteDestination } from "../types";
import { api } from "../services/api";
import { listen } from "@tauri-apps/api/event";

export function useAppStore() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [connectionState, setConnectionState] = useState<ConnectionState>("DISCONNECTED");
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isApplyingRouting, setIsApplyingRouting] = useState<boolean>(false);
  const [errorDetails, setErrorDetails] = useState<string | null>(null);

  const stateRef = useRef<ConnectionState>(connectionState);
  stateRef.current = connectionState;

  // Fetch initial state
  const refreshAll = useCallback(async () => {
    try {
      const [fetchedSettings, fetchedState, fetchedHealth, fetchedLogs] = await Promise.all([
        api.getSettings(),
        api.getConnectionState(),
        api.getHealthStatus(),
        api.getLogs(),
      ]);

      setSettings(fetchedSettings);
      setConnectionState(fetchedState);
      setHealth(fetchedHealth);
      setLogs(fetchedLogs);
    } catch (err) {
      console.error("Error refreshing app state:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshAll();

    // 1. Event-driven connection state updates
    let unlistenFn: (() => void) | null = null;
    listen<ConnectionState>("connection-state-changed", (event) => {
      if (event.payload) {
        setConnectionState(event.payload);
      }
    }).then((unlisten) => {
      unlistenFn = unlisten;
    }).catch((err) => {
      console.warn("Failed to attach Tauri event listener:", err);
    });

    // 2. Adaptive polling reconciliation
    let timeoutId: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const [st, hl, lg] = await Promise.all([
          api.getConnectionState(),
          api.getHealthStatus(),
          api.getLogs(),
        ]);
        setConnectionState(st);
        setHealth(hl);
        setLogs(lg);
      } catch {
        // network polling error
      }

      const isTransitioning =
        stateRef.current !== "CONNECTED" &&
        stateRef.current !== "DISCONNECTED" &&
        stateRef.current !== "ERROR";

      timeoutId = setTimeout(poll, isTransitioning ? 250 : 2000);
    };

    timeoutId = setTimeout(poll, 1000);

    return () => {
      if (unlistenFn) unlistenFn();
      clearTimeout(timeoutId);
    };
  }, [refreshAll]);

  const updateSettings = async (newSettings: AppSettings) => {
    if (connectionState === "CONNECTED") {
      setIsApplyingRouting(true);
    }
    try {
      await api.saveSettings(newSettings);
      setSettings(newSettings);
      setErrorDetails(null);
    } catch (err: any) {
      console.error("Failed to save/apply settings:", err);
      const msg = err?.toString() || "Failed to apply live routing changes";
      setErrorDetails(msg);
      throw err;
    } finally {
      setIsApplyingRouting(false);
    }
  };

  const addApplicationRule = async (rule: ApplicationRule) => {
    if (!settings) return;
    const updated = {
      ...settings,
      applicationRules: [...settings.applicationRules, rule],
    };
    await updateSettings(updated);
  };

  const updateApplicationRule = async (updatedRule: ApplicationRule) => {
    if (!settings) return;
    const updated = {
      ...settings,
      applicationRules: settings.applicationRules.map((r) =>
        r.id === updatedRule.id ? updatedRule : r
      ),
    };
    await updateSettings(updated);
  };

  const toggleApplicationRule = async (id: string, enabled: boolean) => {
    if (!settings) return;
    const updated = {
      ...settings,
      applicationRules: settings.applicationRules.map((r) => (r.id === id ? { ...r, enabled } : r)),
    };
    await updateSettings(updated);
  };

  const deleteApplicationRule = async (id: string) => {
    if (!settings) return;
    const updated = {
      ...settings,
      applicationRules: settings.applicationRules.filter((r) => r.id !== id),
    };
    await updateSettings(updated);
  };

  const updateApplicationRoute = async (id: string, destination: RouteDestination) => {
    if (!settings) return;
    const updated = {
      ...settings,
      applicationRules: settings.applicationRules.map((r) =>
        r.id === id
          ? {
              ...r,
              destination,
            }
          : r
      ),
    };
    await updateSettings(updated);
  };

  // Immediate State Transition Triggers
  const triggerConnect = async () => {
    setErrorDetails(null);
    // Instant 0ms visual feedback on click
    setConnectionState("STARTING_AETHER");
    try {
      await api.connect();
      const [st, h] = await Promise.all([api.getConnectionState(), api.getHealthStatus()]);
      setConnectionState(st);
      setHealth(h);
    } catch (err: any) {
      setConnectionState("ERROR");
      setErrorDetails(err?.toString() || "Connection error");
    }
  };

  const triggerDisconnect = async () => {
    // Instant 0ms visual feedback on click
    setConnectionState("DISCONNECTING");
    try {
      await api.disconnect();
      const st = await api.getConnectionState();
      setConnectionState(st);
    } catch (err: any) {
      console.error("Disconnect error:", err);
    }
  };

  return {
    settings,
    connectionState,
    health,
    logs,
    isLoading,
    isApplyingRouting,
    errorDetails,
    updateSettings,
    addApplicationRule,
    updateApplicationRule,
    toggleApplicationRule,
    deleteApplicationRule,
    updateApplicationRoute,
    triggerConnect,
    triggerDisconnect,
    refreshAll,
  };
}
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

  // Request epoch / version counter to prevent stale polling responses from overwriting newer event states
  const stateVersionRef = useRef<number>(0);
  const connectInFlightRef = useRef<boolean>(false);
  const disconnectInFlightRef = useRef<boolean>(false);

  // Fetch initial state
  const refreshAll = useCallback(async () => {
    const versionAtStart = stateVersionRef.current;
    try {
      const [fetchedSettings, fetchedHealth, fetchedLogs] = await Promise.all([
        api.getSettings(),
        api.getHealthStatus(),
        api.getLogs(),
      ]);

      setSettings(fetchedSettings);
      setHealth(fetchedHealth);
      setLogs(fetchedLogs);

      // Fetch connection state AFTER slow requests and apply only if version didn't change
      const fetchedState = await api.getConnectionState();
      if (stateVersionRef.current === versionAtStart) {
        setConnectionState(fetchedState);
      }
    } catch (err) {
      console.error("Error refreshing app state:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshAll();

    // 1. Authoritative Event-driven connection state updates
    let unlistenFn: (() => void) | null = null;
    listen<ConnectionState>("connection-state-changed", (event) => {
      if (event.payload) {
        stateVersionRef.current += 1;
        setConnectionState(event.payload);
      }
    }).then((unlisten) => {
      unlistenFn = unlisten;
    }).catch((err) => {
      console.warn("Failed to attach Tauri event listener:", err);
    });

    // 2. Adaptive polling reconciliation with epoch protection
    let timeoutId: ReturnType<typeof setTimeout>;
    const poll = async () => {
      const isTransitioning =
        stateRef.current !== "CONNECTED" &&
        stateRef.current !== "DISCONNECTED" &&
        stateRef.current !== "ERROR";

      const versionAtStart = stateVersionRef.current;

      try {
        if (isTransitioning) {
          // During transitional states, NEVER execute expensive health probes.
          // Only fetch lightweight connection state and logs.
          const [st, lg] = await Promise.all([
            api.getConnectionState(),
            api.getLogs(),
          ]);
          if (stateVersionRef.current === versionAtStart) {
            setConnectionState(st);
          }
          setLogs(lg);
        } else {
          // When state is stable, poll health and logs, then check connection state after slow health
          const [hl, lg] = await Promise.all([
            api.getHealthStatus(),
            api.getLogs(),
          ]);
          setHealth(hl);
          setLogs(lg);

          const st = await api.getConnectionState();
          if (stateVersionRef.current === versionAtStart) {
            setConnectionState(st);
          }
        }
      } catch {
        // network polling error ignored
      }

      const nextTransitioning =
        stateRef.current !== "CONNECTED" &&
        stateRef.current !== "DISCONNECTED" &&
        stateRef.current !== "ERROR";

      timeoutId = setTimeout(poll, nextTransitioning ? 800 : 2000);
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

  // Immediate State Transition Triggers with in-flight click protection
  const triggerConnect = async () => {
    if (connectInFlightRef.current) {
      console.log("Connect command already in flight, ignoring duplicate trigger");
      return;
    }
    connectInFlightRef.current = true;
    stateVersionRef.current += 1;
    setErrorDetails(null);
    // Instant visual feedback
    setConnectionState("STARTING_AETHER");

    try {
      await api.connect();
      stateVersionRef.current += 1;
      const st = await api.getConnectionState();
      setConnectionState(st);
      if (st === "CONNECTED") {
        const hl = await api.getHealthStatus();
        setHealth(hl);
      }
    } catch (err: any) {
      stateVersionRef.current += 1;
      setConnectionState("ERROR");
      setErrorDetails(err?.toString() || "Connection error");
    } finally {
      connectInFlightRef.current = false;
    }
  };

  const optimizeInFlightRef = useRef<boolean>(false);
  const triggerFindFasterGateway = async () => {
    if (optimizeInFlightRef.current) {
      console.log("Optimization command already in flight, ignoring duplicate trigger");
      return;
    }
    optimizeInFlightRef.current = true;
    stateVersionRef.current += 1;
    setErrorDetails(null);
    setConnectionState("SCANNING_AETHER");

    try {
      const res = await api.findFasterGateway();
      stateVersionRef.current += 1;
      const st = await api.getConnectionState();
      setConnectionState(st);
      if (st === "CONNECTED") {
        const hl = await api.getHealthStatus();
        setHealth(hl);
      }
      return res;
    } catch (err: any) {
      stateVersionRef.current += 1;
      const st = await api.getConnectionState();
      setConnectionState(st);
      setErrorDetails(err?.toString() || "Gateway scan failed");
      throw err;
    } finally {
      optimizeInFlightRef.current = false;
    }
  };

  const triggerDisconnect = async () => {
    if (disconnectInFlightRef.current) {
      console.log("Disconnect command already in flight, ignoring duplicate trigger");
      return;
    }
    disconnectInFlightRef.current = true;
    stateVersionRef.current += 1;
    // Instant visual feedback
    setConnectionState("DISCONNECTING");

    try {
      await api.disconnect();
      stateVersionRef.current += 1;
      const st = await api.getConnectionState();
      setConnectionState(st);
    } catch (err: any) {
      console.error("Disconnect error:", err);
    } finally {
      disconnectInFlightRef.current = false;
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
    triggerFindFasterGateway,
    triggerDisconnect,
    refreshAll,
  };
}
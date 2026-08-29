import React, { useState, useEffect } from "react";
import {
  Power,
  Globe,
  ShieldCheck,
  AlertTriangle,
  Radio,
  Network,
  Cpu,
  Activity,
  Terminal,
  Zap,
  CheckCircle2,
  X,
} from "lucide-react";
import { ConnectionState, HealthStatus, RouteOptimizationResult } from "../../types";
import { api } from "../../services/api";

interface ConnectionHeroProps {
  connectionState: ConnectionState;
  health: HealthStatus | null;
  onConnect: () => void;
  onDisconnect: () => void;
  onFindFasterGateway?: () => Promise<RouteOptimizationResult | void>;
  onViewDiagnostics?: () => void;
  errorDetails?: string | null;
}

export const ConnectionHero: React.FC<ConnectionHeroProps> = ({
  connectionState,
  health,
  onConnect,
  onDisconnect,
  onFindFasterGateway,
  onViewDiagnostics,
  errorDetails,
}) => {
  const [showConfirmModal, setShowConfirmModal] = useState<boolean>(false);
  const [isOptimizing, setIsOptimizing] = useState<boolean>(false);
  const [optimizationResult, setOptimizationResult] = useState<RouteOptimizationResult | null>(null);
  const [optimizationError, setOptimizationError] = useState<string | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState<number>(0);
  const [bestCandidateRtt, setBestCandidateRtt] = useState<number | null>(null);

  const isConnected = connectionState === "CONNECTED";
  const isDisconnected = connectionState === "DISCONNECTED";
  const isError = connectionState === "ERROR";
  const isTransitioning = !isConnected && !isDisconnected && !isError;

  const handleOptimizeClick = () => {
    if (isConnected) {
      setShowConfirmModal(true);
    } else {
      executeFindFasterGateway();
    }
  };

  const executeFindFasterGateway = async () => {
    setShowConfirmModal(false);
    setIsOptimizing(true);
    setOptimizationError(null);
    try {
      if (onFindFasterGateway) {
        const res = await onFindFasterGateway();
        if (res) {
          setOptimizationResult(res);
        }
      }
    } catch (err: any) {
      setOptimizationError(err?.message || err?.toString() || "Optimization failed");
    } finally {
      setIsOptimizing(false);
    }
  };

  useEffect(() => {
    let timer: any = null;
    let pollTimer: any = null;

    if (isTransitioning) {
      setElapsedSeconds(0);
      setBestCandidateRtt(null);

      timer = setInterval(() => {
        setElapsedSeconds((prev) => prev + 1);
      }, 1000);

      const pollCandidate = async () => {
        try {
          const rtt = await api.getBestCandidateRtt();
          if (rtt) {
            setBestCandidateRtt(rtt);
          }
        } catch (_) {}
      };
      pollCandidate();
      pollTimer = setInterval(pollCandidate, 1500);
    } else {
      setElapsedSeconds(0);
      setBestCandidateRtt(null);
    }

    return () => {
      if (timer) clearInterval(timer);
      if (pollTimer) clearInterval(pollTimer);
    };
  }, [isTransitioning]);

  const formatElapsed = (seconds: number) => {
    const m = Math.floor(seconds / 60)
      .toString()
      .padStart(2, "0");
    const s = (seconds % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  };

  const getSubStatusText = () => {
    if (isConnected) {
      const trace = health?.cloudflareTrace;
      if (trace && trace.colo && trace.latencyMs !== undefined) {
        return `${getCityFromColo(trace.colo)} (${trace.colo}) · ${trace.latencyMs} ms · Egress: ${trace.ip}`;
      }
      return "Connected · Validating network egress telemetry...";
    }
    if (
      connectionState === "STARTING_AETHER" ||
      connectionState === "WAITING_FOR_AETHER"
    ) {
      return "Spawning managed Aether WireGuard/Shadowsocks daemon...";
    }
    if (connectionState === "SCANNING_AETHER") {
      return "Probing resilient Aether gateway candidates (Thorough Mode)...";
    }
    if (connectionState === "TESTING_AETHER") {
      return "Verifying SOCKS5 proxy handshake on 127.0.0.1:1819...";
    }
    if (connectionState === "STARTING_ROUTER") {
      return "Initializing sing-box Wintun driver & network adapter...";
    }
    if (connectionState === "TESTING_ROUTING") {
      return "Executing 3-stage egress routing & DNS verification...";
    }
    if (isError) {
      return errorDetails || "Network subsystem encountered an unrecoverable routing error.";
    }
    return "Routing engine standby · Ready to establish isolated TUN tunnel";
  };

  return (
    <div className="flex flex-col items-center px-4 pt-2 pb-1 select-none">
      {/* Precision Routing Topology Display */}
      <div className="w-full max-w-xl bg-app-panel border border-app-border rounded-md p-4 flex flex-col items-center relative overflow-hidden shadow-sm">
        {/* Subtle background rail line */}
        <div className="absolute inset-x-0 top-0 h-0.5 bg-gradient-to-r from-transparent via-app-border to-transparent" />

        {/* Top: Public Gateway Node with Find Faster Gateway Trigger */}
        <div className="w-full flex items-center justify-between px-3 py-1.5 rounded-sm bg-app-inset border border-app-border-subtle text-xs font-mono mb-3">
          <div className="flex items-center gap-2">
            <Globe className={`w-3.5 h-3.5 ${isConnected ? "text-signal-green" : isTransitioning ? "text-signal-cyan animate-pulse" : "text-ink-400"}`} />
            <span className="text-ink-300 uppercase tracking-wide text-[11px] font-medium">WAN Gateway:</span>
            <span className={`text-[11px] ${isConnected ? "text-signal-green font-semibold" : "text-ink-400"}`}>
              {isConnected && health?.cloudflareTrace
                ? `${health.cloudflareTrace.ip} (${health.cloudflareTrace.colo})`
                : isTransitioning
                ? "Negotiating..."
                : "Standby"}
            </span>
          </div>

          <div className="flex items-center gap-2.5 text-[11px]">
            {isConnected && health?.cloudflareTrace?.latencyMs !== undefined && (
              <span className="text-signal-green flex items-center gap-1">
                <Activity className="w-3 h-3" />
                <span>{health.cloudflareTrace.latencyMs} ms</span>
              </span>
            )}
            
            {/* Find Faster Gateway button */}
            <button
              onClick={handleOptimizeClick}
              disabled={isTransitioning || isOptimizing}
              className="flex items-center gap-1 px-2 py-0.5 rounded-xs bg-app-surface hover:bg-app-panel border border-app-border hover:border-signal-cyan/60 text-ink-200 hover:text-signal-cyan text-[10px] font-mono transition-all disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
              title={isConnected ? "Perform a fresh Thorough sweep for a lower latency gateway" : "Connect using a fresh Thorough candidate sweep"}
            >
              <Zap className={`w-3 h-3 ${isOptimizing ? "animate-pulse text-signal-cyan" : "text-signal-cyan"}`} />
              <span>{isConnected ? "Find Faster Gateway" : "Find Best Gateway"}</span>
            </button>

            <span className="flex items-center gap-1.5 ml-1">
              <span className={`w-1.5 h-1.5 rounded-full ${isConnected ? "bg-signal-green" : isTransitioning ? "bg-signal-cyan animate-pulse" : "bg-ink-500"}`} />
              <span className="text-ink-400 font-sans text-[10px] uppercase">
                {isConnected ? "Active" : isTransitioning ? "Syncing" : "Standby"}
              </span>
            </span>
          </div>
        </div>

        {/* Vertical Highway Signal Lines */}
        <div className="flex flex-col items-center my-0.5 relative">
          <div className={`w-0.5 h-4 transition-colors duration-500 ${isConnected ? "bg-signal-green" : isTransitioning ? "bg-signal-cyan" : "bg-app-border"}`} />
          <div className={`w-2 h-2 rounded-full border transition-all ${isConnected ? "border-signal-green bg-signal-green/30" : isTransitioning ? "border-signal-cyan bg-signal-cyan/40 animate-ping" : "border-app-border bg-app-inset"}`} />
          <div className={`w-0.5 h-4 transition-colors duration-500 ${isConnected ? "bg-signal-green" : isTransitioning ? "bg-signal-cyan" : "bg-app-border"}`} />
        </div>

        {/* Route Optimization Result / Telemetry Diff Card */}
        {optimizationResult && (
          <div
            className={`w-full max-w-md my-1.5 p-2.5 rounded-sm border text-xs font-mono flex items-start justify-between gap-2 transition-all shadow-sm ${
              optimizationResult.success
                ? (optimizationResult.latencyDeltaMs ?? 0) > 0
                  ? "bg-signal-green-dim border-signal-green/40 text-signal-green"
                  : "bg-signal-cyan-dim border-signal-cyan/40 text-signal-cyan"
                : "bg-signal-amber-dim border-signal-amber/40 text-signal-amber"
            }`}
          >
            <div className="flex items-start gap-2">
              {optimizationResult.success ? (
                <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0" />
              ) : (
                <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0" />
              )}
              <div>
                <div className="font-bold uppercase tracking-wider text-[10px]">
                  {optimizationResult.success
                    ? (optimizationResult.latencyDeltaMs ?? 0) > 0
                      ? "Gateway Optimized"
                      : "Fresh Scan Complete"
                    : "Previous Working Gateway Restored"}
                </div>
                <div className="text-[11px] mt-0.5 text-ink-200">
                  {optimizationResult.previousLatencyMs !== undefined &&
                  optimizationResult.previousLatencyMs !== null ? (
                    <span>
                      Previous:{" "}
                      <span className="text-ink-400 font-semibold">
                        {optimizationResult.previousLatencyMs} ms
                        {optimizationResult.previousPop && ` (${optimizationResult.previousPop})`}
                      </span>
                      {optimizationResult.newLatencyMs !== undefined &&
                        optimizationResult.newLatencyMs !== null && (
                          <span>
                            {" "}→ New:{" "}
                            <span className="font-bold text-ink-100">
                              {optimizationResult.newLatencyMs} ms
                              {optimizationResult.newPop && ` (${optimizationResult.newPop})`}
                            </span>
                          </span>
                        )}
                      {optimizationResult.latencyDeltaMs !== undefined &&
                        optimizationResult.latencyDeltaMs !== null &&
                        optimizationResult.latencyDeltaMs > 0 && (
                          <span className="text-signal-green font-bold ml-1">
                            (+{optimizationResult.latencyDeltaMs} ms faster)
                          </span>
                        )}
                    </span>
                  ) : (
                    <span>{optimizationResult.message}</span>
                  )}
                </div>
              </div>
            </div>
            <button
              onClick={() => setOptimizationResult(null)}
              className="text-ink-400 hover:text-ink-100 p-0.5 cursor-pointer"
              title="Dismiss notification"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {optimizationError && (
          <div className="w-full max-w-md my-1.5 p-2 rounded-sm border bg-signal-red-dim border-signal-red/40 text-signal-red text-xs font-mono flex items-center justify-between">
            <div className="flex items-center gap-1.5">
              <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
              <span>{optimizationError}</span>
            </div>
            <button
              onClick={() => setOptimizationError(null)}
              className="text-signal-red/70 hover:text-signal-red p-0.5 cursor-pointer"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Center: The Precision Routing Core Module */}
        <div className="w-full max-w-md my-1">
          <div className={`w-full rounded-md border p-3.5 transition-all duration-300 ${
            isConnected
              ? "bg-app-surface border-signal-green/40 shadow-[0_0_20px_rgba(16,185,129,0.06)]"
              : isTransitioning
              ? "bg-app-surface border-signal-cyan/50 shadow-[0_0_20px_rgba(0,210,255,0.06)]"
              : isError
              ? "bg-app-surface border-signal-red/50 shadow-[0_0_20px_rgba(239,68,68,0.08)]"
              : "bg-app-surface border-app-border hover:border-ink-500"
          }`}>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <div className={`p-1.5 rounded-sm border ${
                  isConnected
                    ? "bg-signal-green-dim border-signal-green/30 text-signal-green"
                    : isTransitioning
                    ? "bg-signal-cyan-dim border-signal-cyan/30 text-signal-cyan animate-pulse"
                    : isError
                    ? "bg-signal-red-dim border-signal-red/30 text-signal-red"
                    : "bg-app-inset border-app-border text-ink-400"
                }`}>
                  <Cpu className="w-4 h-4" />
                </div>
                <div>
                  <div className="text-xs font-semibold tracking-wide text-ink-100 flex items-center gap-2">
                    <span>ROUTING CORE</span>
                    <span className="text-[10px] font-mono font-normal px-1.5 py-0.2 rounded-xs bg-app-inset border border-app-border text-ink-300">
                      singbox-tun
                    </span>
                  </div>
                  <div className="text-[11px] text-ink-400 font-mono">
                    {isConnected
                      ? "172.19.0.1/30 · Strict Mode"
                      : isTransitioning
                      ? "Configuring adapter stack..."
                      : "Ready to route system egress"}
                  </div>
                </div>
              </div>

              {/* State Chip */}
              <div className={`px-2 py-0.5 rounded-sm text-[10px] font-mono uppercase tracking-wider font-semibold border ${
                isConnected
                  ? "bg-signal-green-dim text-signal-green border-signal-green/30"
                  : isTransitioning
                  ? "bg-signal-cyan-dim text-signal-cyan border-signal-cyan/30 animate-pulse"
                  : isError
                  ? "bg-signal-red-dim text-signal-red border-signal-red/30"
                  : "bg-app-inset text-ink-400 border-app-border"
              }`}>
                {connectionState}
              </div>
            </div>

            {/* Real Transition Progress & Telemetry */}
            {isTransitioning && (
              <div className="mb-3 p-2.5 bg-app-inset rounded-sm border border-signal-cyan/30 shadow-inner">
                <div className="flex items-center justify-between text-[10px] font-mono text-ink-300 mb-1">
                  <span className="text-signal-cyan font-bold tracking-wider flex items-center gap-1.5">
                    <Activity className="w-3 h-3 animate-pulse text-signal-cyan" />
                    {connectionState === "SCANNING_AETHER" || isOptimizing
                      ? "GATEWAY SCAN"
                      : connectionState === "STARTING_AETHER" || connectionState === "WAITING_FOR_AETHER"
                      ? "GATEWAY INITIALIZATION"
                      : connectionState === "STARTING_ROUTER"
                      ? "ROUTER SETUP"
                      : connectionState === "TESTING_ROUTING"
                      ? "VERIFICATION"
                      : "NETWORK TRANSITION"}
                  </span>
                  <span className="text-ink-400 font-mono">
                    Elapsed: <strong className="text-ink-200 font-semibold">{formatElapsed(elapsedSeconds)}</strong>
                  </span>
                </div>

                <div className="flex items-center justify-between text-[11px] font-mono text-ink-400 mt-1">
                  <span className="truncate pr-2">
                    {connectionState === "SCANNING_AETHER" || isOptimizing
                      ? "Searching for a faster gateway..."
                      : connectionState === "STARTING_ROUTER"
                      ? "Configuring Wintun adapter..."
                      : connectionState === "TESTING_ROUTING"
                      ? "Validating 3-stage routing..."
                      : "Starting managed Aether daemon..."}
                  </span>
                  <span className="shrink-0 text-ink-300 text-[10px]">
                    Best candidate:{" "}
                    <strong className={bestCandidateRtt ? "text-signal-green font-bold" : "text-ink-500"}>
                      {bestCandidateRtt ? `${bestCandidateRtt} ms` : "—"}
                    </strong>
                  </span>
                </div>

                {/* Subtle pulsing activity rail */}
                <div className="w-full bg-app-panel h-1 rounded-xs overflow-hidden mt-2 relative">
                  <div className="absolute inset-0 bg-gradient-to-r from-signal-cyan/10 via-signal-cyan to-signal-cyan/10 animate-[pulse_1.5s_ease-in-out_infinite]" />
                </div>
              </div>
            )}

            {/* Primary Operation Button */}
            {isConnected ? (
              <div className="flex items-center gap-2">
                <div className="flex-1 px-3 py-2 rounded-sm bg-signal-green-dim border border-signal-green/30 flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-signal-green">
                    <ShieldCheck className="w-4 h-4" />
                    <span>TUNNEL ROUTING ACTIVE</span>
                  </div>
                  <span className="text-[10px] font-mono text-signal-green/80">SOCKS 1819</span>
                </div>
                <button
                  onClick={onDisconnect}
                  className="px-4 py-2 rounded-sm bg-app-elevated hover:bg-zinc-800 text-ink-200 hover:text-white text-xs font-semibold border border-app-border hover:border-ink-400 transition-all cursor-pointer flex items-center gap-1.5"
                >
                  <Power className="w-3.5 h-3.5 text-signal-red" />
                  <span>Disconnect</span>
                </button>
              </div>
            ) : isError ? (
              <div className="flex items-center gap-2">
                <button
                  onClick={onConnect}
                  className="flex-1 py-2 rounded-sm bg-signal-red hover:bg-signal-red-muted text-white text-xs font-bold transition-all shadow-sm cursor-pointer flex items-center justify-center gap-1.5"
                >
                  <AlertTriangle className="w-3.5 h-3.5" />
                  <span>RETRY CONNECTION</span>
                </button>
                {onViewDiagnostics && (
                  <button
                    onClick={onViewDiagnostics}
                    className="px-3 py-2 rounded-sm bg-app-elevated hover:bg-zinc-800 text-ink-300 text-xs font-medium border border-app-border transition-colors cursor-pointer flex items-center gap-1"
                  >
                    <Terminal className="w-3.5 h-3.5" />
                    <span>Logs</span>
                  </button>
                )}
              </div>
            ) : isTransitioning ? (
              <button
                disabled
                className="w-full py-2.5 rounded-sm bg-app-inset text-signal-cyan border border-signal-cyan/40 text-xs font-mono font-semibold flex items-center justify-center gap-2 cursor-wait"
              >
                <div className="w-3.5 h-3.5 border-2 border-signal-cyan border-t-transparent rounded-full animate-spin" />
                <span>ESTABLISHING ROUTING STACK...</span>
              </button>
            ) : (
              <button
                onClick={onConnect}
                className="w-full py-2.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black font-bold text-xs tracking-wide transition-all shadow-sm hover:shadow-[0_0_15px_rgba(0,210,255,0.25)] cursor-pointer flex items-center justify-center gap-2"
              >
                <Power className="w-4 h-4 stroke-[2.5]" />
                <span>INITIALIZE TUNNEL ROUTING</span>
              </button>
            )}
          </div>
        </div>

        {/* Substatus Telemetry Line */}
        <p className="text-[11px] text-ink-400 text-center font-normal mt-2 max-w-lg">
          {getSubStatusText()}
        </p>

        {/* Bottom: Split Routing Destination Highways */}
        <div className="w-full mt-3 pt-3 border-t border-app-border-subtle grid grid-cols-3 gap-2 text-center text-xs font-mono">
          <div className="p-2 rounded-sm bg-app-inset border border-app-border-subtle flex flex-col items-center">
            <div className="flex items-center gap-1 text-[10px] text-ink-400 uppercase">
              <Radio className={`w-3 h-3 ${isConnected ? "text-signal-green" : "text-ink-500"}`} />
              <span>Aether TUN</span>
            </div>
            <div className={`text-xs font-semibold mt-0.5 ${isConnected ? "text-signal-green" : "text-ink-400"}`}>
              {isConnected ? "Port 1819" : "Standby"}
            </div>
          </div>

          <div className="p-2 rounded-sm bg-app-inset border border-app-border-subtle flex flex-col items-center">
            <div className="flex items-center gap-1 text-[10px] text-ink-400 uppercase">
              <Network className="w-3 h-3 text-signal-amber" />
              <span>Secondary</span>
            </div>
            <div className="text-xs font-semibold text-ink-300 mt-0.5">
              Port 10808
            </div>
          </div>

          <div className="p-2 rounded-sm bg-app-inset border border-app-border-subtle flex flex-col items-center">
            <div className="flex items-center gap-1 text-[10px] text-ink-400 uppercase">
              <Globe className="w-3 h-3 text-signal-cyan" />
              <span>Direct LAN</span>
            </div>
            <div className="text-xs font-semibold text-ink-300 mt-0.5">
              Bypass Rules
            </div>
          </div>
        </div>
      </div>

      {/* Confirmation Modal for Find Faster Gateway while Connected */}
      {showConfirmModal && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-xs flex items-center justify-center p-4">
          <div className="w-full max-w-sm bg-app-panel border border-app-border rounded-md p-4 shadow-2xl text-ink-200 select-none animate-in fade-in zoom-in-95 duration-150">
            <div className="flex items-center gap-2 mb-2.5 text-signal-cyan">
              <Zap className="w-4 h-4" />
              <h3 className="text-xs font-bold font-mono uppercase tracking-wider">
                Search for a Faster Gateway
              </h3>
            </div>
            <p className="text-xs text-ink-300 mb-3 leading-relaxed">
              Finding a faster gateway performs a fresh network scan and temporarily interrupts the active connection. A thorough scan sweeps full candidate ranges.
            </p>
            <div className="p-2 rounded-sm bg-app-inset border border-app-border-subtle text-[11px] text-ink-400 font-mono mb-4">
              Your previous working gateway will be restored automatically if a responsive alternative is not found.
            </div>
            <div className="flex items-center justify-end gap-2 text-xs font-mono">
              <button
                onClick={() => setShowConfirmModal(false)}
                className="px-3 py-1.5 rounded-sm bg-app-surface hover:bg-app-inset border border-app-border text-ink-300 hover:text-ink-100 transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={executeFindFasterGateway}
                className="px-3 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black font-bold flex items-center gap-1.5 transition-all cursor-pointer shadow-sm"
              >
                <Zap className="w-3.5 h-3.5 fill-black" />
                <span>Start Scan</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

function getCityFromColo(colo: string): string {
  const map: Record<string, string> = {
    FRA: "Frankfurt",
    DUS: "Dusseldorf",
    AMS: "Amsterdam",
    LHR: "London",
    CDG: "Paris",
    VIE: "Vienna",
    ZRH: "Zurich",
    IST: "Istanbul",
    DXB: "Dubai",
    SIN: "Singapore",
    NRT: "Tokyo",
    HKG: "Hong Kong",
    LAX: "Los Angeles",
    SFO: "San Francisco",
    JFK: "New York",
  };
  return map[colo.toUpperCase()] || colo;
}


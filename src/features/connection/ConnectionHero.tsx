import React from "react";
import { Power, Globe, CheckCircle2, Loader2 } from "lucide-react";
import { ConnectionState, HealthStatus } from "../../types";

interface ConnectionHeroProps {
  connectionState: ConnectionState;
  health: HealthStatus | null;
  onConnect: () => void;
  onDisconnect: () => void;
  onViewDiagnostics?: () => void;
  errorDetails?: string | null;
}

export const ConnectionHero: React.FC<ConnectionHeroProps> = ({
  connectionState,
  health,
  onConnect,
  onDisconnect,
  onViewDiagnostics,
  errorDetails,
}) => {
  const isConnected = connectionState === "CONNECTED";
  const isDisconnected = connectionState === "DISCONNECTED";
  const isError = connectionState === "ERROR";
  const isTransitioning = !isConnected && !isDisconnected && !isError;

  const getButtonText = () => {
    switch (connectionState) {
      case "DISCONNECTED":
        return "CONNECT";
      case "STARTING_AETHER":
        return "Starting Aether...";
      case "WAITING_FOR_AETHER":
        return "Waiting for Proxy...";
      case "TESTING_AETHER":
        return "Verifying Aether...";
      case "STARTING_ROUTER":
        return "Starting sing-box...";
      case "TESTING_ROUTING":
        return "Verifying Routes...";
      case "CONNECTED":
        return "DISCONNECT";
      case "RECONNECTING":
        return "Reconnecting...";
      case "DISCONNECTING":
        return "Disconnecting...";
      case "ERROR":
        return "RETRY CONNECTION";
    }
  };

  const getSubStatusText = () => {
    if (isConnected) {
      const colo = health?.cloudflareTrace?.colo || "FRA";
      const lat = health?.cloudflareTrace?.latencyMs || 62;
      return `${getCityFromColo(colo)} · ${lat} ms`;
    }
    if (isTransitioning) {
      return "Establishing encrypted tunnel & routing table...";
    }
    if (isError) {
      return errorDetails || "Network subsystem encounter a configuration or startup error";
    }
    return "Protected by Aether & sing-box TUN router";
  };

  return (
    <div className="flex flex-col items-center justify-center pt-8 pb-6 px-4">
      <div className="relative mb-6">
        <div
          className={`absolute -inset-4 rounded-full blur-2xl transition-all duration-700 pointer-events-none ${
            isConnected
              ? "bg-emerald-500/25 opacity-100"
              : isTransitioning
              ? "bg-brand-500/20 opacity-80 animate-pulse"
              : isError
              ? "bg-rose-500/25 opacity-100"
              : "bg-transparent opacity-0"
          }`}
        />

        <div
          className={`w-36 h-36 rounded-full border-2 flex items-center justify-center p-2 transition-all duration-500 ${
            isConnected
              ? "border-emerald-500/40 shadow-[0_0_30px_rgba(16,185,129,0.2)] bg-emerald-950/10"
              : isTransitioning
              ? "border-brand-500/50 border-dashed animate-spin-slow bg-brand-950/10"
              : isError
              ? "border-rose-500/50 bg-rose-950/10"
              : "border-zinc-800 bg-zinc-900/30 hover:border-zinc-700"
          }`}
        >
          <button
            onClick={isConnected ? onDisconnect : onConnect}
            disabled={isTransitioning}
            className={`w-full h-full rounded-full flex flex-col items-center justify-center gap-1 text-white font-semibold transition-all duration-300 transform active:scale-95 shadow-xl ${
              isConnected
                ? "bg-gradient-to-b from-emerald-600 to-emerald-700 hover:from-emerald-500 hover:to-emerald-600 shadow-emerald-900/40"
                : isTransitioning
                ? "bg-zinc-800 text-zinc-300 cursor-wait"
                : isError
                ? "bg-gradient-to-b from-rose-600 to-rose-700 hover:from-rose-500 hover:to-rose-600 shadow-rose-900/40"
                : "bg-gradient-to-b from-brand-600 to-brand-700 hover:from-brand-500 hover:to-brand-600 shadow-brand-900/30 hover:shadow-brand-500/20"
            }`}
          >
            {isTransitioning ? (
              <Loader2 className="w-8 h-8 animate-spin text-brand-300" />
            ) : (
              <Power className="w-8 h-8 stroke-[2.2]" />
            )}
            <span className="text-[11px] tracking-wider font-bold uppercase mt-0.5">
              {getButtonText()}
            </span>
          </button>
        </div>
      </div>

      <div className="text-center space-y-1.5 max-w-md">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-zinc-900/80 border border-zinc-800 text-xs">
          <span
            className={`w-2 h-2 rounded-full ${
              isConnected
                ? "bg-emerald-400 shadow-[0_0_8px_#34d399]"
                : isTransitioning
                ? "bg-amber-400 animate-ping"
                : isError
                ? "bg-rose-400"
                : "bg-zinc-600"
            }`}
          />
          <span className="font-medium text-zinc-200">
            {isConnected
              ? "Securely Connected"
              : isTransitioning
              ? "Connecting..."
              : isError
              ? "Connection Error"
              : "Disconnected"}
          </span>
        </div>

        <p className="text-xs text-zinc-400 font-normal">{getSubStatusText()}</p>

        {isConnected && health?.cloudflareTrace && (
          <div className="flex items-center justify-center gap-3 pt-1 text-[11px] text-zinc-400">
            <span className="flex items-center gap-1 text-zinc-300">
              <Globe className="w-3 h-3 text-brand-400" />
              <span>IP: {health.cloudflareTrace.ip}</span>
            </span>
            <span className="text-zinc-600">•</span>
            <span className="flex items-center gap-1 text-emerald-400 font-mono">
              <CheckCircle2 className="w-3 h-3" />
              <span>POP: {health.cloudflareTrace.colo}</span>
            </span>
          </div>
        )}

        {isError && (
          <div className="pt-2 flex items-center justify-center gap-2">
            <button
              onClick={onConnect}
              className="px-3 py-1 rounded bg-rose-600/20 text-rose-300 hover:bg-rose-600/30 text-xs font-medium border border-rose-500/30 transition-colors"
            >
              Retry
            </button>
            {onViewDiagnostics && (
              <button
                onClick={onViewDiagnostics}
                className="px-3 py-1 rounded bg-zinc-800 text-zinc-300 hover:bg-zinc-700 text-xs font-medium transition-colors"
              >
                View Diagnostics
              </button>
            )}
          </div>
        )}
      </div>
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

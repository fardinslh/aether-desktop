import React from "react";
import { Globe, Radio, Shield, Network } from "lucide-react";
import { HealthStatus, ConnectionState } from "../../types";

interface StatusOverviewProps {
  health: HealthStatus | null;
  connectionState: ConnectionState;
}

export const StatusOverview: React.FC<StatusOverviewProps> = ({ health, connectionState }) => {
  const isConnected = connectionState === "CONNECTED";

  const telemetryItems = [
    {
      id: "egress",
      label: "EGRESS TRANSPORT",
      icon: Globe,
      statusText: isConnected ? "TUNNEL EGRESS" : "DIRECT/LOCAL",
      address: isConnected && health?.cloudflareTrace ? `${health.cloudflareTrace.ip}` : "Standard Gateway",
      meta: isConnected && health?.cloudflareTrace ? `${health.cloudflareTrace.colo} · ${health.cloudflareTrace.latencyMs ?? "—"}ms` : "ISP Default",
      isActive: isConnected,
      color: "signal-green",
    },
    {
      id: "aether",
      label: "AETHER DAEMON",
      icon: Radio,
      statusText: isConnected ? "MANAGED SOCKS5" : "STANDBY",
      address: "127.0.0.1:1819",
      meta: isConnected ? "WireGuard Engine" : "Ready to spawn",
      isActive: isConnected,
      color: "signal-green",
    },
    {
      id: "singbox",
      label: "ROUTER & TUN",
      icon: Shield,
      statusText: isConnected ? "WINTUN STACK" : "INACTIVE",
      address: "singbox-tun",
      meta: "172.19.0.1/30 (Strict)",
      isActive: isConnected,
      color: "signal-cyan",
    },
    {
      id: "secondary",
      label: "SECONDARY PROXY",
      icon: Network,
      statusText: "V2RAY SOCKS",
      address: "127.0.0.1:10808",
      meta: health?.secondaryProxy.ok ? "Ready & Bound" : "Standby",
      isActive: health?.secondaryProxy.ok ?? false,
      color: "signal-amber",
    },
  ];

  return (
    <div className="px-4 mt-2 select-none">
      <div className="w-full bg-app-panel border border-app-border rounded-md p-2.5">
        <div className="flex items-center justify-between px-1 mb-2">
          <div className="text-[10px] font-mono font-semibold tracking-wider text-ink-400 uppercase flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-signal-cyan" />
            <span>SUBSYSTEM TELEMETRY & ROUTING RACK</span>
          </div>
          <span className="text-[10px] font-mono text-ink-500">POLL: 800ms</span>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
          {telemetryItems.map((item) => {
            const Icon = item.icon;
            return (
              <div
                key={item.id}
                className="bg-app-inset border border-app-border-subtle rounded-sm p-2 flex flex-col justify-between hover:border-app-border transition-colors"
              >
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[10px] font-mono font-semibold text-ink-400 flex items-center gap-1">
                    <Icon className="w-3 h-3 text-ink-400" />
                    <span>{item.label}</span>
                  </span>
                  <span className="flex items-center gap-1">
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        item.isActive ? "bg-signal-green shadow-[0_0_4px_#10b981]" : "bg-ink-500"
                      }`}
                    />
                    <span className="text-[9px] font-mono uppercase text-ink-400">
                      {item.isActive ? "OK" : "IDLE"}
                    </span>
                  </span>
                </div>

                <div className="text-xs font-mono font-semibold text-ink-200 truncate">
                  {item.address}
                </div>

                <div className="text-[10px] font-mono text-ink-400 truncate mt-0.5">
                  {item.meta}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

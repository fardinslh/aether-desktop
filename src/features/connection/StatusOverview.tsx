import React from "react";
import { Globe, Radio, Shield, Network, CheckCircle2, AlertCircle } from "lucide-react";
import { HealthStatus, ConnectionState } from "../../types";

interface StatusOverviewProps {
  health: HealthStatus | null;
  connectionState: ConnectionState;
}

export const StatusOverview: React.FC<StatusOverviewProps> = ({ health, connectionState }) => {
  const isConnected = connectionState === "CONNECTED";

  const cards = [
    {
      id: "internet",
      title: "Internet",
      icon: Globe,
      status: isConnected ? "Active" : "Normal",
      ok: isConnected ? health?.internet.ok : true,
      subtitle: isConnected ? "Through sing-box TUN" : "Direct network",
    },
    {
      id: "aether",
      title: "Aether Tunnel",
      icon: Radio,
      status: isConnected ? "Running" : "Standby",
      ok: isConnected ? health?.aetherTunnel.ok : true,
      subtitle: isConnected ? "SOCKS5 127.0.0.1:1819" : "Ready to launch",
    },
    {
      id: "routing",
      title: "System Routing",
      icon: Shield,
      status: isConnected ? "Active" : "Disabled",
      ok: isConnected ? health?.routing.ok : true,
      subtitle: isConnected ? "singbox-tun adapter" : "Standard table",
    },
    {
      id: "secondary",
      title: "Secondary Proxy",
      icon: Network,
      status: isConnected ? "Active" : "Standby",
      ok: isConnected ? health?.secondaryProxy.ok : true,
      subtitle: "127.0.0.1:10808",
    },
  ];

  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5 px-4">
      {cards.map((c) => {
        const Icon = c.icon;
        const isOk = c.ok;

        return (
          <div
            key={c.id}
            className="p-2.5 rounded-xl bg-background-card border border-zinc-800/80 hover:border-zinc-700/80 transition-colors"
          >
            <div className="flex items-center justify-between mb-1.5">
              <div className="p-1.5 rounded-lg bg-zinc-900 border border-zinc-800/80 text-zinc-400">
                <Icon className="w-3.5 h-3.5 text-brand-400" />
              </div>
              {isOk ? (
                <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              ) : (
                <AlertCircle className="w-3.5 h-3.5 text-amber-400" />
              )}
            </div>
            <div className="text-xs font-semibold text-zinc-200">{c.title}</div>
            <div className="text-[11px] text-zinc-400 flex items-center gap-1 mt-0.5">
              <span className={`w-1.5 h-1.5 rounded-full ${isOk && isConnected ? "bg-emerald-400" : "bg-zinc-500"}`} />
              <span className="truncate">{c.subtitle}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
};

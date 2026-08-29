import React, { useState } from "react";
import {
  Sliders,
  Radio,
  Network,
  Shield,
  RotateCcw,
  Save,
  CheckCircle2,
  HelpCircle,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { AppSettings, CloudflareTrace } from "../../types";
import { api } from "../../services/api";

interface SettingsViewProps {
  settings: AppSettings;
  onSave: (settings: AppSettings) => void;
  onReset: () => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({ settings, onSave, onReset }) => {
  const [localSettings, setLocalSettings] = useState<AppSettings>(settings);
  const [activeTab, setActiveTab] = useState<
    "general" | "aether" | "secondary" | "singbox" | "compatibility"
  >("general");
  const [testTrace, setTestTrace] = useState<CloudflareTrace | null>(null);
  const [testLoading, setTestLoading] = useState<boolean>(false);
  const [testError, setTestError] = useState<string | null>(null);
  const [saveSuccess, setSaveSuccess] = useState<boolean>(false);
  const [showAdvancedAether, setShowAdvancedAether] = useState<boolean>(false);

  const handleTestSecondaryProxy = async () => {
    setTestLoading(true);
    setTestError(null);
    setTestTrace(null);
    try {
      const trace = await api.testSecondaryProxy();
      setTestTrace(trace);
    } catch (err: any) {
      setTestError(err.toString());
    } finally {
      setTestLoading(false);
    }
  };

  const handleSave = () => {
    onSave(localSettings);
    setSaveSuccess(true);
    setTimeout(() => setSaveSuccess(false), 2500);
  };

  return (
    <div className="flex flex-col h-full px-4 py-2.5 space-y-2.5 select-none">
      {/* Settings Top Bar */}
      <div className="flex items-center justify-between bg-app-panel border border-app-border rounded-md p-3">
        <div>
          <h2 className="text-xs font-bold tracking-wider uppercase text-ink-100 font-mono">
            ENGINE CONFIGURATION & SUBSYSTEMS
          </h2>
          <p className="text-[11px] text-ink-400 font-sans mt-0.5">
            Configure Aether, Secondary SOCKS5, sing-box Wintun driver, and compatibility layer parameters.
          </p>
        </div>

        <div className="flex items-center gap-2 font-mono">
          <button
            onClick={onReset}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-sm border border-app-border bg-app-surface hover:bg-app-elevated text-ink-300 text-xs transition-colors cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset</span>
          </button>
          <button
            onClick={handleSave}
            className="flex items-center gap-1.5 px-4 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black text-xs font-bold transition-all shadow-sm cursor-pointer"
          >
            {saveSuccess ? <CheckCircle2 className="w-3.5 h-3.5" /> : <Save className="w-3.5 h-3.5" />}
            <span>{saveSuccess ? "APPLIED" : "APPLY SETTINGS"}</span>
          </button>
        </div>
      </div>

      {/* Configuration Subsystem Tabs */}
      <div className="flex border-b border-app-border gap-1 font-mono text-xs">
        {[
          { id: "general", label: "GENERAL", icon: Sliders },
          { id: "aether", label: "AETHER DAEMON", icon: Radio },
          { id: "secondary", label: "SECONDARY SOCKS", icon: Network },
          { id: "singbox", label: "SING-BOX TUN", icon: Shield },
          { id: "compatibility", label: "COMPATIBILITY", icon: HelpCircle },
        ].map((t) => {
          const Icon = t.icon;
          const isActive = activeTab === t.id;
          return (
            <button
              key={t.id}
              onClick={() => setActiveTab(t.id as any)}
              className={`flex items-center gap-1.5 px-3 pb-2 border-b-2 transition-all cursor-pointer text-[11px] ${
                isActive
                  ? "border-signal-cyan text-signal-cyan font-bold"
                  : "border-transparent text-ink-400 hover:text-ink-200"
              }`}
            >
              <Icon className="w-3.5 h-3.5" />
              <span>{t.label}</span>
            </button>
          );
        })}
      </div>

      <div className="flex-1 overflow-y-auto rounded-md border border-app-border bg-app-panel p-4 space-y-3.5 max-h-[380px]">
        {activeTab === "general" && (
          <div className="space-y-2.5 font-sans">
            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div>
                <div className="text-xs font-semibold text-ink-100">Start with Windows</div>
                <div className="text-[10px] text-ink-400">Launch Aether Desktop automatically on system login</div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.general.startWithWindows}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    general: { ...localSettings.general, startWithWindows: e.target.checked },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div>
                <div className="text-xs font-semibold text-ink-100">Auto Connect</div>
                <div className="text-[10px] text-ink-400">Automatically establish routing tunnel upon application launch</div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.general.autoConnect}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    general: { ...localSettings.general, autoConnect: e.target.checked },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div>
                <div className="text-xs font-semibold text-ink-100">Minimize to System Tray</div>
                <div className="text-[10px] text-ink-400">Keep running in the background when the main window is closed</div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.general.minimizeToTray}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    general: { ...localSettings.general, minimizeToTray: e.target.checked },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div>
                <div className="text-xs font-semibold text-ink-100">Automatic Reconnection</div>
                <div className="text-[10px] text-ink-400">Silently reconnect if network sleep or interface reset occurs</div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.general.reconnectAutomatically}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    general: { ...localSettings.general, reconnectAutomatically: e.target.checked },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer"
              />
            </div>
          </div>
        )}

        {activeTab === "aether" && (
          <div className="space-y-3.5 font-sans">
            <div>
              <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                Aether Binary Location
              </label>
              <input
                type="text"
                value={localSettings.aether.executablePath}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    aether: { ...localSettings.aether, executablePath: e.target.value },
                  })
                }
                className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  Protocol Profile
                </label>
                <select
                  value={localSettings.aether.protocol || "wireguard"}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, protocol: e.target.value as any },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                >
                  <option value="wireguard">WireGuard (--wg) [Recommended]</option>
                  <option value="masque">MASQUE (--masque)</option>
                  <option value="warp_in_warp">WARP-in-WARP / Gool (--gool)</option>
                </select>
              </div>

              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  IP Routing Mode
                </label>
                <select
                  value={localSettings.aether.ipMode || "ipv4"}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, ipMode: e.target.value as any },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                >
                  <option value="ipv4">IPv4 (-4) [Recommended]</option>
                  <option value="ipv6">IPv6 (-6)</option>
                  <option value="dual">Dual Stack (--dual)</option>
                </select>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  Endpoint Scan Mode
                </label>
                <select
                  value={localSettings.aether.scanMode || "thorough"}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, scanMode: e.target.value as any },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                >
                  <option value="thorough">Thorough (--thorough) [Recommended]</option>
                  <option value="balanced">Balanced (--balanced)</option>
                  <option value="turbo">Turbo (--turbo)</option>
                  <option value="stealth">Stealth (--stealth)</option>
                  <option value="ironclad">Ironclad (--ironclad)</option>
                </select>
              </div>

              <div className="flex items-center justify-between p-2 rounded-sm bg-app-surface border border-app-border self-end">
                <div>
                  <div className="text-xs font-semibold text-ink-100 font-mono">QUICK RECONNECT</div>
                  <div className="text-[10px] text-ink-400 font-sans">Fast tunnel resumption (--quick-reconnect)</div>
                </div>
                <input
                  type="checkbox"
                  checked={localSettings.aether.quickReconnect ?? true}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, quickReconnect: e.target.checked },
                    })
                  }
                  className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer"
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  SOCKS5 Host
                </label>
                <input
                  type="text"
                  value={localSettings.aether.host}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, host: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  SOCKS5 Port
                </label>
                <input
                  type="number"
                  value={localSettings.aether.port}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, port: Number(e.target.value) },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
            </div>

            {/* Advanced developer parameters */}
            <div className="border border-app-border-subtle rounded-sm p-3 bg-app-inset font-mono">
              <button
                onClick={() => setShowAdvancedAether(!showAdvancedAether)}
                className="flex items-center justify-between w-full text-xs font-semibold text-ink-400 hover:text-ink-200 cursor-pointer"
              >
                <span>ADVANCED CLI FLAGS</span>
                {showAdvancedAether ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
              </button>
              {showAdvancedAether && (
                <div className="mt-2.5 pt-2.5 border-t border-app-border-subtle space-y-2">
                  <label className="block text-[10px] text-ink-400">
                    Additional CLI Arguments (space-separated)
                  </label>
                  <input
                    type="text"
                    value={(localSettings.aether.additionalArguments || []).join(" ")}
                    onChange={(e) =>
                      setLocalSettings({
                        ...localSettings,
                        aether: {
                          ...localSettings.aether,
                          additionalArguments: e.target.value.split(" ").filter((s) => s.trim().length > 0),
                        },
                      })
                    }
                    placeholder="e.g. --verbose"
                    className="w-full px-3 py-1.5 bg-app-panel border border-app-border rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                  />
                </div>
              )}
            </div>
          </div>
        )}

        {activeTab === "secondary" && (
          <div className="space-y-3.5 font-sans">
            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div>
                <div className="text-xs font-semibold text-ink-100 font-mono">ENABLE SECONDARY PROXY ROUTE</div>
                <div className="text-[10px] text-ink-400">Routes selected AI & development applications through v2rayN/Xray</div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.secondaryProxy.enabled}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    secondaryProxy: { ...localSettings.secondaryProxy, enabled: e.target.checked },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  Secondary SOCKS Host
                </label>
                <input
                  type="text"
                  value={localSettings.secondaryProxy.host}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      secondaryProxy: { ...localSettings.secondaryProxy, host: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  Secondary SOCKS Port
                </label>
                <input
                  type="number"
                  value={localSettings.secondaryProxy.port}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      secondaryProxy: { ...localSettings.secondaryProxy, port: Number(e.target.value) },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
            </div>

            <div className="p-3 rounded-sm border border-app-border bg-app-surface space-y-2 font-mono">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-ink-200">PROXY REACHABILITY PROBE</span>
                <button
                  onClick={handleTestSecondaryProxy}
                  disabled={testLoading}
                  className="px-3 py-1 bg-app-panel hover:bg-app-elevated disabled:opacity-40 text-ink-200 text-xs border border-app-border rounded-sm transition-colors cursor-pointer"
                >
                  {testLoading ? "PROBING..." : "PROBE PORT 10808"}
                </button>
              </div>

              {testTrace && (
                <div className="text-xs text-signal-green bg-signal-green-dim border border-signal-green/30 p-2 rounded-sm space-y-0.5">
                  <div className="font-semibold">✓ SECONDARY PROXY OPERATIONAL</div>
                  <div className="text-[10px] text-ink-300">
                    EGRESS: {testTrace.ip} · POP: {testTrace.colo} ({testTrace.loc}) · LATENCY: {testTrace.latencyMs} ms
                  </div>
                </div>
              )}

              {testError && (
                <div className="text-xs text-signal-red bg-signal-red-dim border border-signal-red/30 p-2 rounded-sm">
                  <div className="font-semibold">✕ PROXY UNREACHABLE</div>
                  <div className="text-[10px] text-ink-300">{testError}</div>
                </div>
              )}
            </div>
          </div>
        )}

        {activeTab === "singbox" && (
          <div className="space-y-3.5 font-sans">
            <div>
              <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                sing-box Binary Location
              </label>
              <input
                type="text"
                value={localSettings.singBox.executablePath}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    singBox: { ...localSettings.singBox, executablePath: e.target.value },
                  })
                }
                className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  TUN Adapter Interface Name
                </label>
                <input
                  type="text"
                  value={localSettings.singBox.interfaceName}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, interfaceName: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  TUN MTU
                </label>
                <input
                  type="number"
                  value={localSettings.singBox.mtu}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, mtu: Number(e.target.value) },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  TUN IPv4 Subnet
                </label>
                <input
                  type="text"
                  value={localSettings.singBox.tunAddress}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, tunAddress: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                />
              </div>
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  sing-box Log Verbosity
                </label>
                <select
                  value={localSettings.singBox.logLevel}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, logLevel: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                >
                  <option value="trace">trace</option>
                  <option value="debug">debug</option>
                  <option value="info">info</option>
                  <option value="warn">warn</option>
                  <option value="error">error</option>
                </select>
              </div>
            </div>
          </div>
        )}

        {activeTab === "compatibility" && (
          <div className="space-y-2.5 font-sans">
            <div className="p-2.5 rounded-sm bg-app-inset border border-app-border-subtle mb-2 font-mono">
              <span className="text-[10px] font-bold uppercase tracking-wider text-signal-cyan">
                GLOBAL COMPATIBILITY & BYPASS POLICIES
              </span>
              <p className="text-[10px] text-ink-400 font-sans mt-0.5">
                Generic compatibility rules operate as fallbacks and will never override explicit application rules.
              </p>
            </div>

            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div className="pr-4">
                <div className="text-xs font-semibold text-ink-100">
                  Generals Online STUN/TURN Compatibility Fallback (Ports 3478, 5349)
                </div>
                <div className="text-[10px] text-ink-400 mt-0.5 leading-relaxed">
                  Routes unassigned STUN/TURN traffic directly for legacy games like Generals Online. Explicitly assigned applications (like Discord) will follow their selected route.
                </div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.compatibility.generalsStunTurnFallback}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    compatibility: {
                      ...localSettings.compatibility,
                      generalsStunTurnFallback: e.target.checked,
                    },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer flex-shrink-0"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-sm bg-app-surface border border-app-border">
              <div className="pr-4">
                <div className="text-xs font-semibold text-ink-100">Local Area Network (LAN) Bypass</div>
                <div className="text-[10px] text-ink-400 mt-0.5 leading-relaxed">
                  Bypasses private IP subnets (192.168.x.x, 10.x.x.x) so routers, NAS, and local devices remain accessible.
                </div>
              </div>
              <input
                type="checkbox"
                checked={localSettings.compatibility.privateIpBypass}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    compatibility: {
                      ...localSettings.compatibility,
                      privateIpBypass: e.target.checked,
                    },
                  })
                }
                className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border cursor-pointer flex-shrink-0"
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
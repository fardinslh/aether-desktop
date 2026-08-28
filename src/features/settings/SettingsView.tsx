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
    <div className="flex flex-col h-full px-4 py-3 space-y-3">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-zinc-100">Application Settings</h2>
          <p className="text-xs text-zinc-400">
            Configure Aether, Secondary Proxy, sing-box TUN, and compatibility fallback behavior.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={onReset}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 text-xs font-medium transition-colors"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset Defaults</span>
          </button>
          <button
            onClick={handleSave}
            className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white text-xs font-semibold shadow-md shadow-brand-900/30 transition-all active:scale-95"
          >
            {saveSuccess ? <CheckCircle2 className="w-3.5 h-3.5 text-emerald-300" /> : <Save className="w-3.5 h-3.5" />}
            <span>{saveSuccess ? "Saved!" : "Save Changes"}</span>
          </button>
        </div>
      </div>

      <div className="flex border-b border-zinc-800 gap-2 text-xs font-medium">
        {[
          { id: "general", label: "General", icon: Sliders },
          { id: "aether", label: "Aether Primary", icon: Radio },
          { id: "secondary", label: "Secondary SOCKS", icon: Network },
          { id: "singbox", label: "sing-box TUN", icon: Shield },
          { id: "compatibility", label: "Compatibility", icon: HelpCircle },
        ].map((t) => {
          const Icon = t.icon;
          const isActive = activeTab === t.id;
          return (
            <button
              key={t.id}
              onClick={() => setActiveTab(t.id as any)}
              className={`flex items-center gap-1.5 pb-2.5 border-b-2 transition-all ${
                isActive
                  ? "border-brand-500 text-brand-400 font-semibold"
                  : "border-transparent text-zinc-400 hover:text-zinc-300"
              }`}
            >
              <Icon className="w-3.5 h-3.5" />
              <span>{t.label}</span>
            </button>
          );
        })}
      </div>

      <div className="flex-1 overflow-y-auto rounded-xl border border-zinc-800/80 bg-background-card p-4 space-y-4 max-h-[380px]">
        {activeTab === "general" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div>
                <div className="text-xs font-semibold text-zinc-200">Start with Windows</div>
                <div className="text-[11px] text-zinc-400">Launch Aether Desktop automatically on system login</div>
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div>
                <div className="text-xs font-semibold text-zinc-200">Auto Connect</div>
                <div className="text-[11px] text-zinc-400">Automatically establish routing tunnel upon application launch</div>
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div>
                <div className="text-xs font-semibold text-zinc-200">Minimize to System Tray</div>
                <div className="text-[11px] text-zinc-400">Keep running in the background when the main window is closed</div>
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div>
                <div className="text-xs font-semibold text-zinc-200">Automatic Reconnection</div>
                <div className="text-[11px] text-zinc-400">Silently reconnect if network sleep or interface reset occurs</div>
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer"
              />
            </div>
          </div>
        )}

        {activeTab === "aether" && (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-zinc-300 mb-1">Aether Executable Path</label>
              <input
                type="text"
                value={localSettings.aether.executablePath}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    aether: { ...localSettings.aether, executablePath: e.target.value },
                  })
                }
                className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
              />
              <p className="text-[11px] text-zinc-500 mt-1">Default development path: C:\Aether\aether.exe</p>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">SOCKS5 Host</label>
                <input
                  type="text"
                  value={localSettings.aether.host}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, host: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">SOCKS5 Port</label>
                <input
                  type="number"
                  value={localSettings.aether.port}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      aether: { ...localSettings.aether, port: Number(e.target.value) },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
            </div>
          </div>
        )}

        {activeTab === "secondary" && (
          <div className="space-y-4">
            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div>
                <div className="text-xs font-semibold text-zinc-200">Enable Secondary SOCKS5 Proxy</div>
                <div className="text-[11px] text-zinc-400">Routes selected AI & development applications through v2rayN/Xray</div>
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">Secondary Proxy Host</label>
                <input
                  type="text"
                  value={localSettings.secondaryProxy.host}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      secondaryProxy: { ...localSettings.secondaryProxy, host: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">Secondary Proxy Port</label>
                <input
                  type="number"
                  value={localSettings.secondaryProxy.port}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      secondaryProxy: { ...localSettings.secondaryProxy, port: Number(e.target.value) },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
            </div>

            <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-950/60 space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-zinc-300">Proxy Health Diagnostics</span>
                <button
                  onClick={handleTestSecondaryProxy}
                  disabled={testLoading}
                  className="px-3 py-1 bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 text-zinc-200 text-xs font-medium rounded-md transition-colors"
                >
                  {testLoading ? "Testing..." : "Test Connection"}
                </button>
              </div>

              {testTrace && (
                <div className="text-xs text-emerald-400 bg-emerald-950/20 border border-emerald-500/20 p-2.5 rounded-md space-y-0.5">
                  <div className="font-semibold">✓ SOCKS5 Proxy Operational</div>
                  <div className="text-[11px] text-zinc-400">
                    Public IP: {testTrace.ip} · POP: {testTrace.colo} ({testTrace.loc}) · Latency: {testTrace.latencyMs} ms
                  </div>
                </div>
              )}

              {testError && (
                <div className="text-xs text-rose-400 bg-rose-950/20 border border-rose-500/20 p-2.5 rounded-md">
                  <div className="font-semibold">✕ Proxy Unreachable</div>
                  <div className="text-[11px] text-zinc-400">{testError}</div>
                </div>
              )}
            </div>
          </div>
        )}

        {activeTab === "singbox" && (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-zinc-300 mb-1">sing-box Executable Path</label>
              <input
                type="text"
                value={localSettings.singBox.executablePath}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    singBox: { ...localSettings.singBox, executablePath: e.target.value },
                  })
                }
                className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">TUN Interface Name</label>
                <input
                  type="text"
                  value={localSettings.singBox.interfaceName}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, interfaceName: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">MTU</label>
                <input
                  type="number"
                  value={localSettings.singBox.mtu}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, mtu: Number(e.target.value) },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">TUN Subnet Address</label>
                <input
                  type="text"
                  value={localSettings.singBox.tunAddress}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, tunAddress: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">Log Level</label>
                <select
                  value={localSettings.singBox.logLevel}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      singBox: { ...localSettings.singBox, logLevel: e.target.value },
                    })
                  }
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
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
          <div className="space-y-3">
            <div className="p-3 rounded-lg bg-zinc-950/60 border border-zinc-800/80 mb-2">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-brand-400">
                Fallback & Legacy Routing Rules
              </span>
              <p className="text-[11px] text-zinc-400 mt-0.5">
                Generic compatibility rules operate as fallbacks and will never override explicit application rules.
              </p>
            </div>

            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div className="pr-4">
                <div className="text-xs font-semibold text-zinc-200">
                  Generals Online STUN/TURN Compatibility Fallback (Ports 3478, 5349)
                </div>
                <div className="text-[11px] text-zinc-400 mt-0.5 leading-relaxed">
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer flex-shrink-0"
              />
            </div>

            <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
              <div className="pr-4">
                <div className="text-xs font-semibold text-zinc-200">Local Area Network (LAN) Bypass</div>
                <div className="text-[11px] text-zinc-400 mt-0.5 leading-relaxed">
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
                className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer flex-shrink-0"
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
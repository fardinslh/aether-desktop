import React, { useState, useEffect } from "react";
import {
  X,
  Search,
  FolderOpen,
  Activity,
  ShieldCheck,
  Server,
  Zap,
  AlertCircle,
  CheckCircle2,
  Edit2,
} from "lucide-react";
import { ApplicationRule, RouteDestination, RunningProcessInfo } from "../../types";
import { api } from "../../services/api";
import { AppIcon } from "../../components/AppIcon";

interface AddApplicationModalProps {
  isOpen: boolean;
  existingRules: ApplicationRule[];
  onClose: () => void;
  onAddRule: (rule: ApplicationRule) => void;
  onEditExisting?: (rule: ApplicationRule) => void;
}

export const AddApplicationModal: React.FC<AddApplicationModalProps> = ({
  isOpen,
  existingRules,
  onClose,
  onAddRule,
  onEditExisting,
}) => {
  const [tab, setTab] = useState<"running" | "browse">("running");
  const [runningApps, setRunningApps] = useState<RunningProcessInfo[]>([]);
  const [runningFilter, setRunningFilter] = useState<string>("");
  const [loadingApps, setLoadingApps] = useState<boolean>(false);

  // Selected or entered app info
  const [displayName, setDisplayName] = useState<string>("");
  const [processName, setProcessName] = useState<string>("");
  const [executablePath, setExecutablePath] = useState<string>("");
  const [iconBase64, setIconBase64] = useState<string | null>(null);
  const [destination, setDestination] = useState<RouteDestination>("secondaryProxy");

  // Duplicate state
  const [duplicateRule, setDuplicateRule] = useState<ApplicationRule | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadRunningApps();
      resetForm();
    }
  }, [isOpen]);

  const loadRunningApps = async () => {
    setLoadingApps(true);
    try {
      const apps = await api.getRunningApplications();
      setRunningApps(apps);
    } catch (e) {
      console.error("Failed to load running applications:", e);
    } finally {
      setLoadingApps(false);
    }
  };

  const resetForm = () => {
    setDisplayName("");
    setProcessName("");
    setExecutablePath("");
    setIconBase64(null);
    setDestination("secondaryProxy");
    setDuplicateRule(null);
  };

  // Check for duplicates whenever processName changes
  useEffect(() => {
    if (!processName.trim()) {
      setDuplicateRule(null);
      return;
    }
    const cleanProc = processName.trim().toLowerCase();
    const existing = existingRules.find(
      (r) => r.processName.toLowerCase() === cleanProc || r.processName.toLowerCase() === `${cleanProc}.exe`
    );
    setDuplicateRule(existing || null);
  }, [processName, existingRules]);

  const handleSelectRunningApp = (app: RunningProcessInfo) => {
    setDisplayName(app.name);
    setProcessName(app.processName);
    setExecutablePath(app.executablePath || "");
    setIconBase64(app.iconBase64 || null);
  };

  const handleNativeBrowse = async () => {
    try {
      const selected = await api.pickExecutableFile();
      if (selected) {
        setExecutablePath(selected);
        const metadata = await api.inspectExecutable(selected);
        setDisplayName(metadata.displayName);
        setProcessName(metadata.processName);
        if (metadata.iconBase64) {
          setIconBase64(metadata.iconBase64);
        }
      }
    } catch (e) {
      console.error("Native file dialog error:", e);
    }
  };

  const handleBrowsePathChange = async (path: string) => {
    setExecutablePath(path);
    if (path.trim() && (path.includes("\\") || path.includes("/"))) {
      try {
        const metadata = await api.inspectExecutable(path);
        setDisplayName(metadata.displayName);
        setProcessName(metadata.processName);
        if (metadata.iconBase64) {
          setIconBase64(metadata.iconBase64);
        }
      } catch {
        // manual typing fallback
      }
    } else {
      setProcessName(path.trim());
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!processName.trim() || duplicateRule) return;

    const normalizedProc = processName.toLowerCase().endsWith(".exe")
      ? processName
      : `${processName}.exe`;

    const newRule: ApplicationRule = {
      id: crypto.randomUUID ? crypto.randomUUID() : `rule-${Date.now()}`,
      displayName: displayName.trim() || normalizedProc.replace(/\.exe$/i, ""),
      processName: normalizedProc,
      executablePath: executablePath.trim() || null,
      destination,
      enabled: true,
      source: "user",
      priority: "normal",
      iconBase64: iconBase64 || null,
    };

    onAddRule(newRule);
    onClose();
  };

  if (!isOpen) return null;

  const filteredRunning = runningApps.filter((a) =>
    a.name.toLowerCase().includes(runningFilter.toLowerCase()) ||
    a.processName.toLowerCase().includes(runningFilter.toLowerCase())
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xs p-4 select-none">
      <div className="bg-app-panel border border-app-border rounded-md w-full max-w-lg shadow-xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-app-border bg-app-surface">
          <div>
            <h3 className="text-xs font-bold font-mono tracking-wider uppercase text-ink-100">
              ADD APPLICATION ROUTING RULE
            </h3>
            <p className="text-[11px] text-ink-400">
              Define per-process destination routing in the system stack.
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-sm text-ink-400 hover:text-ink-100 hover:bg-app-panel transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Method Switcher Tabs */}
        <div className="flex border-b border-app-border px-4 pt-2.5 gap-2 text-xs font-mono">
          <button
            type="button"
            onClick={() => setTab("running")}
            className={`flex items-center gap-1.5 pb-2 border-b-2 transition-all cursor-pointer ${
              tab === "running"
                ? "border-signal-cyan text-signal-cyan font-semibold"
                : "border-transparent text-ink-400 hover:text-ink-200"
            }`}
          >
            <Activity className="w-3.5 h-3.5" />
            <span>RUNNING PROCESSES</span>
          </button>
          <button
            type="button"
            onClick={() => setTab("browse")}
            className={`flex items-center gap-1.5 pb-2 border-b-2 transition-all cursor-pointer ${
              tab === "browse"
                ? "border-signal-cyan text-signal-cyan font-semibold"
                : "border-transparent text-ink-400 hover:text-ink-200"
            }`}
          >
            <FolderOpen className="w-3.5 h-3.5" />
            <span>BROWSE EXECUTABLE</span>
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-4 space-y-3.5">
          {/* Method 1: Running Process Selection */}
          {tab === "running" && (
            <div className="space-y-2">
              <div className="relative">
                <Search className="w-3.5 h-3.5 absolute left-3 top-2.5 text-ink-400" />
                <input
                  type="text"
                  placeholder="Filter active process list..."
                  value={runningFilter}
                  onChange={(e) => setRunningFilter(e.target.value)}
                  className="w-full pl-8 pr-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs font-mono text-ink-200 placeholder-ink-500 focus:outline-none focus:border-signal-cyan"
                />
              </div>

              <div className="border border-app-border rounded-sm bg-app-inset max-h-48 overflow-y-auto divide-y divide-app-border-subtle">
                {loadingApps ? (
                  <div className="p-6 text-center text-xs font-mono text-ink-400">Detecting running applications...</div>
                ) : filteredRunning.length === 0 ? (
                  <div className="p-6 text-center text-xs font-mono text-ink-400">No applications matched. Switch to "BROWSE EXECUTABLE" above.</div>
                ) : (
                  filteredRunning.map((app) => {
                    const isSelected = processName.toLowerCase() === app.processName.toLowerCase();
                    return (
                      <button
                        key={`${app.pid}-${app.processName}`}
                        type="button"
                        onClick={() => handleSelectRunningApp(app)}
                        className={`w-full flex items-center justify-between p-2 text-left transition-colors cursor-pointer ${
                          isSelected
                            ? "bg-signal-cyan-dim border-l-2 border-signal-cyan text-ink-100"
                            : "hover:bg-app-surface/60 text-ink-300"
                        }`}
                      >
                        <div className="flex items-center gap-2">
                          <AppIcon processName={app.processName} displayName={app.name} iconBase64={app.iconBase64} size="sm" />
                          <div>
                            <div className="text-xs font-semibold text-ink-100">{app.name}</div>
                            <div className="text-[10px] font-mono text-ink-400">{app.processName}</div>
                          </div>
                        </div>
                        {isSelected && <CheckCircle2 className="w-3.5 h-3.5 text-signal-cyan flex-shrink-0" />}
                      </button>
                    );
                  })
                )}
              </div>
            </div>
          )}

          {/* Method 2: Browse / Enter File */}
          {tab === "browse" && (
            <div className="space-y-3">
              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  Target Binary Path (.exe)
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="e.g. C:\Program Files\Spotify\Spotify.exe or Telegram.exe"
                    value={executablePath || processName}
                    onChange={(e) => handleBrowsePathChange(e.target.value)}
                    className="flex-1 px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 font-mono focus:outline-none focus:border-signal-cyan"
                  />
                  <button
                    type="button"
                    onClick={handleNativeBrowse}
                    className="px-3 py-1.5 bg-app-surface hover:bg-app-elevated border border-app-border text-ink-200 rounded-sm text-xs font-mono flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Browse...</span>
                  </button>
                </div>
              </div>

              <div>
                <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
                  Friendly Label
                </label>
                <input
                  type="text"
                  placeholder="e.g. Spotify Desktop"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs text-ink-200 focus:outline-none focus:border-signal-cyan"
                />
              </div>
            </div>
          )}

          {/* Duplicate Warning */}
          {duplicateRule && (
            <div className="p-2.5 rounded-sm bg-signal-amber-dim border border-signal-amber/30 flex items-start justify-between gap-2 text-xs text-signal-amber font-mono">
              <div className="flex items-start gap-2">
                <AlertCircle className="w-4 h-4 text-signal-amber flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-semibold">Application Rule Already Exists</div>
                  <div className="text-[10px] text-ink-300 mt-0.5">
                    <span className="font-semibold text-ink-100">{duplicateRule.displayName || duplicateRule.name}</span> ({duplicateRule.processName}) is already configured.
                  </div>
                </div>
              </div>
              {onEditExisting && (
                <button
                  type="button"
                  onClick={() => onEditExisting(duplicateRule)}
                  className="flex items-center gap-1 px-2 py-0.5 rounded-xs bg-signal-amber/20 hover:bg-signal-amber/30 text-signal-amber font-medium text-[10px] flex-shrink-0 cursor-pointer"
                >
                  <Edit2 className="w-3 h-3" />
                  <span>Edit Rule</span>
                </button>
              )}
            </div>
          )}

          {/* Selected Application Preview */}
          {processName && !duplicateRule && (
            <div className="flex items-center gap-2.5 p-2 rounded-sm bg-app-inset border border-app-border">
              <AppIcon processName={processName} displayName={displayName} iconBase64={iconBase64} size="sm" />
              <div>
                <div className="text-xs font-semibold text-ink-100">
                  {displayName || processName.replace(/\.exe$/i, "")}
                </div>
                <div className="text-[10px] font-mono text-ink-400">{processName}</div>
              </div>
            </div>
          )}

          {/* Routing Destination Selection */}
          <div className="space-y-1.5 pt-1">
            <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300">
              Steer Traffic Destination:
            </label>

            <div className="grid grid-cols-3 gap-2 font-mono">
              {/* Direct */}
              <button
                type="button"
                onClick={() => setDestination("direct")}
                className={`flex flex-col items-center p-2.5 rounded-sm border text-center transition-all cursor-pointer ${
                  destination === "direct"
                    ? "bg-signal-cyan-dim border-signal-cyan text-signal-cyan"
                    : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border"
                }`}
              >
                <Zap className="w-3.5 h-3.5 mb-1 text-signal-cyan" />
                <span className="text-[11px] font-bold">DIRECT INTERNET</span>
                <span className="text-[9px] text-ink-400 mt-0.5">Bypass TUN</span>
              </button>

              {/* Secondary Proxy */}
              <button
                type="button"
                onClick={() => setDestination("secondaryProxy")}
                className={`flex flex-col items-center p-2.5 rounded-sm border text-center transition-all cursor-pointer ${
                  destination === "secondaryProxy"
                    ? "bg-signal-amber-dim border-signal-amber text-signal-amber"
                    : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border"
                }`}
              >
                <Server className="w-3.5 h-3.5 mb-1 text-signal-amber" />
                <span className="text-[11px] font-bold">SECONDARY SOCKS</span>
                <span className="text-[9px] text-ink-400 mt-0.5">Port 10808</span>
              </button>

              {/* Aether */}
              <button
                type="button"
                onClick={() => setDestination("aether")}
                className={`flex flex-col items-center p-2.5 rounded-sm border text-center transition-all cursor-pointer ${
                  destination === "aether"
                    ? "bg-signal-green-dim border-signal-green text-signal-green"
                    : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border"
                }`}
              >
                <ShieldCheck className="w-3.5 h-3.5 mb-1 text-signal-green" />
                <span className="text-[11px] font-bold">AETHER TUNNEL</span>
                <span className="text-[9px] text-ink-400 mt-0.5">Global Gateway</span>
              </button>
            </div>
          </div>

          {/* Footer Buttons */}
          <div className="flex items-center justify-end gap-2 pt-3 border-t border-app-border font-mono">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded-sm border border-app-border text-ink-300 hover:bg-app-surface text-xs transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!processName.trim() || !!duplicateRule}
              className="px-4 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted disabled:opacity-30 disabled:pointer-events-none text-black font-bold text-xs transition-all shadow-sm cursor-pointer"
            >
              Save Route Rule
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
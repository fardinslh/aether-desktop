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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4 animate-fade-in">
      <div className="bg-background-card border border-zinc-800 rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-800">
          <div>
            <h3 className="text-sm font-semibold text-zinc-100">Add Application Routing Rule</h3>
            <p className="text-xs text-zinc-400">
              Select an application and configure its network path.
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Method Switcher Tabs */}
        <div className="flex border-b border-zinc-800 px-5 pt-3 gap-3 text-xs font-medium">
          <button
            type="button"
            onClick={() => setTab("running")}
            className={`flex items-center gap-1.5 pb-2.5 border-b-2 transition-all ${
              tab === "running"
                ? "border-brand-500 text-brand-400 font-semibold"
                : "border-transparent text-zinc-400 hover:text-zinc-300"
            }`}
          >
            <Activity className="w-3.5 h-3.5" />
            <span>Select Running Application</span>
          </button>
          <button
            type="button"
            onClick={() => setTab("browse")}
            className={`flex items-center gap-1.5 pb-2.5 border-b-2 transition-all ${
              tab === "browse"
                ? "border-brand-500 text-brand-400 font-semibold"
                : "border-transparent text-zinc-400 hover:text-zinc-300"
            }`}
          >
            <FolderOpen className="w-3.5 h-3.5" />
            <span>Browse / Enter .EXE</span>
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-5 space-y-4">
          {/* Method 1: Running Process Selection */}
          {tab === "running" && (
            <div className="space-y-2">
              <div className="relative">
                <Search className="w-3.5 h-3.5 absolute left-3 top-2.5 text-zinc-500" />
                <input
                  type="text"
                  placeholder="Search running applications..."
                  value={runningFilter}
                  onChange={(e) => setRunningFilter(e.target.value)}
                  className="w-full pl-8 pr-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-brand-500"
                />
              </div>

              <div className="border border-zinc-800/80 rounded-xl bg-zinc-950/60 max-h-48 overflow-y-auto divide-y divide-zinc-900">
                {loadingApps ? (
                  <div className="p-6 text-center text-xs text-zinc-500">Detecting running applications...</div>
                ) : filteredRunning.length === 0 ? (
                  <div className="p-6 text-center text-xs text-zinc-500">No applications matched. Switch to "Browse .EXE" above.</div>
                ) : (
                  filteredRunning.map((app) => {
                    const isSelected = processName.toLowerCase() === app.processName.toLowerCase();
                    return (
                      <button
                        key={`${app.pid}-${app.processName}`}
                        type="button"
                        onClick={() => handleSelectRunningApp(app)}
                        className={`w-full flex items-center justify-between p-2.5 text-left transition-colors ${
                          isSelected
                            ? "bg-brand-600/15 border-l-2 border-brand-500 text-zinc-100"
                            : "hover:bg-zinc-900/60 text-zinc-300"
                        }`}
                      >
                        <div className="flex items-center gap-2.5">
                          <AppIcon processName={app.processName} displayName={app.name} iconBase64={app.iconBase64} size="sm" />
                          <div>
                            <div className="text-xs font-semibold">{app.name}</div>
                            <div className="text-[11px] font-mono text-zinc-500">{app.processName}</div>
                          </div>
                        </div>
                        {isSelected && <CheckCircle2 className="w-4 h-4 text-brand-400 flex-shrink-0" />}
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
                <label className="block text-xs font-semibold text-zinc-300 mb-1">
                  Select Executable File (.exe)
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="e.g. C:\Program Files\Spotify\Spotify.exe or Telegram.exe"
                    value={executablePath || processName}
                    onChange={(e) => handleBrowsePathChange(e.target.value)}
                    className="flex-1 px-3 py-2 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                  />
                  <button
                    type="button"
                    onClick={handleNativeBrowse}
                    className="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-xs font-medium flex items-center gap-1.5 shadow-sm transition-colors"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Browse...</span>
                  </button>
                </div>
                <p className="text-[11px] text-zinc-500 mt-1">
                  Click <span className="text-zinc-300 font-medium">Browse</span> to pick any Windows .exe with the native open dialog.
                </p>
              </div>

              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">
                  Display Name
                </label>
                <input
                  type="text"
                  placeholder="e.g. Spotify"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 focus:outline-none focus:border-brand-500"
                />
              </div>
            </div>
          )}

          {/* Duplicate Warning */}
          {duplicateRule && (
            <div className="p-3 rounded-xl bg-amber-950/20 border border-amber-500/30 flex items-start justify-between gap-2.5 text-xs text-amber-300">
              <div className="flex items-start gap-2.5">
                <AlertCircle className="w-4 h-4 text-amber-400 flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-semibold">Application Already Configured</div>
                  <div className="text-[11px] text-zinc-400 mt-0.5">
                    <span className="font-semibold text-zinc-200">{duplicateRule.displayName || duplicateRule.name}</span> ({duplicateRule.processName}) is already configured.
                  </div>
                </div>
              </div>
              {onEditExisting && (
                <button
                  type="button"
                  onClick={() => onEditExisting(duplicateRule)}
                  className="flex items-center gap-1 px-2.5 py-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 font-medium text-[11px] flex-shrink-0"
                >
                  <Edit2 className="w-3 h-3" />
                  <span>Edit Rule</span>
                </button>
              )}
            </div>
          )}

          {/* Selected Application Preview */}
          {processName && !duplicateRule && (
            <div className="flex items-center gap-3 p-3 rounded-xl bg-zinc-900/60 border border-zinc-800">
              <AppIcon processName={processName} displayName={displayName} iconBase64={iconBase64} size="md" />
              <div>
                <div className="text-xs font-semibold text-zinc-200">
                  {displayName || processName.replace(/\.exe$/i, "")}
                </div>
                <div className="text-[11px] font-mono text-zinc-400">{processName}</div>
              </div>
            </div>
          )}

          {/* Routing Destination Selection */}
          <div className="space-y-2 pt-1">
            <label className="block text-xs font-semibold text-zinc-300">
              Route this application through:
            </label>

            <div className="grid grid-cols-3 gap-2.5">
              {/* Secondary Proxy */}
              <button
                type="button"
                onClick={() => setDestination("secondaryProxy")}
                className={`flex flex-col items-center p-3 rounded-xl border text-center transition-all ${
                  destination === "secondaryProxy"
                    ? "bg-cyan-950/30 border-cyan-500 text-cyan-300 shadow-md shadow-cyan-950/30"
                    : "bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                <Server className="w-4 h-4 mb-1.5" />
                <span className="text-xs font-semibold">Secondary Proxy</span>
                <span className="text-[10px] text-zinc-500 mt-0.5">V2Ray / Xray</span>
              </button>

              {/* Direct */}
              <button
                type="button"
                onClick={() => setDestination("direct")}
                className={`flex flex-col items-center p-3 rounded-xl border text-center transition-all ${
                  destination === "direct"
                    ? "bg-emerald-950/30 border-emerald-500 text-emerald-300 shadow-md shadow-emerald-950/30"
                    : "bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                <Zap className="w-4 h-4 mb-1.5" />
                <span className="text-xs font-semibold">Direct Internet</span>
                <span className="text-[10px] text-zinc-500 mt-0.5">Bypass VPN</span>
              </button>

              {/* Aether */}
              <button
                type="button"
                onClick={() => setDestination("aether")}
                className={`flex flex-col items-center p-3 rounded-xl border text-center transition-all ${
                  destination === "aether"
                    ? "bg-brand-950/30 border-brand-500 text-brand-300 shadow-md shadow-brand-900/30"
                    : "bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                <ShieldCheck className="w-4 h-4 mb-1.5" />
                <span className="text-xs font-semibold">Aether Tunnel</span>
                <span className="text-[10px] text-zinc-500 mt-0.5">Primary VPN</span>
              </button>
            </div>
          </div>

          {/* Footer Buttons */}
          <div className="flex items-center justify-end gap-2 pt-3 border-t border-zinc-800">
            <button
              type="button"
              onClick={onClose}
              className="px-3.5 py-1.5 rounded-lg border border-zinc-800 text-zinc-300 hover:bg-zinc-800 text-xs font-medium transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!processName.trim() || !!duplicateRule}
              className="px-4 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 disabled:opacity-40 disabled:pointer-events-none text-white text-xs font-semibold shadow-md shadow-brand-900/30 transition-all active:scale-95"
            >
              Add Application Rule
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
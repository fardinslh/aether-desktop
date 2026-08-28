import React, { useState, useEffect } from "react";
import { AppSettings, DependencyStatus, DownloadProgress } from "../../types";
import { api } from "../../services/api";
import {
  ShieldCheck,
  CheckCircle2,
  AlertCircle,
  Download,
  FolderOpen,
  ArrowRight,
  Server,
  Loader2,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

interface FirstRunWizardProps {
  settings?: AppSettings;
  currentSettings?: AppSettings;
  onComplete: (updated: AppSettings) => Promise<void>;
}

export const FirstRunWizard: React.FC<FirstRunWizardProps> = ({ settings, currentSettings: propCurrent, onComplete }) => {
  const currentSettings = (settings || propCurrent)!;

  const [depStatus, setDepStatus] = useState<DependencyStatus | null>(null);
  const [aetherInstalling, setAetherInstalling] = useState(false);
  const [aetherProgress, setAetherProgress] = useState<DownloadProgress | null>(null);
  const [aetherError, setAetherError] = useState<string | null>(null);

  const [singboxInstalling, setSingboxInstalling] = useState(false);
  const [singboxProgress, setSingboxProgress] = useState<DownloadProgress | null>(null);
  const [singboxError, setSingboxError] = useState<string | null>(null);

  const [showAdvanced, setShowAdvanced] = useState(false);
  const [manualAetherPath, setManualAetherPath] = useState(currentSettings.aether.executablePath);
  const [manualSingboxPath, setManualSingboxPath] = useState(currentSettings.singBox.executablePath);
  const [manualAetherFeedback, setManualAetherFeedback] = useState<string | null>(null);
  const [manualSingboxFeedback, setManualSingboxFeedback] = useState<string | null>(null);

  const [secProxyHost, setSecProxyHost] = useState(currentSettings.secondaryProxy.host);
  const [secProxyPort, setSecProxyPort] = useState(currentSettings.secondaryProxy.port);
  const [secProxyTesting, setSecProxyTesting] = useState(false);
  const [secProxyResult, setSecProxyResult] = useState<string | null>(null);

  const [isFinishing, setIsFinishing] = useState(false);

  const refreshDependencies = async () => {
    try {
      const status = await api.checkDependencies();
      setDepStatus(status);
      setManualAetherPath(status.aetherPath);
      setManualSingboxPath(status.singboxPath);
    } catch (e) {
      console.error("Failed to check dependencies:", e);
    }
  };

  useEffect(() => {
    refreshDependencies();

    let unlisten: (() => void) | undefined;
    api.onDependencyProgress((progress) => {
      if (progress.component === "aether") {
        setAetherProgress(progress);
      } else if (progress.component === "sing-box") {
        setSingboxProgress(progress);
      }
    }).then((u) => {
      unlisten = u;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleInstallAether = async () => {
    setAetherInstalling(true);
    setAetherError(null);
    try {
      await api.installAether();
      await refreshDependencies();
    } catch (err: any) {
      setAetherError(err?.toString() || "Failed to install Aether");
    } finally {
      setAetherInstalling(false);
      setAetherProgress(null);
    }
  };

  const handleInstallSingbox = async () => {
    setSingboxInstalling(true);
    setSingboxError(null);
    try {
      await api.installSingbox();
      await refreshDependencies();
    } catch (err: any) {
      setSingboxError(err?.toString() || "Failed to install sing-box");
    } finally {
      setSingboxInstalling(false);
      setSingboxProgress(null);
    }
  };

  const handleBrowseAether = async () => {
    try {
      const selected = await api.pickExecutableFile();
      if (selected) {
        setManualAetherPath(selected);
        try {
          const ver = await api.validateAetherPath(selected);
          setManualAetherFeedback(`Validated: ${ver}`);
          const updated = {
            ...currentSettings,
            aether: { ...currentSettings.aether, executablePath: selected },
          };
          await api.saveSettings(updated);
          await refreshDependencies();
        } catch (err: any) {
          setManualAetherFeedback(`Validation failed: ${err}`);
        }
      }
    } catch (e) {
      console.error("Error picking Aether:", e);
    }
  };

  const handleBrowseSingbox = async () => {
    try {
      const selected = await api.pickExecutableFile();
      if (selected) {
        setManualSingboxPath(selected);
        try {
          const ver = await api.validateSingboxPath(selected);
          setManualSingboxFeedback(`Validated: ${ver}`);
          const updated = {
            ...currentSettings,
            singBox: { ...currentSettings.singBox, executablePath: selected },
          };
          await api.saveSettings(updated);
          await refreshDependencies();
        } catch (err: any) {
          setManualSingboxFeedback(`Validation failed: ${err}`);
        }
      }
    } catch (e) {
      console.error("Error picking sing-box:", e);
    }
  };

  const handleTestSecondaryProxy = async () => {
    setSecProxyTesting(true);
    setSecProxyResult(null);
    try {
      const trace = await api.testSecondaryProxy();
      setSecProxyResult(`Online (POP: ${trace.colo}, ${trace.latencyMs} ms)`);
    } catch (e: any) {
      setSecProxyResult(`Unreachable (${e?.toString() || "Connection error"})`);
    } finally {
      setSecProxyTesting(false);
    }
  };

  const isAetherReady = depStatus?.aetherInstalled === true;
  const isSingboxReady = depStatus?.singboxInstalled === true;
  const canFinish = isAetherReady && isSingboxReady;

  const handleFinish = async () => {
    if (!canFinish) return;
    setIsFinishing(true);
    try {
      const updated: AppSettings = {
        ...currentSettings,
        aether: {
          ...currentSettings.aether,
          executablePath: manualAetherPath,
        },
        singBox: {
          ...currentSettings.singBox,
          executablePath: manualSingboxPath,
        },
        secondaryProxy: {
          ...currentSettings.secondaryProxy,
          host: secProxyHost,
          port: Number(secProxyPort) || 10808,
        },
        firstRunCompleted: true,
      };
      await onComplete(updated);
    } catch (e) {
      console.error("Failed to complete first run wizard:", e);
    } finally {
      setIsFinishing(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-background-base/95 backdrop-blur-md z-40 flex items-center justify-center p-6 select-none overflow-y-auto">
      <div className="bg-background-surface border border-zinc-800/90 rounded-2xl max-w-2xl w-full p-8 shadow-2xl space-y-6">
        {/* Header */}
        <div className="text-center space-y-2">
          <div className="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-brand-600/10 border border-brand-500/20 text-brand-400 mb-2">
            <ShieldCheck className="w-8 h-8 text-brand-400" />
          </div>
          <h1 className="text-2xl font-bold text-zinc-100 tracking-tight">Welcome to Aether Desktop</h1>
          <p className="text-sm text-zinc-400 max-w-md mx-auto">
            Automated, split-tunnel VPN orchestration designed for seamless gaming, voice chat, and secure internet routing.
          </p>
        </div>

        {/* Required Dependencies Section */}
        <div className="space-y-4">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 flex items-center gap-2">
            <span>Required Core Engines</span>
            <span className="text-[10px] lowercase text-zinc-500">(installed automatically)</span>
          </h2>

          {/* Aether Card */}
          <div className="p-4 rounded-xl bg-background-subtle border border-zinc-800/80 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-lg bg-brand-600/20 border border-brand-500/30 flex items-center justify-center text-brand-400 font-bold text-sm">
                  AE
                </div>
                <div>
                  <div className="text-sm font-semibold text-zinc-200 flex items-center gap-2">
                    <span>Aether Core Client</span>
                    {isAetherReady ? (
                      <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-emerald-950/70 text-emerald-400 border border-emerald-800/50">
                        <CheckCircle2 className="w-3 h-3" /> Ready {depStatus?.aetherVersion ? `(${depStatus.aetherVersion})` : ""}
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-amber-950/70 text-amber-400 border border-amber-800/50">
                        <AlertCircle className="w-3 h-3" /> Not Installed
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-zinc-400">
                    Primary VPN client tunnel running as a local SOCKS5 proxy on port 1819.
                  </p>
                </div>
              </div>

              {!isAetherReady && (
                <button
                  onClick={handleInstallAether}
                  disabled={aetherInstalling}
                  className="px-3.5 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white font-medium text-xs flex items-center gap-1.5 shadow-sm transition-all disabled:opacity-50"
                >
                  {aetherInstalling ? (
                    <>
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      <span>Installing...</span>
                    </>
                  ) : (
                    <>
                      <Download className="w-3.5 h-3.5" />
                      <span>Download & Install</span>
                    </>
                  )}
                </button>
              )}
            </div>

            {/* Aether Download Progress */}
            {aetherProgress && (
              <div className="space-y-1.5 pt-2 border-t border-zinc-800/60">
                <div className="flex justify-between text-xs text-zinc-400">
                  <span>{aetherProgress.status}</span>
                  <span className="font-mono text-brand-400 font-semibold">{aetherProgress.percent}%</span>
                </div>
                <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-brand-500 rounded-full transition-all duration-200"
                    style={{ width: `${aetherProgress.percent}%` }}
                  />
                </div>
              </div>
            )}

            {aetherError && (
              <div className="text-xs text-rose-400 bg-rose-950/40 p-2.5 rounded-lg border border-rose-900/50">
                {aetherError}
              </div>
            )}
          </div>

          {/* sing-box Card */}
          <div className="p-4 rounded-xl bg-background-subtle border border-zinc-800/80 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-lg bg-blue-600/20 border border-blue-500/30 flex items-center justify-center text-blue-400 font-bold text-sm">
                  SB
                </div>
                <div>
                  <div className="text-sm font-semibold text-zinc-200 flex items-center gap-2">
                    <span>sing-box TUN Router</span>
                    {isSingboxReady ? (
                      <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-emerald-950/70 text-emerald-400 border border-emerald-800/50">
                        <CheckCircle2 className="w-3 h-3" /> Ready {depStatus?.singboxVersion ? `(${depStatus.singboxVersion.split(' ')[0]})` : ""}
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-amber-950/70 text-amber-400 border border-amber-800/50">
                        <AlertCircle className="w-3 h-3" /> Not Installed
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-zinc-400">
                    High-performance TUN adapter managing Windows routing rules and live bypasses.
                  </p>
                </div>
              </div>

              {!isSingboxReady && (
                <button
                  onClick={handleInstallSingbox}
                  disabled={singboxInstalling}
                  className="px-3.5 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white font-medium text-xs flex items-center gap-1.5 shadow-sm transition-all disabled:opacity-50"
                >
                  {singboxInstalling ? (
                    <>
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      <span>Installing...</span>
                    </>
                  ) : (
                    <>
                      <Download className="w-3.5 h-3.5" />
                      <span>Download & Install</span>
                    </>
                  )}
                </button>
              )}
            </div>

            {/* sing-box Download Progress */}
            {singboxProgress && (
              <div className="space-y-1.5 pt-2 border-t border-zinc-800/60">
                <div className="flex justify-between text-xs text-zinc-400">
                  <span>{singboxProgress.status}</span>
                  <span className="font-mono text-blue-400 font-semibold">{singboxProgress.percent}%</span>
                </div>
                <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-blue-500 rounded-full transition-all duration-200"
                    style={{ width: `${singboxProgress.percent}%` }}
                  />
                </div>
              </div>
            )}

            {singboxError && (
              <div className="text-xs text-rose-400 bg-rose-950/40 p-2.5 rounded-lg border border-rose-900/50">
                {singboxError}
              </div>
            )}
          </div>

          {/* Secondary Proxy (Optional) */}
          <div className="p-4 rounded-xl bg-background-subtle border border-zinc-800/80 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-lg bg-purple-600/20 border border-purple-500/30 flex items-center justify-center text-purple-400">
                  <Server className="w-4 h-4" />
                </div>
                <div>
                  <div className="text-sm font-semibold text-zinc-200 flex items-center gap-2">
                    <span>Secondary Proxy (V2Ray / Xray)</span>
                    <span className="text-[10px] text-zinc-500 uppercase tracking-wider font-normal">Optional</span>
                  </div>
                  <p className="text-xs text-zinc-400">
                    Used for routing Discord, Chrome, and AI Developer tools (default: 127.0.0.1:10808).
                  </p>
                </div>
              </div>

              <button
                onClick={handleTestSecondaryProxy}
                disabled={secProxyTesting}
                className="px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium transition-colors"
              >
                {secProxyTesting ? "Testing..." : "Test Connection"}
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3 pt-2">
              <div>
                <label className="text-[11px] text-zinc-400">Host</label>
                <input
                  type="text"
                  value={secProxyHost}
                  onChange={(e) => setSecProxyHost(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-2.5 py-1 text-xs text-zinc-200 font-mono mt-0.5"
                />
              </div>
              <div>
                <label className="text-[11px] text-zinc-400">Port</label>
                <input
                  type="number"
                  value={secProxyPort}
                  onChange={(e) => setSecProxyPort(Number(e.target.value))}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-2.5 py-1 text-xs text-zinc-200 font-mono mt-0.5"
                />
              </div>
            </div>

            {secProxyResult && (
              <div className="text-xs font-mono text-zinc-400 pt-1">
                Status: <span className={secProxyResult.startsWith("Online") ? "text-emerald-400 font-semibold" : "text-amber-400"}>{secProxyResult}</span>
              </div>
            )}
          </div>
        </div>

        {/* Collapsible Advanced Section for Manual Path Selection */}
        <div className="border-t border-zinc-800/80 pt-4">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center justify-between w-full text-xs font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
          >
            <span>Advanced: Use Existing Executables</span>
            {showAdvanced ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>

          {showAdvanced && (
            <div className="mt-4 space-y-4 bg-zinc-950/60 p-4 rounded-xl border border-zinc-800/60">
              <div className="space-y-1.5">
                <label className="text-xs text-zinc-400 font-medium">Custom aether.exe Path</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={manualAetherPath}
                    onChange={(e) => setManualAetherPath(e.target.value)}
                    placeholder="C:\Path\To\aether.exe"
                    className="flex-1 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-xs font-mono text-zinc-200"
                  />
                  <button
                    onClick={handleBrowseAether}
                    className="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-xs flex items-center gap-1.5"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Browse</span>
                  </button>
                </div>
                {manualAetherFeedback && (
                  <p className={`text-[11px] ${manualAetherFeedback.startsWith("Validated") ? "text-emerald-400" : "text-amber-400"}`}>
                    {manualAetherFeedback}
                  </p>
                )}
              </div>

              <div className="space-y-1.5">
                <label className="text-xs text-zinc-400 font-medium">Custom sing-box.exe Path</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={manualSingboxPath}
                    onChange={(e) => setManualSingboxPath(e.target.value)}
                    placeholder="C:\Path\To\sing-box.exe"
                    className="flex-1 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-xs font-mono text-zinc-200"
                  />
                  <button
                    onClick={handleBrowseSingbox}
                    className="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-xs flex items-center gap-1.5"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Browse</span>
                  </button>
                </div>
                {manualSingboxFeedback && (
                  <p className={`text-[11px] ${manualSingboxFeedback.startsWith("Validated") ? "text-emerald-400" : "text-amber-400"}`}>
                    {manualSingboxFeedback}
                  </p>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Footer Finish CTA */}
        <div className="flex items-center justify-between pt-2 border-t border-zinc-800/80">
          <div className="text-xs text-zinc-500">
            {canFinish ? (
              <span className="text-emerald-400 flex items-center gap-1.5">
                <CheckCircle2 className="w-3.5 h-3.5" /> Core engines ready to connect
              </span>
            ) : (
              <span className="text-amber-400 flex items-center gap-1.5">
                <AlertCircle className="w-3.5 h-3.5" /> Install Aether and sing-box to continue
              </span>
            )}
          </div>

          <button
            onClick={handleFinish}
            disabled={!canFinish || isFinishing}
            className="px-6 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-medium text-xs flex items-center gap-2 shadow-lg shadow-brand-600/20 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {isFinishing ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Saving Setup...</span>
              </>
            ) : (
              <>
                <span>Complete Setup</span>
                <ArrowRight className="w-4 h-4" />
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
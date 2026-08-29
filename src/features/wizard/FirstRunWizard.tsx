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
    <div className="fixed inset-0 bg-black/85 backdrop-blur-sm z-40 flex items-center justify-center p-6 select-none overflow-y-auto">
      <div className="bg-app-panel border border-app-border rounded-md max-w-2xl w-full p-6 shadow-2xl space-y-5">
        {/* Header */}
        <div className="text-center space-y-1.5 border-b border-app-border pb-4">
          <div className="inline-flex items-center justify-center w-12 h-12 rounded-sm bg-signal-cyan-dim border border-signal-cyan/30 text-signal-cyan mb-1">
            <ShieldCheck className="w-6 h-6 text-signal-cyan" />
          </div>
          <h1 className="text-lg font-bold font-mono tracking-wider text-ink-100 uppercase">
            AETHER DESKTOP INITIALIZATION
          </h1>
          <p className="text-xs text-ink-400 max-w-md mx-auto font-sans">
            Automated, per-application routing orchestration for Windows network stack.
          </p>
        </div>

        {/* Required Dependencies Section */}
        <div className="space-y-3 font-sans">
          <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-ink-200 flex items-center gap-2">
            <span>CORE SUBSYSTEM ENGINES</span>
          </h2>

          {/* Aether Card */}
          <div className="p-3.5 rounded-sm bg-app-surface border border-app-border space-y-2.5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-xs bg-signal-cyan-dim border border-signal-cyan/30 flex items-center justify-center text-signal-cyan font-bold font-mono text-xs">
                  AE
                </div>
                <div>
                  <div className="text-xs font-semibold text-ink-100 flex items-center gap-2">
                    <span>Aether Core Client</span>
                    {isAetherReady ? (
                      <span className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.2 rounded-xs bg-signal-green-dim text-signal-green border border-signal-green/30">
                        <CheckCircle2 className="w-2.5 h-2.5" /> READY {depStatus?.aetherVersion ? `(${depStatus.aetherVersion})` : ""}
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.2 rounded-xs bg-signal-amber-dim text-signal-amber border border-signal-amber/30">
                        <AlertCircle className="w-2.5 h-2.5" /> NOT INSTALLED
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] text-ink-400 font-mono">
                    Local SOCKS5 proxy daemon on port 1819.
                  </p>
                </div>
              </div>

              {!isAetherReady && (
                <button
                  onClick={handleInstallAether}
                  disabled={aetherInstalling}
                  className="px-3 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black font-bold text-xs flex items-center gap-1.5 shadow-sm transition-all disabled:opacity-40 cursor-pointer"
                >
                  {aetherInstalling ? (
                    <>
                      <Loader2 className="w-3 h-3 animate-spin" />
                      <span>INSTALLING...</span>
                    </>
                  ) : (
                    <>
                      <Download className="w-3 h-3" />
                      <span>INSTALL</span>
                    </>
                  )}
                </button>
              )}
            </div>

            {/* Aether Download Progress */}
            {aetherProgress && (
              <div className="space-y-1 pt-2 border-t border-app-border-subtle font-mono text-xs">
                <div className="flex justify-between text-ink-400 text-[10px]">
                  <span>{aetherProgress.status}</span>
                  <span className="text-signal-cyan font-semibold">{aetherProgress.percent}%</span>
                </div>
                <div className="w-full h-1 bg-app-inset rounded-full overflow-hidden">
                  <div
                    className="h-full bg-signal-cyan transition-all duration-200"
                    style={{ width: `${aetherProgress.percent}%` }}
                  />
                </div>
              </div>
            )}

            {aetherError && (
              <div className="text-[11px] font-mono text-signal-red bg-signal-red-dim p-2 rounded-xs border border-signal-red/30">
                {aetherError}
              </div>
            )}
          </div>

          {/* sing-box Card */}
          <div className="p-3.5 rounded-sm bg-app-surface border border-app-border space-y-2.5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-xs bg-signal-cyan-dim border border-signal-cyan/30 flex items-center justify-center text-signal-cyan font-bold font-mono text-xs">
                  SB
                </div>
                <div>
                  <div className="text-xs font-semibold text-ink-100 flex items-center gap-2">
                    <span>sing-box TUN Router</span>
                    {isSingboxReady ? (
                      <span className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.2 rounded-xs bg-signal-green-dim text-signal-green border border-signal-green/30">
                        <CheckCircle2 className="w-2.5 h-2.5" /> READY {depStatus?.singboxVersion ? `(${depStatus.singboxVersion.split(' ')[0]})` : ""}
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.2 rounded-xs bg-signal-amber-dim text-signal-amber border border-signal-amber/30">
                        <AlertCircle className="w-2.5 h-2.5" /> NOT INSTALLED
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] text-ink-400 font-mono">
                    Wintun router managing network paths.
                  </p>
                </div>
              </div>

              {!isSingboxReady && (
                <button
                  onClick={handleInstallSingbox}
                  disabled={singboxInstalling}
                  className="px-3 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black font-bold text-xs flex items-center gap-1.5 shadow-sm transition-all disabled:opacity-40 cursor-pointer"
                >
                  {singboxInstalling ? (
                    <>
                      <Loader2 className="w-3 h-3 animate-spin" />
                      <span>INSTALLING...</span>
                    </>
                  ) : (
                    <>
                      <Download className="w-3 h-3" />
                      <span>INSTALL</span>
                    </>
                  )}
                </button>
              )}
            </div>

            {/* sing-box Download Progress */}
            {singboxProgress && (
              <div className="space-y-1 pt-2 border-t border-app-border-subtle font-mono text-xs">
                <div className="flex justify-between text-ink-400 text-[10px]">
                  <span>{singboxProgress.status}</span>
                  <span className="text-signal-cyan font-semibold">{singboxProgress.percent}%</span>
                </div>
                <div className="w-full h-1 bg-app-inset rounded-full overflow-hidden">
                  <div
                    className="h-full bg-signal-cyan transition-all duration-200"
                    style={{ width: `${singboxProgress.percent}%` }}
                  />
                </div>
              </div>
            )}

            {singboxError && (
              <div className="text-[11px] font-mono text-signal-red bg-signal-red-dim p-2 rounded-xs border border-signal-red/30">
                {singboxError}
              </div>
            )}
          </div>

          {/* Secondary Proxy (Optional) */}
          <div className="p-3.5 rounded-sm bg-app-surface border border-app-border space-y-2.5 font-sans">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-xs bg-signal-amber-dim border border-signal-amber/30 flex items-center justify-center text-signal-amber">
                  <Server className="w-4 h-4" />
                </div>
                <div>
                  <div className="text-xs font-semibold text-ink-100 flex items-center gap-2">
                    <span>Secondary SOCKS5 Proxy</span>
                    <span className="text-[9px] font-mono text-ink-400 border border-app-border-subtle px-1 py-0.1 rounded-xs">
                      OPTIONAL
                    </span>
                  </div>
                  <p className="text-[11px] text-ink-400 font-mono">
                    Targeted proxy for AI tools and Discord (default: 127.0.0.1:10808).
                  </p>
                </div>
              </div>

              <button
                onClick={handleTestSecondaryProxy}
                disabled={secProxyTesting}
                className="px-3 py-1 bg-app-panel hover:bg-app-elevated border border-app-border text-ink-200 text-xs font-mono rounded-sm transition-colors cursor-pointer"
              >
                {secProxyTesting ? "PROBING..." : "PROBE"}
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3 pt-1 font-mono text-xs">
              <div>
                <label className="text-[10px] text-ink-400">HOST</label>
                <input
                  type="text"
                  value={secProxyHost}
                  onChange={(e) => setSecProxyHost(e.target.value)}
                  className="w-full bg-app-inset border border-app-border-subtle rounded-sm px-2.5 py-1 text-xs text-ink-200 font-mono mt-0.5 focus:outline-none focus:border-signal-cyan"
                />
              </div>
              <div>
                <label className="text-[10px] text-ink-400">PORT</label>
                <input
                  type="number"
                  value={secProxyPort}
                  onChange={(e) => setSecProxyPort(Number(e.target.value))}
                  className="w-full bg-app-inset border border-app-border-subtle rounded-sm px-2.5 py-1 text-xs text-ink-200 font-mono mt-0.5 focus:outline-none focus:border-signal-cyan"
                />
              </div>
            </div>

            {secProxyResult && (
              <div className="text-[11px] font-mono text-ink-400 pt-1">
                STATUS: <span className={secProxyResult.startsWith("Online") ? "text-signal-green font-semibold" : "text-signal-amber"}>{secProxyResult}</span>
              </div>
            )}
          </div>
        </div>

        {/* Collapsible Advanced Section for Manual Path Selection */}
        <div className="border-t border-app-border pt-3 font-mono">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center justify-between w-full text-xs text-ink-400 hover:text-ink-200 transition-colors cursor-pointer"
          >
            <span>ADVANCED: SPECIFY CUSTOM BINARIES</span>
            {showAdvanced ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
          </button>

          {showAdvanced && (
            <div className="mt-3 space-y-3 bg-app-inset p-3 rounded-sm border border-app-border-subtle text-xs">
              <div className="space-y-1">
                <label className="text-[10px] text-ink-400">CUSTOM AETHER.EXE PATH</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={manualAetherPath}
                    onChange={(e) => setManualAetherPath(e.target.value)}
                    placeholder="C:\Path\To\aether.exe"
                    className="flex-1 bg-app-panel border border-app-border rounded-sm px-3 py-1.5 text-xs font-mono text-ink-200 focus:outline-none focus:border-signal-cyan"
                  />
                  <button
                    onClick={handleBrowseAether}
                    className="px-3 py-1.5 bg-app-surface hover:bg-app-elevated border border-app-border text-ink-200 rounded-sm text-xs flex items-center gap-1.5 cursor-pointer"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Browse</span>
                  </button>
                </div>
                {manualAetherFeedback && (
                  <p className={`text-[10px] ${manualAetherFeedback.startsWith("Validated") ? "text-signal-green" : "text-signal-amber"}`}>
                    {manualAetherFeedback}
                  </p>
                )}
              </div>

              <div className="space-y-1">
                <label className="text-[10px] text-ink-400">CUSTOM SING-BOX.EXE PATH</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={manualSingboxPath}
                    onChange={(e) => setManualSingboxPath(e.target.value)}
                    placeholder="C:\Path\To\sing-box.exe"
                    className="flex-1 bg-app-panel border border-app-border rounded-sm px-3 py-1.5 text-xs font-mono text-ink-200 focus:outline-none focus:border-signal-cyan"
                  />
                  <button
                    onClick={handleBrowseSingbox}
                    className="px-3 py-1.5 bg-app-surface hover:bg-app-elevated border border-app-border text-ink-200 rounded-sm text-xs flex items-center gap-1.5 cursor-pointer"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Browse</span>
                  </button>
                </div>
                {manualSingboxFeedback && (
                  <p className={`text-[10px] ${manualSingboxFeedback.startsWith("Validated") ? "text-signal-green" : "text-signal-amber"}`}>
                    {manualSingboxFeedback}
                  </p>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Footer Finish CTA */}
        <div className="flex items-center justify-between pt-2 border-t border-app-border font-mono">
          <div className="text-xs">
            {canFinish ? (
              <span className="text-signal-green flex items-center gap-1.5 text-[11px]">
                <CheckCircle2 className="w-3.5 h-3.5" /> SUBSYSTEM ENGINES VERIFIED
              </span>
            ) : (
              <span className="text-signal-amber flex items-center gap-1.5 text-[11px]">
                <AlertCircle className="w-3.5 h-3.5" /> Aether & sing-box required to proceed
              </span>
            )}
          </div>

          <button
            onClick={handleFinish}
            disabled={!canFinish || isFinishing}
            className="px-5 py-2 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black font-bold text-xs flex items-center gap-2 shadow-sm transition-all disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer"
          >
            {isFinishing ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>INITIALIZING...</span>
              </>
            ) : (
              <>
                <span>COMPLETE INITIALIZATION</span>
                <ArrowRight className="w-3.5 h-3.5" />
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
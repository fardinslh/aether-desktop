import React, { useState, useEffect, useRef } from "react";
import { Download, RefreshCw, FileCode, CheckCircle2, Copy, X, ArrowDown } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { LogEntry } from "../../types";
import { api } from "../../services/api";

interface DiagnosticsViewProps {
  logs: LogEntry[];
  onRefreshLogs: () => void;
}

export const DiagnosticsView: React.FC<DiagnosticsViewProps> = ({ logs, onRefreshLogs }) => {
  const [configPreview, setConfigPreview] = useState<string>("");
  const [configLoading, setConfigLoading] = useState<boolean>(false);
  const [showConfigModal, setShowConfigModal] = useState<boolean>(false);
  const [copied, setCopied] = useState<boolean>(false);
  const [selectedSource, setSelectedSource] = useState<string>("ALL");
  const [exportStatus, setExportStatus] = useState<"idle" | "exporting" | "saved">("idle");

  const logContainerRef = useRef<HTMLDivElement>(null);
  const shouldAutoScrollRef = useRef<boolean>(true);
  const [isAutoScrollPaused, setIsAutoScrollPaused] = useState<boolean>(false);

  const handleScroll = () => {
    const el = logContainerRef.current;
    if (!el) return;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    const isNearBottom = distanceFromBottom <= 60;
    shouldAutoScrollRef.current = isNearBottom;
    setIsAutoScrollPaused(!isNearBottom);
  };

  const handleScrollToBottom = () => {
    shouldAutoScrollRef.current = true;
    setIsAutoScrollPaused(false);
    if (logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  };

  const handleFetchConfig = async () => {
    setConfigLoading(true);
    try {
      const preview = await api.getSingBoxConfigPreview();
      setConfigPreview(preview);
      setShowConfigModal(true);
    } catch (e: any) {
      alert("Failed to generate preview: " + e);
    } finally {
      setConfigLoading(false);
    }
  };

  const handleExportLogs = async () => {
    if (exportStatus === "exporting") return;
    setExportStatus("exporting");
    try {
      const now = new Date();
      const yyyy = now.getFullYear();
      const mm = String(now.getMonth() + 1).padStart(2, "0");
      const dd = String(now.getDate()).padStart(2, "0");
      const hh = String(now.getHours()).padStart(2, "0");
      const min = String(now.getMinutes()).padStart(2, "0");
      const ss = String(now.getSeconds()).padStart(2, "0");
      const defaultFilename = `aether-logs-${yyyy}-${mm}-${dd}T${hh}-${min}-${ss}.log`;

      const filePath = await save({
        title: "Export Raw Diagnostics Log",
        defaultPath: defaultFilename,
        filters: [
          { name: "Log Files", extensions: ["log"] },
          { name: "Text Files", extensions: ["txt"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });

      if (!filePath) {
        // User cancelled native dialog -> silently return to idle
        setExportStatus("idle");
        return;
      }

      await api.saveExportedLogs(filePath);
      setExportStatus("saved");
      setTimeout(() => setExportStatus("idle"), 2500);
    } catch (e: any) {
      setExportStatus("idle");
      alert("Failed to export logs: " + (e?.message || e));
    }
  };

  const handleCopyConfig = () => {
    navigator.clipboard.writeText(configPreview);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const filteredLogs = logs.filter((l) =>
    selectedSource === "ALL" ? true : l.source.toUpperCase() === selectedSource
  );

  const DISPLAY_LOG_LIMIT = 1500;
  const isDisplayTruncated = filteredLogs.length > DISPLAY_LOG_LIMIT;
  const displayLogs = isDisplayTruncated
    ? filteredLogs.slice(-DISPLAY_LOG_LIMIT)
    : filteredLogs;

  // 1. When filter changes: snap to newest matching logs and restore live-follow mode
  useEffect(() => {
    shouldAutoScrollRef.current = true;
    setIsAutoScrollPaused(false);
    requestAnimationFrame(() => {
      if (logContainerRef.current) {
        logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
      }
    });
  }, [selectedSource]);

  // 2. When new filtered logs arrive: auto-scroll if live-follow is active
  useEffect(() => {
    if (shouldAutoScrollRef.current && logContainerRef.current) {
      requestAnimationFrame(() => {
        if (logContainerRef.current && shouldAutoScrollRef.current) {
          logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
        }
      });
    }
  }, [displayLogs]);

  return (
    <div className="flex flex-col h-full px-4 py-2.5 space-y-2.5 select-none">
      {/* Console Toolbar Header */}
      <div className="flex items-center justify-between bg-app-panel border border-app-border rounded-md p-3">
        <div>
          <h2 className="text-xs font-bold tracking-wider uppercase text-ink-100 font-mono">
            SYSTEM DIAGNOSTIC CONSOLE
          </h2>
          <p className="text-[11px] text-ink-400 font-sans mt-0.5">
            Real-time event stream, process lifecycle telemetry, and generated Wintun ruleset.
          </p>
        </div>

        <div className="flex items-center gap-2 font-mono">
          <button
            onClick={handleFetchConfig}
            disabled={configLoading}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-sm border border-app-border bg-app-surface hover:bg-app-elevated text-ink-200 text-xs font-medium transition-colors cursor-pointer"
          >
            <FileCode className="w-3.5 h-3.5 text-signal-cyan" />
            <span>Inspect sing-box JSON</span>
          </button>
          <button
            onClick={handleExportLogs}
            disabled={exportStatus === "exporting"}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-sm bg-app-surface hover:bg-app-elevated border border-app-border text-ink-200 text-xs font-medium transition-colors cursor-pointer disabled:opacity-60"
            title="Export all runtime logs to a native file"
          >
            {exportStatus === "saved" ? (
              <>
                <CheckCircle2 className="w-3.5 h-3.5 text-signal-green" />
                <span className="text-signal-green font-semibold">Saved</span>
              </>
            ) : exportStatus === "exporting" ? (
              <>
                <Download className="w-3.5 h-3.5 animate-bounce text-signal-cyan" />
                <span>Exporting...</span>
              </>
            ) : (
              <>
                <Download className="w-3.5 h-3.5" />
                <span>Export Raw Log</span>
              </>
            )}
          </button>
          <button
            onClick={onRefreshLogs}
            className="p-1.5 rounded-sm border border-app-border bg-app-surface hover:bg-app-elevated text-ink-400 hover:text-ink-100 transition-colors cursor-pointer"
            title="Refresh Log Stream"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Subsystem Filter Pills */}
      <div className="flex gap-1.5 font-mono text-[10px]">
        {["ALL", "APP", "STATE", "SETTINGS", "AETHER", "ROUTING", "SECONDARYPROXY"].map((src) => (
          <button
            key={src}
            onClick={() => setSelectedSource(src)}
            className={`px-2 py-1 rounded-xs transition-all cursor-pointer ${
              selectedSource === src
                ? "bg-app-surface text-signal-cyan border border-signal-cyan/40 font-bold shadow-sm"
                : "bg-app-panel text-ink-400 hover:text-ink-200 border border-app-border"
            }`}
          >
            {src}
          </button>
        ))}
      </div>

      {/* Terminal Log Console */}
      <div className="relative flex-1 min-h-0 flex flex-col">
        <div
          ref={logContainerRef}
          onScroll={handleScroll}
          className="flex-1 overflow-y-auto rounded-md border border-app-border bg-app-inset p-3 font-mono text-[11px] space-y-1 max-h-[380px]"
        >
          {displayLogs.length === 0 ? (
            <div className="text-ink-500 text-center py-10">No log telemetry events recorded in current runtime buffer.</div>
          ) : (
            <>
              {isDisplayTruncated && (
                <div className="text-[10px] text-ink-400 bg-app-surface/60 border border-app-border-subtle px-2 py-1 rounded-xs mb-1 text-center font-mono">
                  Displaying latest {DISPLAY_LOG_LIMIT.toLocaleString()} of {filteredLogs.length.toLocaleString()} events · Complete {logs.length.toLocaleString()} log history exported via Export Raw Log
                </div>
              )}
              {displayLogs.map((l) => (
                <div key={l.id} className="flex items-start gap-2 hover:bg-app-surface/40 px-1 py-0.5 rounded-xs">
                  <span className="text-ink-500 shrink-0 select-none text-[10px]">
                    {new Date(l.timestamp).toLocaleTimeString()}
                  </span>
                  <span
                    className={`px-1 py-0.1 rounded-xs text-[9px] font-bold shrink-0 select-none ${
                      l.level === "ERROR"
                        ? "bg-signal-red-dim text-signal-red border border-signal-red/30"
                        : l.level === "WARN"
                        ? "bg-signal-amber-dim text-signal-amber border border-signal-amber/30"
                        : "bg-app-panel text-ink-400 border border-app-border-subtle"
                    }`}
                  >
                    {l.level}
                  </span>
                  <span className="text-signal-cyan shrink-0 font-medium select-none text-[10px]">
                    [{l.source}]
                  </span>
                  <span className="text-ink-200 break-all leading-tight">{l.message}</span>
                </div>
              ))}
            </>
          )}
        </div>

        {/* Floating "Latest logs" button when user scrolled up */}
        {isAutoScrollPaused && filteredLogs.length > 0 && (
          <button
            onClick={handleScrollToBottom}
            className="absolute bottom-3 right-4 z-10 flex items-center gap-1.5 px-2.5 py-1 rounded-sm bg-app-surface/95 hover:bg-app-elevated border border-signal-cyan/60 text-signal-cyan font-mono text-[10px] font-semibold shadow-lg backdrop-blur-xs transition-all cursor-pointer animate-in fade-in duration-150"
            title="Scroll to newest logs"
          >
            <ArrowDown className="w-3 h-3" />
            <span>Latest logs</span>
          </button>
        )}
      </div>

      {/* sing-box JSON Preview Modal */}
      {showConfigModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xs p-4 select-none">
          <div className="w-full max-w-2xl rounded-md bg-app-panel border border-app-border shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
            <div className="flex items-center justify-between px-4 py-3 border-b border-app-border bg-app-surface">
              <div className="flex items-center gap-2">
                <FileCode className="w-4 h-4 text-signal-cyan" />
                <h3 className="text-xs font-bold font-mono tracking-wider text-ink-100 uppercase">
                  ACTIVE SING-BOX CONFIGURATION JSON
                </h3>
              </div>
              <div className="flex items-center gap-2 font-mono">
                <button
                  onClick={handleCopyConfig}
                  className="flex items-center gap-1 px-3 py-1 bg-app-panel hover:bg-app-elevated border border-app-border text-ink-200 text-xs rounded-sm transition-colors cursor-pointer"
                >
                  {copied ? <CheckCircle2 className="w-3 h-3 text-signal-green" /> : <Copy className="w-3 h-3" />}
                  <span>{copied ? "COPIED" : "COPY JSON"}</span>
                </button>
                <button
                  onClick={() => setShowConfigModal(false)}
                  className="p-1 rounded-sm text-ink-400 hover:text-ink-100 hover:bg-app-panel transition-colors cursor-pointer"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="p-4 overflow-y-auto bg-app-inset font-mono text-[11px] text-ink-300">
              <pre className="whitespace-pre-wrap">{configPreview}</pre>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

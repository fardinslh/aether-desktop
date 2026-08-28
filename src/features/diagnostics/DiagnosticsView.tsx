import React, { useState } from "react";
import { Download, RefreshCw, FileCode, CheckCircle2, Copy } from "lucide-react";
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

  const handleFetchConfig = async () => {
    setConfigLoading(true);
    try {
      const preview = await api.generateSingBoxConfigPreview();
      setConfigPreview(preview);
      setShowConfigModal(true);
    } catch (e: any) {
      alert("Failed to generate preview: " + e);
    } finally {
      setConfigLoading(false);
    }
  };

  const handleExportLogs = async () => {
    try {
      const data = await api.exportLogs();
      const blob = new Blob([data], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `aether-logs-${new Date().toISOString().replace(/[:.]/g, "-")}.log`;
      a.click();
    } catch (e) {
      alert("Failed to export logs: " + e);
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

  return (
    <div className="flex flex-col h-full px-4 py-3 space-y-3">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-zinc-100">Live System Diagnostics</h2>
          <p className="text-xs text-zinc-400">
            Real-time event stream and generated sing-box configuration inspection.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleFetchConfig}
            disabled={configLoading}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 text-xs font-medium transition-colors"
          >
            <FileCode className="w-3.5 h-3.5 text-brand-400" />
            <span>View sing-box Config</span>
          </button>
          <button
            onClick={handleExportLogs}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-semibold transition-colors"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Export Logs</span>
          </button>
          <button
            onClick={onRefreshLogs}
            className="p-1.5 rounded-lg border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-zinc-200 transition-colors"
            title="Refresh Logs"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <div className="flex gap-1.5 text-xs">
        {["ALL", "APP", "STATE", "SETTINGS", "AETHER", "ROUTING", "SECONDARYPROXY"].map((src) => (
          <button
            key={src}
            onClick={() => setSelectedSource(src)}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${
              selectedSource === src
                ? "bg-brand-500/20 text-brand-300 border border-brand-500/30"
                : "bg-zinc-900 text-zinc-400 hover:text-zinc-300 border border-zinc-800"
            }`}
          >
            {src}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto rounded-xl border border-zinc-800/80 bg-zinc-950 p-3 font-mono text-[11px] space-y-1 max-h-[360px]">
        {filteredLogs.length === 0 ? (
          <div className="text-zinc-600 text-center py-8">No log events recorded in this session.</div>
        ) : (
          filteredLogs.map((l) => (
            <div key={l.id} className="flex items-start gap-2 hover:bg-zinc-900/40 px-1 py-0.5 rounded">
              <span className="text-zinc-500 shrink-0 select-none">
                {new Date(l.timestamp).toLocaleTimeString()}
              </span>
              <span
                className={`px-1 rounded text-[10px] font-semibold shrink-0 select-none ${
                  l.level === "ERROR"
                    ? "bg-rose-950 text-rose-400 border border-rose-800/40"
                    : l.level === "WARN"
                    ? "bg-amber-950 text-amber-400 border border-amber-800/40"
                    : "bg-zinc-800 text-zinc-400"
                }`}
              >
                {l.level}
              </span>
              <span className="text-brand-400 shrink-0 font-medium select-none">[{l.source}]</span>
              <span className="text-zinc-300 break-all">{l.message}</span>
            </div>
          ))
        )}
      </div>

      {showConfigModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="w-full max-w-2xl rounded-2xl bg-zinc-900 border border-zinc-800 shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
            <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-800">
              <div className="flex items-center gap-2">
                <FileCode className="w-4 h-4 text-brand-400" />
                <h3 className="text-sm font-bold text-zinc-100">Generated sing-box Configuration</h3>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleCopyConfig}
                  className="flex items-center gap-1 px-3 py-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium rounded-lg transition-colors"
                >
                  {copied ? <CheckCircle2 className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
                  <span>{copied ? "Copied!" : "Copy JSON"}</span>
                </button>
                <button
                  onClick={() => setShowConfigModal(false)}
                  className="p-1 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
                >
                  ✕
                </button>
              </div>
            </div>

            <div className="p-4 overflow-y-auto bg-zinc-950 font-mono text-xs text-zinc-300">
              <pre className="whitespace-pre-wrap">{configPreview}</pre>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

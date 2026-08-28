import React, { useState, useEffect } from "react";
import {
  X,
  Trash2,
  Server,
  Zap,
  ShieldCheck,
  Save,
  ChevronDown,
  ChevronRight,
  SlidersHorizontal,
  AlertTriangle,
} from "lucide-react";
import { ApplicationRule, RouteDestination, RulePriority } from "../../types";
import { AppIcon } from "../../components/AppIcon";

interface EditApplicationModalProps {
  isOpen: boolean;
  rule: ApplicationRule | null;
  onClose: () => void;
  onSaveRule: (updatedRule: ApplicationRule) => void;
  onDeleteRule: (id: string) => void;
}

export const EditApplicationModal: React.FC<EditApplicationModalProps> = ({
  isOpen,
  rule,
  onClose,
  onSaveRule,
  onDeleteRule,
}) => {
  const [displayName, setDisplayName] = useState<string>("");
  const [destination, setDestination] = useState<RouteDestination>("secondaryProxy");
  const [priority, setPriority] = useState<RulePriority>("normal");
  const [enabled, setEnabled] = useState<boolean>(true);
  const [isAdvancedOpen, setIsAdvancedOpen] = useState<boolean>(false);

  useEffect(() => {
    if (rule) {
      setDisplayName(rule.displayName || rule.name || "");
      setDestination(rule.destination || rule.route || "secondaryProxy");
      setPriority(rule.priority || "normal");
      setEnabled(rule.enabled);
      // Auto-expand advanced options if rule has high priority so the user understands its status
      setIsAdvancedOpen(rule.priority === "high");
    }
  }, [rule]);

  if (!isOpen || !rule) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const updated: ApplicationRule = {
      ...rule,
      displayName: displayName.trim() || rule.processName.replace(/\.exe$/i, ""),
      destination,
      enabled,
      priority,
    };
    onSaveRule(updated);
    onClose();
  };

  const handleDelete = () => {
    onDeleteRule(rule.id);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4 animate-fade-in">
      <div className="bg-background-card border border-zinc-800 rounded-2xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-800">
          <div className="flex items-center gap-3">
            <AppIcon processName={rule.processName} displayName={displayName} size="md" />
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-zinc-100">{rule.displayName || rule.processName}</h3>
                {rule.source === "preset" && (
                  <span className="text-[10px] px-1.5 py-0.2 rounded bg-zinc-800 text-zinc-400 font-mono">
                    Preset
                  </span>
                )}
                {priority === "high" && (
                  <span className="text-[10px] px-1.5 py-0.2 rounded bg-amber-950/60 border border-amber-600/40 text-amber-300 font-mono font-semibold">
                    High Priority
                  </span>
                )}
              </div>
              <p className="text-xs font-mono text-zinc-400">{rule.processName}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-4 overflow-y-auto flex-1">
          {/* Display Name */}
          <div>
            <label className="block text-xs font-semibold text-zinc-300 mb-1">
              Display Name
            </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 focus:outline-none focus:border-brand-500"
            />
          </div>

          {/* Enabled Toggle */}
          <div className="flex items-center justify-between p-3 rounded-xl bg-zinc-950/60 border border-zinc-800">
            <div>
              <div className="text-xs font-semibold text-zinc-200">Rule Status</div>
              <div className="text-[11px] text-zinc-400">
                {enabled ? "Active - Network traffic is being routed" : "Disabled - Follows default VPN tunnel"}
              </div>
            </div>
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-900 border-zinc-700 cursor-pointer"
            />
          </div>

          {/* Destination Selector */}
          <div className="space-y-2">
            <label className="block text-xs font-semibold text-zinc-300">
              Routing Destination:
            </label>
            <div className="grid grid-cols-3 gap-2">
              <button
                type="button"
                onClick={() => setDestination("secondaryProxy")}
                className={`flex flex-col items-center p-2.5 rounded-xl border text-center transition-all ${
                  destination === "secondaryProxy"
                    ? "bg-cyan-950/30 border-cyan-500 text-cyan-300 shadow-sm"
                    : "bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                <Server className="w-4 h-4 mb-1" />
                <span className="text-[11px] font-semibold">Secondary Proxy</span>
              </button>

              <button
                type="button"
                onClick={() => setDestination("direct")}
                className={`flex flex-col items-center p-2.5 rounded-xl border text-center transition-all ${
                  destination === "direct"
                    ? "bg-emerald-950/30 border-emerald-500 text-emerald-300 shadow-sm"
                    : "bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                <Zap className="w-4 h-4 mb-1" />
                <span className="text-[11px] font-semibold">Direct</span>
              </button>

              <button
                type="button"
                onClick={() => setDestination("aether")}
                className={`flex flex-col items-center p-2.5 rounded-xl border text-center transition-all ${
                  destination === "aether"
                    ? "bg-brand-950/30 border-brand-500 text-brand-300 shadow-sm"
                    : "bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                <ShieldCheck className="w-4 h-4 mb-1" />
                <span className="text-[11px] font-semibold">Aether</span>
              </button>
            </div>
          </div>

          {/* Advanced Collapsible Section */}
          <div className="border border-zinc-800/80 rounded-xl overflow-hidden bg-zinc-950/40">
            <button
              type="button"
              onClick={() => setIsAdvancedOpen(!isAdvancedOpen)}
              className="w-full flex items-center justify-between px-3 py-2.5 text-xs font-semibold text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/40 transition-colors"
            >
              <div className="flex items-center gap-1.5">
                <SlidersHorizontal className="w-3.5 h-3.5 text-zinc-500" />
                <span>Advanced Routing Options</span>
              </div>
              {isAdvancedOpen ? (
                <ChevronDown className="w-3.5 h-3.5 text-zinc-500" />
              ) : (
                <ChevronRight className="w-3.5 h-3.5 text-zinc-500" />
              )}
            </button>

            {isAdvancedOpen && (
              <div className="p-3 border-t border-zinc-800/80 space-y-3 animate-fade-in text-xs">
                <div>
                  <label className="block font-semibold text-zinc-300 mb-1">
                    Rule Precedence & Priority
                  </label>
                  <p className="text-[11px] text-zinc-500 mb-2">
                    Controls whether this application route overrides global STUN/TURN fallback rules (ports 3478, 5349).
                  </p>

                  <div className="space-y-2">
                    <label
                      className={`flex items-start gap-2.5 p-2.5 rounded-lg border cursor-pointer transition-all ${
                        priority === "normal"
                          ? "bg-zinc-900 border-brand-500/60 text-zinc-100"
                          : "bg-zinc-950/80 border-zinc-800/80 text-zinc-400 hover:border-zinc-700"
                      }`}
                    >
                      <input
                        type="radio"
                        name="rulePriority"
                        value="normal"
                        checked={priority === "normal"}
                        onChange={() => setPriority("normal")}
                        className="mt-0.5 text-brand-500 focus:ring-brand-500 bg-zinc-900 border-zinc-700"
                      />
                      <div>
                        <div className="font-semibold text-xs text-zinc-200">Normal Priority (Default)</div>
                        <div className="text-[11px] text-zinc-500">
                          Evaluated after generic STUN/TURN Direct rules. Recommended for web browsers, Spotify, Telegram, and IDEs.
                        </div>
                      </div>
                    </label>

                    <label
                      className={`flex items-start gap-2.5 p-2.5 rounded-lg border cursor-pointer transition-all ${
                        priority === "high"
                          ? "bg-amber-950/20 border-amber-500/60 text-amber-200 shadow-sm"
                          : "bg-zinc-950/80 border-zinc-800/80 text-zinc-400 hover:border-zinc-700"
                      }`}
                    >
                      <input
                        type="radio"
                        name="rulePriority"
                        value="high"
                        checked={priority === "high"}
                        onChange={() => setPriority("high")}
                        className="mt-0.5 text-amber-500 focus:ring-amber-500 bg-zinc-900 border-zinc-700"
                      />
                      <div>
                        <div className="flex items-center gap-1.5 font-semibold text-xs text-amber-300">
                          <span>High Priority Override</span>
                          <AlertTriangle className="w-3 h-3 text-amber-400" />
                        </div>
                        <div className="text-[11px] text-zinc-400">
                          Evaluated <strong>before</strong> generic compatibility rules. Prevents voice/WebRTC connection loops (e.g. Discord Voice channel fix).
                        </div>
                      </div>
                    </label>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex items-center justify-between pt-3 border-t border-zinc-800">
            <button
              type="button"
              onClick={handleDelete}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-rose-400 hover:bg-rose-950/30 border border-transparent hover:border-rose-900/40 text-xs font-medium transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5" />
              <span>Remove Rule</span>
            </button>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3.5 py-1.5 rounded-lg border border-zinc-800 text-zinc-300 hover:bg-zinc-800 text-xs font-medium transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white text-xs font-semibold shadow-md shadow-brand-900/30 transition-all active:scale-95"
              >
                <Save className="w-3.5 h-3.5" />
                <span>Save Changes</span>
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
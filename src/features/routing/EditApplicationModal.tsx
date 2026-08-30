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
  Gamepad2,
} from "lucide-react";
import { ApplicationRule, CompatibilityRule, RouteDestination, RulePriority } from "../../types";
import { AppIcon } from "../../components/AppIcon";

const DOTA_SDR_RULE_ID = "compat-dota2-valve-sdr";

interface EditApplicationModalProps {
  isOpen: boolean;
  rule: ApplicationRule | null;
  compatibilityRules?: CompatibilityRule[];
  onClose: () => void;
  onSaveRule: (updatedRule: ApplicationRule, updatedCompatRules?: CompatibilityRule[]) => void;
  onDeleteRule: (id: string) => void;
}

type DotaGameUdpMode = "follow" | "direct" | "secondary";

export const EditApplicationModal: React.FC<EditApplicationModalProps> = ({
  isOpen,
  rule,
  compatibilityRules = [],
  onClose,
  onSaveRule,
  onDeleteRule,
}) => {
  const [displayName, setDisplayName] = useState<string>("");
  const [destination, setDestination] = useState<RouteDestination>("secondaryProxy");
  const [priority, setPriority] = useState<RulePriority>("normal");
  const [enabled, setEnabled] = useState<boolean>(true);
  const [isAdvancedOpen, setIsAdvancedOpen] = useState<boolean>(false);
  const [dotaUdpMode, setDotaUdpMode] = useState<DotaGameUdpMode>("follow");

  const isDota = rule ? rule.processName.toLowerCase() === "dota2.exe" : false;

  useEffect(() => {
    if (rule) {
      setDisplayName(rule.displayName || rule.name || "");
      setDestination(rule.destination || rule.route || "secondaryProxy");
      setPriority(rule.priority || "normal");
      setEnabled(rule.enabled);

      // Strictly check for the exact managed rule ID
      const dotaManagedRule = compatibilityRules.find(
        (cr) => cr.id === DOTA_SDR_RULE_ID && cr.enabled
      );
      if (dotaManagedRule) {
        if (dotaManagedRule.destination === "direct") {
          setDotaUdpMode("direct");
        } else if (dotaManagedRule.destination === "secondaryProxy") {
          setDotaUdpMode("secondary");
        } else {
          setDotaUdpMode("follow");
        }
      } else {
        setDotaUdpMode("follow");
      }

      // Auto-expand advanced options if rule has high priority or active game compatibility override
      setIsAdvancedOpen(rule.priority === "high" || !!dotaManagedRule);
    }
  }, [rule, compatibilityRules]);

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

    let updatedCompatRules: CompatibilityRule[] | undefined = undefined;

    if (isDota) {
      const otherCompatRules = compatibilityRules.filter((cr) => cr.id !== DOTA_SDR_RULE_ID);
      if (dotaUdpMode === "direct") {
        const dotaCompatRule: CompatibilityRule = {
          id: DOTA_SDR_RULE_ID,
          name: "Valve SDR / Game UDP (Dota 2)",
          description:
            "Routes only Dota 2 Valve UDP traffic (27000-27250) Direct while keeping the rest of Dota on its selected route.",
          enabled: true,
          processNames: ["dota2.exe"],
          network: "udp",
          portRanges: ["27000:27250"],
          ports: Array.from({ length: 251 }, (_, i) => 27000 + i),
          destination: "direct",
          scope: "appScoped",
        };
        updatedCompatRules = [...otherCompatRules, dotaCompatRule];
      } else if (dotaUdpMode === "secondary") {
        const dotaCompatRule: CompatibilityRule = {
          id: DOTA_SDR_RULE_ID,
          name: "Valve SDR / Game UDP (Dota 2)",
          description:
            "Routes only Dota 2 Valve UDP traffic (27000-27250) through Secondary Proxy while keeping the rest of Dota on its selected route.",
          enabled: true,
          processNames: ["dota2.exe"],
          network: "udp",
          portRanges: ["27000:27250"],
          ports: Array.from({ length: 251 }, (_, i) => 27000 + i),
          destination: "secondaryProxy",
          scope: "appScoped",
        };
        updatedCompatRules = [...otherCompatRules, dotaCompatRule];
      } else {
        // "follow" mode removes/disables the managed rule
        updatedCompatRules = otherCompatRules;
      }
    }

    onSaveRule(updated, updatedCompatRules);
    onClose();
  };

  const handleDelete = () => {
    onDeleteRule(rule.id);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xs p-4 select-none">
      <div className="bg-app-panel border border-app-border rounded-md w-full max-w-md shadow-xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-app-border bg-app-surface">
          <div className="flex items-center gap-3">
            <AppIcon processName={rule.processName} displayName={displayName} size="md" />
            <div>
              <div className="flex items-center gap-1.5">
                <h3 className="text-xs font-bold font-mono tracking-wider text-ink-100 uppercase">
                  {rule.displayName || rule.processName}
                </h3>
                {rule.source === "preset" && (
                  <span className="text-[9px] px-1 py-0.1 rounded-xs bg-app-inset text-ink-400 font-mono border border-app-border-subtle">
                    PRESET
                  </span>
                )}
                {priority === "high" && (
                  <span className="text-[9px] px-1 py-0.1 rounded-xs bg-signal-amber-dim border border-signal-amber/40 text-signal-amber font-mono font-semibold">
                    HIGH PRIORITY
                  </span>
                )}
              </div>
              <p className="text-[10px] font-mono text-ink-400">{rule.processName}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-sm text-ink-400 hover:text-ink-100 hover:bg-app-panel transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-4 space-y-3.5 overflow-y-auto flex-1 font-sans">
          {/* Display Name */}
          <div>
            <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300 mb-1">
              Process Display Name
            </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full px-3 py-1.5 bg-app-inset border border-app-border-subtle rounded-sm text-xs font-sans text-ink-200 focus:outline-none focus:border-signal-cyan"
            />
          </div>

          {/* Enabled Toggle */}
          <div className="flex items-center justify-between p-2.5 rounded-sm bg-app-inset border border-app-border-subtle">
            <div>
              <div className="text-xs font-semibold text-ink-100 font-mono">RULE ACTIVE STATE</div>
              <div className="text-[10px] text-ink-400 font-sans">
                {enabled ? "Active in sing-box routing matrix" : "Disabled · Bypasses matrix to global TUN fallback"}
              </div>
            </div>
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="w-3.5 h-3.5 rounded-xs text-signal-cyan focus:ring-signal-cyan bg-app-panel border-app-border cursor-pointer"
            />
          </div>

          {/* Destination Selector */}
          <div className="space-y-1.5">
            <label className="block text-[11px] font-mono font-semibold uppercase text-ink-300">
              Steer Route Destination:
            </label>
            <div className="grid grid-cols-3 gap-2 font-mono">
              {/* Direct */}
              <button
                type="button"
                onClick={() => setDestination("direct")}
                className={`flex flex-col items-center p-2 rounded-sm border text-center transition-all cursor-pointer ${
                  destination === "direct"
                    ? "bg-signal-cyan-dim border-signal-cyan text-signal-cyan"
                    : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border"
                }`}
              >
                <Zap className="w-3.5 h-3.5 mb-1 text-signal-cyan" />
                <span className="text-[10px] font-bold">DIRECT</span>
              </button>

              {/* Secondary */}
              <button
                type="button"
                onClick={() => setDestination("secondaryProxy")}
                className={`flex flex-col items-center p-2 rounded-sm border text-center transition-all cursor-pointer ${
                  destination === "secondaryProxy"
                    ? "bg-signal-amber-dim border-signal-amber text-signal-amber"
                    : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border"
                }`}
              >
                <Server className="w-3.5 h-3.5 mb-1 text-signal-amber" />
                <span className="text-[10px] font-bold">SECONDARY</span>
              </button>

              {/* Aether */}
              <button
                type="button"
                onClick={() => setDestination("aether")}
                className={`flex flex-col items-center p-2 rounded-sm border text-center transition-all cursor-pointer ${
                  destination === "aether"
                    ? "bg-signal-green-dim border-signal-green text-signal-green"
                    : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border"
                }`}
              >
                <ShieldCheck className="w-3.5 h-3.5 mb-1 text-signal-green" />
                <span className="text-[10px] font-bold">AETHER</span>
              </button>
            </div>
          </div>

          {/* Advanced Collapsible Section */}
          <div className="border border-app-border-subtle rounded-sm overflow-hidden bg-app-inset font-mono">
            <button
              type="button"
              onClick={() => setIsAdvancedOpen(!isAdvancedOpen)}
              className="w-full flex items-center justify-between px-3 py-2 text-xs font-semibold text-ink-400 hover:text-ink-200 hover:bg-app-surface transition-colors cursor-pointer"
            >
              <div className="flex items-center gap-1.5">
                <SlidersHorizontal className="w-3.5 h-3.5 text-ink-500" />
                <span>ADVANCED ROUTING PRECEDENCE</span>
              </div>
              {isAdvancedOpen ? (
                <ChevronDown className="w-3.5 h-3.5 text-ink-500" />
              ) : (
                <ChevronRight className="w-3.5 h-3.5 text-ink-500" />
              )}
            </button>

            {isAdvancedOpen && (
              <div className="p-3 border-t border-app-border-subtle space-y-2.5 text-xs">
                <div>
                  <div className="font-semibold text-ink-200 mb-1 text-[11px]">
                    Priority Evaluation Layer
                  </div>
                  <p className="text-[10px] text-ink-400 font-sans mb-2">
                    Controls whether this process route overrides global STUN/TURN fallback bypass rules (ports 3478, 5349).
                  </p>

                  <div className="space-y-1.5 font-sans">
                    <label
                      className={`flex items-start gap-2 p-2 rounded-sm border cursor-pointer transition-all ${
                        priority === "normal"
                          ? "bg-app-surface border-signal-cyan text-ink-100"
                          : "bg-app-panel border-app-border text-ink-400 hover:border-ink-500"
                      }`}
                    >
                      <input
                        type="radio"
                        name="rulePriority"
                        value="normal"
                        checked={priority === "normal"}
                        onChange={() => setPriority("normal")}
                        className="mt-0.5 text-signal-cyan focus:ring-signal-cyan bg-app-inset border-app-border"
                      />
                      <div>
                        <div className="font-semibold text-xs text-ink-100 font-mono">NORMAL PRIORITY (DEFAULT)</div>
                        <div className="text-[10px] text-ink-400">
                          Evaluated after generic STUN/TURN Direct rules. Recommended for web browsers, Spotify, Telegram, and IDEs.
                        </div>
                      </div>
                    </label>

                    <label
                      className={`flex items-start gap-2 p-2 rounded-sm border cursor-pointer transition-all ${
                        priority === "high"
                          ? "bg-signal-amber-dim border-signal-amber text-signal-amber shadow-sm"
                          : "bg-app-panel border-app-border text-ink-400 hover:border-ink-500"
                      }`}
                    >
                      <input
                        type="radio"
                        name="rulePriority"
                        value="high"
                        checked={priority === "high"}
                        onChange={() => setPriority("high")}
                        className="mt-0.5 text-signal-amber focus:ring-signal-amber bg-app-inset border-app-border"
                      />
                      <div>
                        <div className="flex items-center gap-1.5 font-semibold text-xs text-signal-amber font-mono">
                          <span>HIGH PRIORITY OVERRIDE</span>
                          <AlertTriangle className="w-3 h-3 text-signal-amber" />
                        </div>
                        <div className="text-[10px] text-ink-300">
                          Evaluated <strong>before</strong> generic compatibility rules. Fixes voice/WebRTC connection loops (e.g. Discord Voice channel fix).
                        </div>
                      </div>
                    </label>
                  </div>
                </div>

                {/* GAME COMPATIBILITY (Dota 2 Valve SDR UDP) */}
                {isDota && (
                  <div className="pt-2.5 border-t border-app-border-subtle space-y-2 font-sans">
                    <div className="flex items-center justify-between">
                      <div className="font-semibold text-ink-200 text-[11px] font-mono uppercase tracking-wider flex items-center gap-1.5">
                        <Gamepad2 className="w-3.5 h-3.5 text-signal-amber" />
                        <span>GAME COMPATIBILITY</span>
                      </div>
                      <span className="text-[9px] px-1 py-0.2 rounded-xs bg-signal-cyan-dim text-signal-cyan font-mono border border-signal-cyan/30">
                        APP-SCOPED
                      </span>
                    </div>

                    <div className="space-y-1.5 bg-app-panel p-2.5 rounded-sm border border-app-border">
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-semibold text-ink-100 font-mono">
                          Valve SDR / Game UDP
                        </span>
                        {dotaUdpMode !== "follow" && (
                          <span
                            className={`text-[9px] font-mono px-1.5 py-0.2 font-bold rounded-xs ${
                              dotaUdpMode === "secondary"
                                ? "bg-signal-amber text-black"
                                : "bg-signal-cyan text-black"
                            }`}
                          >
                            OVERRIDE: {dotaUdpMode.toUpperCase()}
                          </span>
                        )}
                      </div>

                      <div className="grid grid-cols-3 gap-1.5 font-mono pt-1">
                        <button
                          type="button"
                          onClick={() => setDotaUdpMode("follow")}
                          className={`px-2 py-1.5 rounded-xs border text-center transition-all cursor-pointer text-[10px] font-bold ${
                            dotaUdpMode === "follow"
                              ? "bg-signal-cyan-dim border-signal-cyan text-signal-cyan shadow-sm"
                              : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border hover:text-ink-200"
                          }`}
                        >
                          Follow Main Route
                        </button>
                        <button
                          type="button"
                          onClick={() => setDotaUdpMode("direct")}
                          className={`px-2 py-1.5 rounded-xs border text-center transition-all cursor-pointer text-[10px] font-bold ${
                            dotaUdpMode === "direct"
                              ? "bg-signal-cyan-dim border-signal-cyan text-signal-cyan shadow-sm"
                              : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border hover:text-ink-200"
                          }`}
                        >
                          Direct
                        </button>
                        <button
                          type="button"
                          onClick={() => setDotaUdpMode("secondary")}
                          className={`px-2 py-1.5 rounded-xs border text-center transition-all cursor-pointer text-[10px] font-bold ${
                            dotaUdpMode === "secondary"
                              ? "bg-signal-amber-dim border-signal-amber text-signal-amber shadow-sm"
                              : "bg-app-inset border-app-border-subtle text-ink-400 hover:border-app-border hover:text-ink-200"
                          }`}
                        >
                          Secondary
                        </button>
                      </div>

                      <p className="text-[10px] text-ink-300 font-sans leading-tight pt-1">
                        Controls only Dota 2 UDP traffic on Valve game/SDR ports 27000-27250.
                        All other Dota traffic continues to use the main application route.
                      </p>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex items-center justify-between pt-3 border-t border-app-border font-mono">
            <button
              type="button"
              onClick={handleDelete}
              className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-sm text-signal-red hover:bg-signal-red-dim border border-transparent hover:border-signal-red/30 text-xs transition-colors cursor-pointer"
            >
              <Trash2 className="w-3.5 h-3.5" />
              <span>Delete</span>
            </button>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1.5 rounded-sm border border-app-border text-ink-300 hover:bg-app-surface text-xs transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="flex items-center gap-1.5 px-4 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black font-bold text-xs transition-all shadow-sm cursor-pointer"
              >
                <Save className="w-3.5 h-3.5" />
                <span>Save Rule</span>
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
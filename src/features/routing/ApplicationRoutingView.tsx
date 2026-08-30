import React, { useState } from "react";
import {
  Plus,
  Search,
  Trash2,
  Edit2,
  RefreshCw,
  ArrowRight,
} from "lucide-react";
import { ApplicationRule, CompatibilityRule, RouteDestination } from "../../types";
import { AppIcon } from "../../components/AppIcon";
import { AddApplicationModal } from "./AddApplicationModal";
import { EditApplicationModal } from "./EditApplicationModal";

interface ApplicationRoutingViewProps {
  rules: ApplicationRule[];
  compatibilityRules?: CompatibilityRule[];
  onAddRule: (rule: ApplicationRule) => void;
  onUpdateRule: (rule: ApplicationRule, updatedCompatRules?: CompatibilityRule[]) => void;
  onToggleRule: (id: string, enabled: boolean) => void;
  onDeleteRule: (id: string) => void;
  onChangeRoute: (id: string, route: RouteDestination) => void;
  isApplying?: boolean;
}

export const ApplicationRoutingView: React.FC<ApplicationRoutingViewProps> = ({
  rules,
  compatibilityRules,
  onAddRule,
  onUpdateRule,
  onToggleRule,
  onDeleteRule,
  onChangeRoute,
  isApplying = false,
}) => {
  const [filter, setFilter] = useState<string>("");
  const [activeTab, setActiveTab] = useState<"all" | "direct" | "secondaryProxy" | "aether">("all");
  const [isAddModalOpen, setIsAddModalOpen] = useState<boolean>(false);
  const [editingRule, setEditingRule] = useState<ApplicationRule | null>(null);

  const filteredRules = rules.filter((r) => {
    const dName = r.displayName || r.name || "";
    const pName = r.processName || "";
    const matchesQuery =
      dName.toLowerCase().includes(filter.toLowerCase()) ||
      pName.toLowerCase().includes(filter.toLowerCase());
    
    const currentDest = r.destination || r.route || "aether";
    const matchesTab = activeTab === "all" || currentDest === activeTab;
    return matchesQuery && matchesTab;
  });

  return (
    <div className="flex flex-col h-full space-y-2.5 px-4 py-2.5 select-none">
      {/* Header Bar */}
      <div className="flex items-center justify-between gap-3 bg-app-panel border border-app-border rounded-md p-3">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-xs font-bold tracking-wider uppercase text-ink-100 font-mono">
              APPLICATION ROUTING MATRIX
            </h2>
            {isApplying && (
              <span className="inline-flex items-center gap-1.5 px-2 py-0.2 rounded-xs text-[10px] font-mono bg-signal-cyan-dim text-signal-cyan border border-signal-cyan/30 animate-pulse">
                <RefreshCw className="w-2.5 h-2.5 animate-spin" />
                <span>Applying Routing Stack...</span>
              </span>
            )}
          </div>
          <p className="text-[11px] text-ink-400 font-sans mt-0.5">
            Deterministic per-process traffic steering between Direct Bypass, Secondary Proxy, and Aether TUN.
          </p>
        </div>

        <button
          onClick={() => setIsAddModalOpen(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-sm bg-signal-cyan hover:bg-signal-cyan-muted text-black text-xs font-bold transition-all shadow-sm cursor-pointer flex-shrink-0"
        >
          <Plus className="w-3.5 h-3.5 stroke-[2.5]" />
          <span>Add Process</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 bg-app-panel border border-app-border rounded-md p-2">
        <div className="flex items-center gap-1 bg-app-inset p-0.5 rounded-sm border border-app-border-subtle">
          {(
            [
              { id: "all", label: `All Routes (${rules.length})` },
              {
                id: "secondaryProxy",
                label: `Secondary (${rules.filter((r) => (r.destination || r.route) === "secondaryProxy").length})`,
              },
              {
                id: "direct",
                label: `Direct (${rules.filter((r) => (r.destination || r.route) === "direct").length})`,
              },
              {
                id: "aether",
                label: `Aether (${rules.filter((r) => (r.destination || r.route) === "aether").length})`,
              },
            ] as const
          ).map((t) => (
            <button
              key={t.id}
              onClick={() => setActiveTab(t.id)}
              className={`px-2 py-1 text-[11px] font-mono rounded-xs transition-all cursor-pointer ${
                activeTab === t.id
                  ? "bg-app-surface text-ink-100 border border-app-border font-semibold shadow-sm"
                  : "text-ink-400 hover:text-ink-200 border border-transparent"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="relative w-56">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-2 text-ink-400" />
          <input
            type="text"
            placeholder="Filter process or app name..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-full pl-8 pr-3 py-1 bg-app-inset border border-app-border-subtle rounded-sm text-xs font-mono text-ink-200 placeholder-ink-500 focus:outline-none focus:border-signal-cyan"
          />
        </div>
      </div>

      {/* Rule List Matrix Container */}
      <div className="flex-1 overflow-y-auto rounded-md border border-app-border bg-app-panel divide-y divide-app-border-subtle max-h-[380px]">
        {filteredRules.length === 0 ? (
          <div className="p-8 text-center text-ink-400 text-xs font-mono flex flex-col items-center justify-center space-y-2">
            <div>No matching application process rules found.</div>
            <button
              onClick={() => setIsAddModalOpen(true)}
              className="text-signal-cyan hover:underline font-mono text-xs cursor-pointer"
            >
              + Add a new application rule
            </button>
          </div>
        ) : (
          filteredRules.map((rule) => {
            const currentDest = rule.destination || rule.route || "aether";
            const displayName = rule.displayName || rule.name || rule.processName;
            return (
              <div
                key={rule.id}
                className={`flex items-center justify-between px-3 py-2 transition-colors ${
                  rule.enabled ? "hover:bg-app-surface/60" : "opacity-40 hover:bg-app-inset"
                }`}
              >
                {/* Left: App Identity */}
                <div className="flex items-center gap-2.5 min-w-[220px]">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(e) => onToggleRule(rule.id, e.target.checked)}
                    className="w-3.5 h-3.5 rounded-xs border-app-border bg-app-inset text-signal-cyan focus:ring-signal-cyan cursor-pointer"
                    title={rule.enabled ? "Disable rule" : "Enable rule"}
                  />

                  <AppIcon
                    processName={rule.processName}
                    displayName={displayName}
                    iconBase64={rule.iconBase64}
                    size="sm"
                  />

                  <div className="flex flex-col">
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs font-semibold text-ink-100">{displayName}</span>
                      {rule.source === "preset" && (
                        <span className="text-[9px] px-1 py-0.1 rounded-xs bg-app-inset text-ink-400 font-mono border border-app-border-subtle">
                          PRESET
                        </span>
                      )}
                    </div>
                    <span className="text-[10px] font-mono text-ink-400">{rule.processName}</span>
                  </div>
                </div>

                {/* Center: Topological Signal Rail Line */}
                <div className="hidden sm:flex flex-1 items-center px-4">
                  <div className="w-full flex items-center gap-1">
                    <div className="w-1.5 h-1.5 rounded-full bg-app-border" />
                    <div className="flex-1 h-px bg-app-border-subtle relative">
                      <div className={`absolute right-0 top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full ${
                        currentDest === "secondaryProxy"
                          ? "bg-signal-amber"
                          : currentDest === "direct"
                          ? "bg-signal-cyan"
                          : "bg-signal-green"
                      }`} />
                    </div>
                    <ArrowRight className={`w-3 h-3 ${
                      currentDest === "secondaryProxy"
                        ? "text-signal-amber"
                        : currentDest === "direct"
                        ? "text-signal-cyan"
                        : "text-signal-green"
                    }`} />
                  </div>
                </div>

                {/* Right: Route Switch & Actions */}
                <div className="flex items-center gap-2">
                  <div className="flex items-center bg-app-inset p-0.5 rounded-sm border border-app-border-subtle font-mono text-[10px]">
                    <button
                      onClick={() => onChangeRoute(rule.id, "direct")}
                      className={`px-2 py-0.5 rounded-xs transition-all cursor-pointer ${
                        currentDest === "direct"
                          ? "bg-signal-cyan/20 text-signal-cyan border border-signal-cyan/40 font-semibold"
                          : "text-ink-400 hover:text-ink-200"
                      }`}
                    >
                      DIRECT
                    </button>
                    <button
                      onClick={() => onChangeRoute(rule.id, "secondaryProxy")}
                      className={`px-2 py-0.5 rounded-xs transition-all cursor-pointer ${
                        currentDest === "secondaryProxy"
                          ? "bg-signal-amber/20 text-signal-amber border border-signal-amber/40 font-semibold"
                          : "text-ink-400 hover:text-ink-200"
                      }`}
                    >
                      SECONDARY
                    </button>
                    <button
                      onClick={() => onChangeRoute(rule.id, "aether")}
                      className={`px-2 py-0.5 rounded-xs transition-all cursor-pointer ${
                        currentDest === "aether"
                          ? "bg-signal-green/20 text-signal-green border border-signal-green/40 font-semibold"
                          : "text-ink-400 hover:text-ink-200"
                      }`}
                    >
                      AETHER
                    </button>
                  </div>

                  <button
                    onClick={() => setEditingRule(rule)}
                    className="p-1 text-ink-400 hover:text-ink-100 hover:bg-app-surface rounded-sm transition-colors cursor-pointer"
                    title="Edit Rule"
                  >
                    <Edit2 className="w-3.5 h-3.5" />
                  </button>

                  <button
                    onClick={() => onDeleteRule(rule.id)}
                    className="p-1 text-ink-400 hover:text-signal-red hover:bg-signal-red-dim rounded-sm transition-colors cursor-pointer"
                    title="Delete Rule"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Add Application Modal */}
      <AddApplicationModal
        isOpen={isAddModalOpen}
        existingRules={rules}
        onClose={() => setIsAddModalOpen(false)}
        onAddRule={onAddRule}
        onEditExisting={(existing) => {
          setIsAddModalOpen(false);
          setEditingRule(existing);
        }}
      />

      {/* Edit Application Modal */}
      <EditApplicationModal
        isOpen={!!editingRule}
        rule={editingRule}
        compatibilityRules={compatibilityRules}
        onClose={() => setEditingRule(null)}
        onSaveRule={onUpdateRule}
        onDeleteRule={onDeleteRule}
      />
    </div>
  );
};
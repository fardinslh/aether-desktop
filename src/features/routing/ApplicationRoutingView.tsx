import React, { useState } from "react";
import {
  Plus,
  Search,
  Trash2,
  Zap,
  Server,
  ShieldCheck,
  Edit2,
  RefreshCw,
} from "lucide-react";
import { ApplicationRule, RouteDestination } from "../../types";
import { AppIcon } from "../../components/AppIcon";
import { AddApplicationModal } from "./AddApplicationModal";
import { EditApplicationModal } from "./EditApplicationModal";

interface ApplicationRoutingViewProps {
  rules: ApplicationRule[];
  onAddRule: (rule: ApplicationRule) => void;
  onUpdateRule: (rule: ApplicationRule) => void;
  onToggleRule: (id: string, enabled: boolean) => void;
  onDeleteRule: (id: string) => void;
  onChangeRoute: (id: string, route: RouteDestination) => void;
  isApplying?: boolean;
}

export const ApplicationRoutingView: React.FC<ApplicationRoutingViewProps> = ({
  rules,
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

  const getDestinationBadge = (dest: RouteDestination) => {
    switch (dest) {
      case "direct":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <Zap className="w-3 h-3" />
            Direct Internet
          </span>
        );
      case "secondaryProxy":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-medium bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
            <Server className="w-3 h-3" />
            Secondary Proxy (10808)
          </span>
        );
      case "aether":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-medium bg-brand-500/10 text-brand-400 border border-brand-500/20">
            <ShieldCheck className="w-3 h-3" />
            Aether Tunnel
          </span>
        );
    }
  };

  return (
    <div className="flex flex-col h-full space-y-3 px-4 py-3">
      {/* Header Bar */}
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-semibold text-zinc-100">Application Routing Rules</h2>
            {isApplying && (
              <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium bg-brand-500/20 text-brand-300 border border-brand-500/30 animate-pulse">
                <RefreshCw className="w-2.5 h-2.5 animate-spin" />
                Applying routing changes...
              </span>
            )}
          </div>
          <p className="text-xs text-zinc-400">
            Configure how each application connects: Direct, Secondary Proxy (V2Ray), or Aether Tunnel.
          </p>
        </div>

        <button
          onClick={() => setIsAddModalOpen(true)}
          className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white text-xs font-semibold shadow-md shadow-brand-900/30 transition-all active:scale-95 flex-shrink-0"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>Add Application</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 pt-1 border-b border-zinc-800 pb-2">
        <div className="flex items-center gap-1 bg-zinc-900 p-0.5 rounded-lg border border-zinc-800">
          {(
            [
              { id: "all", label: `All (${rules.length})` },
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
              className={`px-2.5 py-1 text-xs rounded-md font-medium transition-all ${
                activeTab === t.id
                  ? "bg-zinc-800 text-zinc-100 shadow-sm"
                  : "text-zinc-400 hover:text-zinc-300"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="relative w-52">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-2.5 text-zinc-500" />
          <input
            type="text"
            placeholder="Search applications..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-full pl-8 pr-3 py-1 bg-zinc-900 border border-zinc-800 rounded-lg text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-brand-500"
          />
        </div>
      </div>

      {/* Rule List Container */}
      <div className="flex-1 overflow-y-auto rounded-xl border border-zinc-800/80 bg-background-card divide-y divide-zinc-800/60 max-h-[380px]">
        {filteredRules.length === 0 ? (
          <div className="p-10 text-center text-zinc-500 text-xs flex flex-col items-center justify-center space-y-2">
            <div>No matching application rules found.</div>
            <button
              onClick={() => setIsAddModalOpen(true)}
              className="text-brand-400 hover:underline font-medium"
            >
              + Add an application
            </button>
          </div>
        ) : (
          filteredRules.map((rule) => {
            const currentDest = rule.destination || rule.route || "aether";
            const displayName = rule.displayName || rule.name || rule.processName;
            return (
              <div
                key={rule.id}
                className={`flex items-center justify-between p-3 transition-colors ${
                  rule.enabled ? "hover:bg-zinc-800/30" : "opacity-50 hover:bg-zinc-900/30"
                }`}
              >
                <div className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(e) => onToggleRule(rule.id, e.target.checked)}
                    className="w-4 h-4 rounded border-zinc-700 bg-zinc-900 text-brand-500 focus:ring-brand-500 cursor-pointer"
                    title={rule.enabled ? "Disable rule" : "Enable rule"}
                  />

                  <AppIcon
                    processName={rule.processName}
                    displayName={displayName}
                    iconBase64={rule.iconBase64}
                    size="md"
                  />

                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-zinc-200">{displayName}</span>
                      {rule.source === "preset" && (
                        <span className="text-[9px] px-1.5 py-0.2 rounded bg-zinc-800 text-zinc-400 font-mono">
                          Preset
                        </span>
                      )}
                    </div>
                    <div className="text-[11px] font-mono text-zinc-400">{rule.processName}</div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <select
                    value={currentDest}
                    onChange={(e) => onChangeRoute(rule.id, e.target.value as RouteDestination)}
                    className="bg-zinc-900 border border-zinc-800 text-zinc-300 text-xs rounded-lg px-2.5 py-1 focus:outline-none focus:border-brand-500 cursor-pointer"
                  >
                    <option value="secondaryProxy">Secondary Proxy (V2Ray)</option>
                    <option value="direct">Direct Internet</option>
                    <option value="aether">Aether Tunnel</option>
                  </select>

                  {getDestinationBadge(currentDest)}

                  <button
                    onClick={() => setEditingRule(rule)}
                    className="p-1.5 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 rounded-md transition-colors"
                    title="Change / Edit application"
                  >
                    <Edit2 className="w-3.5 h-3.5" />
                  </button>

                  <button
                    onClick={() => onDeleteRule(rule.id)}
                    className="p-1.5 text-zinc-500 hover:text-rose-400 hover:bg-rose-950/20 rounded-md transition-colors"
                    title="Remove application rule"
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
        onClose={() => setEditingRule(null)}
        onSaveRule={onUpdateRule}
        onDeleteRule={onDeleteRule}
      />
    </div>
  );
};
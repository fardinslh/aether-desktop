import React from "react";
import { Cpu, GitFork, Sliders, Terminal } from "lucide-react";

export type NavTab = "dashboard" | "routing" | "settings" | "diagnostics";

interface TabItem {
  id: NavTab;
  label: string;
  sublabel?: string;
  icon: React.ComponentType<{ className?: string }>;
  badge?: number;
}

interface NavbarProps {
  activeTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  rulesCount: number;
}

export const Navbar: React.FC<NavbarProps> = ({ activeTab, onSelectTab, rulesCount }) => {
  const tabs: TabItem[] = [
    { id: "dashboard", label: "Core & Topology", icon: Cpu },
    { id: "routing", label: "Route Matrix", icon: GitFork, badge: rulesCount },
    { id: "diagnostics", label: "Console & Logs", icon: Terminal },
    { id: "settings", label: "Engine Config", icon: Sliders },
  ];

  return (
    <nav className="flex items-center gap-1 border-b border-app-border bg-app-panel px-3 py-1.5 select-none">
      {tabs.map((tab) => {
        const Icon = tab.icon;
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            onClick={() => onSelectTab(tab.id)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-sm text-xs font-medium transition-all cursor-pointer ${
              isActive
                ? "bg-app-surface text-ink-100 border border-app-border shadow-sm"
                : "text-ink-400 hover:text-ink-200 hover:bg-app-surface/50 border border-transparent"
            }`}
          >
            <Icon className={`w-3.5 h-3.5 ${isActive ? "text-signal-cyan" : "text-ink-400"}`} />
            <span className="tracking-tight">{tab.label}</span>
            {tab.badge !== undefined && tab.badge > 0 && (
              <span
                className={`ml-0.5 px-1.5 py-0.2 rounded-xs text-[10px] font-mono ${
                  isActive
                    ? "bg-signal-cyan/20 text-signal-cyan border border-signal-cyan/30"
                    : "bg-app-inset text-ink-400 border border-app-border"
                }`}
              >
                {tab.badge}
              </span>
            )}
          </button>
        );
      })}
    </nav>
  );
};

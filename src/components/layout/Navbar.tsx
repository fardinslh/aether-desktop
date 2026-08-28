import React from "react";
import { Shield, GitFork, Settings, Activity } from "lucide-react";

export type NavTab = "dashboard" | "routing" | "settings" | "diagnostics";

interface TabItem {
  id: NavTab;
  label: string;
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
    { id: "dashboard", label: "Dashboard", icon: Shield },
    { id: "routing", label: "App Routing", icon: GitFork, badge: rulesCount },
    { id: "settings", label: "Settings", icon: Settings },
    { id: "diagnostics", label: "Diagnostics", icon: Activity },
  ];

  return (
    <nav className="flex items-center gap-1 border-b border-zinc-800/80 bg-background-subtle/60 px-4 py-2">
      {tabs.map((tab) => {
        const Icon = tab.icon;
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            onClick={() => onSelectTab(tab.id)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
              isActive
                ? "bg-brand-500/15 text-brand-400 border border-brand-500/30 shadow-[0_0_12px_rgba(99,102,241,0.15)]"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40 border border-transparent"
            }`}
          >
            <Icon className={`w-3.5 h-3.5 ${isActive ? "text-brand-400" : "text-zinc-400"}`} />
            <span>{tab.label}</span>
            {tab.badge !== undefined && tab.badge > 0 && (
              <span
                className={`ml-0.5 px-1.5 py-0.2 rounded-full text-[10px] ${
                  isActive ? "bg-brand-500/30 text-brand-200" : "bg-zinc-800 text-zinc-400"
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

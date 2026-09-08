import React from "react";
import { Cpu, GitFork, Sliders, Terminal } from "lucide-react";
import { NavTab } from "./Navbar";

interface TabItem {
  id: NavTab;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  badge?: number;
}

interface MobileNavbarProps {
  activeTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  rulesCount: number;
}

export const MobileNavbar: React.FC<MobileNavbarProps> = ({
  activeTab,
  onSelectTab,
  rulesCount,
}) => {
  const tabs: TabItem[] = [
    { id: "dashboard", label: "Dashboard", icon: Cpu },
    { id: "routing", label: "Routing", icon: GitFork, badge: rulesCount },
    { id: "diagnostics", label: "Console", icon: Terminal },
    { id: "settings", label: "Settings", icon: Sliders },
  ];

  return (
    <nav className="fixed bottom-0 left-0 right-0 z-40 bg-app-panel/95 backdrop-blur-md border-t border-app-border px-2 pt-1.5 pb-[max(0.625rem,env(safe-area-inset-bottom))] flex items-center justify-around select-none">
      {tabs.map((tab) => {
        const Icon = tab.icon;
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            onClick={() => onSelectTab(tab.id)}
            className={`flex flex-col items-center justify-center flex-1 py-1 rounded-md transition-all cursor-pointer relative ${
              isActive
                ? "text-signal-cyan font-semibold"
                : "text-ink-400 hover:text-ink-200"
            }`}
          >
            <div className="relative">
              <Icon
                className={`w-5 h-5 transition-transform ${
                  isActive ? "text-signal-cyan scale-110" : "text-ink-400"
                }`}
              />
              {tab.badge !== undefined && tab.badge > 0 && (
                <span className="absolute -top-1 -right-2 px-1 py-0.2 rounded-full text-[9px] font-mono font-bold bg-signal-cyan text-black leading-none">
                  {tab.badge}
                </span>
              )}
            </div>
            <span
              className={`text-[10px] mt-1 tracking-tight ${
                isActive ? "text-signal-cyan font-semibold" : "text-ink-400"
              }`}
            >
              {tab.label}
            </span>
          </button>
        );
      })}
    </nav>
  );
};

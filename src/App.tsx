import { useState } from "react";
import { WindowTitleBar } from "./components/layout/WindowTitleBar";
import { Navbar, NavTab } from "./components/layout/Navbar";
import { ConnectionHero } from "./features/connection/ConnectionHero";
import { StatusOverview } from "./features/connection/StatusOverview";
import { ApplicationRoutingView } from "./features/routing/ApplicationRoutingView";
import { SettingsView } from "./features/settings/SettingsView";
import { DiagnosticsView } from "./features/diagnostics/DiagnosticsView";
import { FirstRunWizard } from "./features/wizard/FirstRunWizard";
import { useAppStore } from "./stores/useAppStore";
import { ArrowRight } from "lucide-react";

export function App() {
  const {
    settings,
    connectionState,
    health,
    logs,
    isLoading,
    isApplyingRouting,
    errorDetails,
    updateSettings,
    addApplicationRule,
    updateApplicationRule,
    toggleApplicationRule,
    deleteApplicationRule,
    updateApplicationRoute,
    triggerConnect,
    triggerDisconnect,
    refreshAll,
  } = useAppStore();

  const [activeTab, setActiveTab] = useState<NavTab>("dashboard");

  if (isLoading || !settings) {
    return (
      <div className="h-screen w-screen bg-background flex items-center justify-center text-zinc-400 text-xs font-mono select-none">
        <div className="flex flex-col items-center gap-2">
          <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
          <span>INITIALIZING AETHER CORE...</span>
        </div>
      </div>
    );
  }

  const activeRulesCount = settings.applicationRules.filter((r) => r.enabled).length;

  return (
    <div className="h-screen w-screen flex flex-col bg-background text-zinc-100 overflow-hidden font-sans select-none">
      <WindowTitleBar connectionState={connectionState} />
      <Navbar activeTab={activeTab} onSelectTab={setActiveTab} rulesCount={activeRulesCount} />

      <main className="flex-1 overflow-hidden">
        {activeTab === "dashboard" && (
          <div className="h-full flex flex-col justify-between overflow-y-auto pb-4">
            <ConnectionHero
              connectionState={connectionState}
              health={health}
              onConnect={triggerConnect}
              onDisconnect={triggerDisconnect}
              onViewDiagnostics={() => setActiveTab("diagnostics")}
              errorDetails={errorDetails}
            />

            <StatusOverview health={health} connectionState={connectionState} />

            <div className="px-4 mt-3">
              <div className="p-3 rounded-xl bg-background-card border border-zinc-800/80 flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-1.5 text-xs">
                    <span className="w-2 h-2 rounded-full bg-emerald-400" />
                    <span className="text-zinc-400">Direct:</span>
                    <span className="font-semibold text-zinc-200">
                      {settings.applicationRules.filter((r) => (r.destination || r.route) === "direct" && r.enabled).length} apps
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5 text-xs">
                    <span className="w-2 h-2 rounded-full bg-cyan-400" />
                    <span className="text-zinc-400">Secondary:</span>
                    <span className="font-semibold text-zinc-200">
                      {settings.applicationRules.filter((r) => (r.destination || r.route) === "secondaryProxy" && r.enabled).length} apps
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5 text-xs">
                    <span className="w-2 h-2 rounded-full bg-brand-400" />
                    <span className="text-zinc-400">Aether:</span>
                    <span className="font-semibold text-zinc-200">All other traffic</span>
                  </div>
                </div>

                <button
                  onClick={() => setActiveTab("routing")}
                  className="flex items-center gap-1 text-xs font-semibold text-brand-400 hover:text-brand-300 transition-colors"
                >
                  <span>Manage Rules</span>
                  <ArrowRight className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        )}

        {activeTab === "routing" && (
          <ApplicationRoutingView
            rules={settings.applicationRules}
            onAddRule={addApplicationRule}
            onUpdateRule={updateApplicationRule}
            onToggleRule={toggleApplicationRule}
            onDeleteRule={deleteApplicationRule}
            onChangeRoute={updateApplicationRoute}
            isApplying={isApplyingRouting}
          />
        )}

        {activeTab === "settings" && (
          <SettingsView
            settings={settings}
            onSave={updateSettings}
            onReset={async () => {
              await refreshAll();
            }}
          />
        )}

        {activeTab === "diagnostics" && (
          <DiagnosticsView logs={logs} onRefreshLogs={refreshAll} />
        )}
      </main>

      {!settings.firstRunCompleted && (
        <FirstRunWizard
          settings={settings}
          onComplete={async (updated) => {
            await updateSettings(updated);
          }}
        />
      )}
    </div>
  );
}

export default App;
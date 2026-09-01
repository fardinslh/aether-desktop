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
    triggerCancel,
    triggerFindFasterGateway,
    triggerDisconnect,
    refreshAll,
  } = useAppStore();

  const [activeTab, setActiveTab] = useState<NavTab>("dashboard");

  if (isLoading || !settings) {
    return (
      <div className="h-screen w-screen bg-app-bg flex items-center justify-center text-ink-400 text-xs font-mono select-none">
        <div className="flex flex-col items-center gap-2">
          <div className="w-5 h-5 border-2 border-signal-cyan border-t-transparent rounded-full animate-spin" />
          <span className="tracking-widest">INITIALIZING AETHER ENGINE...</span>
        </div>
      </div>
    );
  }

  const activeRulesCount = settings.applicationRules.filter((r) => r.enabled).length;

  return (
    <div className="h-screen w-screen flex flex-col bg-app-bg text-ink-200 overflow-hidden font-sans select-none">
      <WindowTitleBar connectionState={connectionState} />
      <Navbar activeTab={activeTab} onSelectTab={setActiveTab} rulesCount={activeRulesCount} />

      <main className="flex-1 overflow-hidden">
        {activeTab === "dashboard" && (
          <div className="h-full flex flex-col justify-between overflow-y-auto pb-3">
            <ConnectionHero
              connectionState={connectionState}
              health={health}
              onConnect={triggerConnect}
              onCancel={triggerCancel}
              onFindFasterGateway={triggerFindFasterGateway}
              onDisconnect={triggerDisconnect}
              onViewDiagnostics={() => setActiveTab("diagnostics")}
              errorDetails={errorDetails}
            />

            <StatusOverview health={health} connectionState={connectionState} />

            <div className="px-4 mt-2">
              <div className="p-2.5 rounded-md bg-app-panel border border-app-border flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-1.5 text-xs font-mono">
                    <span className="w-2 h-2 rounded-full bg-signal-cyan" />
                    <span className="text-ink-400">Direct:</span>
                    <span className="font-semibold text-ink-200">
                      {settings.applicationRules.filter((r) => (r.destination || r.route) === "direct" && r.enabled).length} apps
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5 text-xs font-mono">
                    <span className="w-2 h-2 rounded-full bg-signal-amber" />
                    <span className="text-ink-400">Secondary:</span>
                    <span className="font-semibold text-ink-200">
                      {settings.applicationRules.filter((r) => (r.destination || r.route) === "secondaryProxy" && r.enabled).length} apps
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5 text-xs font-mono">
                    <span className="w-2 h-2 rounded-full bg-signal-green" />
                    <span className="text-ink-400">Aether:</span>
                    <span className="font-semibold text-ink-200">Global Fallback</span>
                  </div>
                </div>

                <button
                  onClick={() => setActiveTab("routing")}
                  className="flex items-center gap-1 text-xs font-semibold text-signal-cyan hover:text-signal-cyan-muted transition-colors cursor-pointer"
                >
                  <span>Route Matrix</span>
                  <ArrowRight className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        )}

        {activeTab === "routing" && (
          <ApplicationRoutingView
            rules={settings.applicationRules}
            compatibilityRules={settings.compatibility.customCompatibilityRules}
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
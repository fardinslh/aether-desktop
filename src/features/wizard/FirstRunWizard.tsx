import React, { useState, useEffect } from "react";
import { ShieldCheck, Radio, Shield, Network, ArrowRight, CheckCircle2, AlertCircle } from "lucide-react";
import { AppSettings, BinaryValidationResult } from "../../types";
import { api } from "../../services/api";

interface FirstRunWizardProps {
  settings: AppSettings;
  onComplete: (updatedSettings: AppSettings) => void;
}

export const FirstRunWizard: React.FC<FirstRunWizardProps> = ({ settings, onComplete }) => {
  const [step, setStep] = useState<number>(1);
  const [validation, setValidation] = useState<BinaryValidationResult | null>(null);
  const [aetherPath, setAetherPath] = useState<string>(settings.aether.executablePath);
  const [singboxPath, setSingboxPath] = useState<string>(settings.singBox.executablePath);
  const [secondaryHost, setSecondaryHost] = useState<string>(settings.secondaryProxy.host);
  const [secondaryPort, setSecondaryPort] = useState<number>(settings.secondaryProxy.port);
  const [secondaryEnabled, setSecondaryEnabled] = useState<boolean>(settings.secondaryProxy.enabled);

  useEffect(() => {
    api.validateBinaries().then(setValidation);
  }, []);

  const handleFinish = () => {
    const updated: AppSettings = {
      ...settings,
      aether: { ...settings.aether, executablePath: aetherPath },
      singBox: { ...settings.singBox, executablePath: singboxPath },
      secondaryProxy: {
        ...settings.secondaryProxy,
        enabled: secondaryEnabled,
        host: secondaryHost,
        port: secondaryPort,
      },
      firstRunCompleted: true,
    };
    onComplete(updated);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background p-6 select-none">
      <div className="w-full max-w-xl rounded-2xl bg-zinc-900 border border-zinc-800 shadow-2xl p-6 flex flex-col space-y-5">
        {/* Step Indicator */}
        <div className="flex items-center justify-between border-b border-zinc-800 pb-4">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded-lg bg-brand-600/20 text-brand-400 flex items-center justify-center text-xs font-bold">
              {step}
            </div>
            <span className="text-xs font-semibold text-zinc-200">
              {step === 1 && "Welcome to Aether Desktop"}
              {step === 2 && "Configure Aether Tunnel"}
              {step === 3 && "Configure sing-box Router"}
              {step === 4 && "Optional Secondary Proxy (V2Ray)"}
              {step === 5 && "Ready to Connect"}
            </span>
          </div>
          <span className="text-xs text-zinc-500 font-mono">Step {step} of 5</span>
        </div>

        {/* Step Contents */}
        <div className="min-h-[220px] flex flex-col justify-center">
          {step === 1 && (
            <div className="space-y-3 text-center py-4">
              <div className="w-16 h-16 rounded-2xl bg-brand-600/20 text-brand-400 flex items-center justify-center mx-auto mb-3">
                <ShieldCheck className="w-8 h-8" />
              </div>
              <h2 className="text-lg font-bold text-zinc-100">Welcome to Aether Desktop</h2>
              <p className="text-xs text-zinc-400 max-w-md mx-auto leading-relaxed">
                Aether Desktop unifies your Aether proxy and sing-box TUN router into a seamless one-click Windows VPN with automatic application-level routing.
              </p>
            </div>
          )}

          {step === 2 && (
            <div className="space-y-4">
              <div className="flex items-center gap-2.5">
                <Radio className="w-5 h-5 text-brand-400" />
                <div>
                  <div className="text-xs font-bold text-zinc-100">Aether Executable</div>
                  <div className="text-[11px] text-zinc-400">Primary tunnel process responsible for global encrypted traffic</div>
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">Path to aether.exe</label>
                <input
                  type="text"
                  value={aetherPath}
                  onChange={(e) => setAetherPath(e.target.value)}
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>

              <div className="flex items-center gap-2 text-xs">
                {validation?.aetherExists ? (
                  <span className="flex items-center gap-1 text-emerald-400">
                    <CheckCircle2 className="w-3.5 h-3.5" /> aether.exe detected
                  </span>
                ) : (
                  <span className="flex items-center gap-1 text-amber-400">
                    <AlertCircle className="w-3.5 h-3.5" /> File not found at path (can be updated later in Settings)
                  </span>
                )}
              </div>
            </div>
          )}

          {step === 3 && (
            <div className="space-y-4">
              <div className="flex items-center gap-2.5">
                <Shield className="w-5 h-5 text-emerald-400" />
                <div>
                  <div className="text-xs font-bold text-zinc-100">sing-box TUN Router</div>
                  <div className="text-[11px] text-zinc-400">System router creating the virtual TUN adapter with elevated rules</div>
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold text-zinc-300 mb-1">Path to sing-box.exe</label>
                <input
                  type="text"
                  value={singboxPath}
                  onChange={(e) => setSingboxPath(e.target.value)}
                  className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                />
              </div>

              <div className="flex items-center gap-2 text-xs">
                {validation?.singboxExists ? (
                  <span className="flex items-center gap-1 text-emerald-400">
                    <CheckCircle2 className="w-3.5 h-3.5" /> sing-box.exe detected
                  </span>
                ) : (
                  <span className="flex items-center gap-1 text-amber-400">
                    <AlertCircle className="w-3.5 h-3.5" /> File not found at path (can be updated later in Settings)
                  </span>
                )}
              </div>
            </div>
          )}

          {step === 4 && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                  <Network className="w-5 h-5 text-cyan-400" />
                  <div>
                    <div className="text-xs font-bold text-zinc-100">Secondary Proxy (v2rayN/Xray)</div>
                    <div className="text-[11px] text-zinc-400">Optional SOCKS endpoint for Antigravity, ChatGPT, and VS Code</div>
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={secondaryEnabled}
                  onChange={(e) => setSecondaryEnabled(e.target.checked)}
                  className="w-4 h-4 rounded text-brand-500 focus:ring-brand-500 bg-zinc-950 border-zinc-700 cursor-pointer"
                />
              </div>

              <div className="grid grid-cols-2 gap-3 pt-1">
                <div>
                  <label className="block text-xs font-semibold text-zinc-300 mb-1">Host</label>
                  <input
                    type="text"
                    value={secondaryHost}
                    onChange={(e) => setSecondaryHost(e.target.value)}
                    className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-semibold text-zinc-300 mb-1">Port</label>
                  <input
                    type="number"
                    value={secondaryPort}
                    onChange={(e) => setSecondaryPort(Number(e.target.value))}
                    className="w-full px-3 py-1.5 bg-zinc-950 border border-zinc-800 rounded-lg text-xs text-zinc-200 font-mono focus:outline-none focus:border-brand-500"
                  />
                </div>
              </div>
            </div>
          )}

          {step === 5 && (
            <div className="space-y-3 text-center py-4">
              <div className="w-16 h-16 rounded-2xl bg-emerald-600/20 text-emerald-400 flex items-center justify-center mx-auto mb-3">
                <CheckCircle2 className="w-8 h-8" />
              </div>
              <h2 className="text-lg font-bold text-zinc-100">Setup Complete!</h2>
              <p className="text-xs text-zinc-400 max-w-md mx-auto leading-relaxed">
                Your presets and loop-prevention rules are ready. Press Connect to establish your secured network tunnel.
              </p>
            </div>
          )}
        </div>

        {/* Footer Navigation */}
        <div className="flex items-center justify-between border-t border-zinc-800 pt-4">
          {step > 1 ? (
            <button
              onClick={() => setStep(step - 1)}
              className="px-4 py-1.5 rounded-lg border border-zinc-800 text-xs font-medium text-zinc-300 hover:bg-zinc-800 transition-colors"
            >
              Back
            </button>
          ) : (
            <div />
          )}

          {step < 5 ? (
            <button
              onClick={() => setStep(step + 1)}
              className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white text-xs font-semibold shadow-md shadow-brand-900/30 transition-all active:scale-95"
            >
              <span>Next</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          ) : (
            <button
              onClick={handleFinish}
              className="flex items-center gap-1.5 px-5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-bold shadow-md shadow-emerald-900/30 transition-all active:scale-95"
            >
              <span>Finish & Open App</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

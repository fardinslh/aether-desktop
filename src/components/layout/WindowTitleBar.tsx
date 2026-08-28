import React, { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, Copy, X, Shield, ShieldCheck, ShieldAlert } from "lucide-react";
import { ConnectionState } from "../../types";

interface WindowTitleBarProps {
  connectionState: ConnectionState;
}

export const WindowTitleBar: React.FC<WindowTitleBarProps> = ({ connectionState }) => {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        const appWindow = getCurrentWindow();
        const initialMax = await appWindow.isMaximized();
        setIsMaximized(initialMax);

        unlisten = await appWindow.onResized(async () => {
          try {
            const max = await appWindow.isMaximized();
            setIsMaximized(max);
          } catch (e) {
            console.error("Failed to query maximized state on resize:", e);
          }
        });
      } catch (err) {
        console.warn("Window event listener not available (browser dev mode):", err);
      }
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleMinimize = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const appWindow = getCurrentWindow();
      await appWindow.minimize();
    } catch (err) {
      console.error("Failed to minimize window:", err);
    }
  };

  const handleToggleMaximize = async (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    try {
      const appWindow = getCurrentWindow();
      await appWindow.toggleMaximize();
      const max = await appWindow.isMaximized();
      setIsMaximized(max);
    } catch (err) {
      console.error("Failed to toggle maximize window:", err);
    }
  };

  const handleClose = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const appWindow = getCurrentWindow();
      await appWindow.close();
    } catch (err) {
      console.error("Failed to close window:", err);
    }
  };

  const handleDoubleClick = (e: React.MouseEvent) => {
    // Only toggle if clicked directly on draggable region, not on buttons
    const target = e.target as HTMLElement;
    if (target.closest("button") || target.closest("[data-no-drag]")) {
      return;
    }
    handleToggleMaximize();
  };

  const isConnected = connectionState === "CONNECTED";
  const isError = connectionState === "ERROR";
  const isTransitioning =
    connectionState !== "CONNECTED" && connectionState !== "DISCONNECTED" && connectionState !== "ERROR";

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={handleDoubleClick}
      className="h-10 bg-background-subtle border-b border-zinc-800/80 flex items-center justify-between px-3 select-none z-50 sticky top-0 cursor-default"
    >
      {/* Brand & Connection State Indicator */}
      <div className="flex items-center gap-2.5 pointer-events-none" data-tauri-drag-region>
        <div className="flex items-center justify-center w-5 h-5 rounded bg-brand-600/20 text-brand-400">
          {isConnected ? (
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
          ) : isError ? (
            <ShieldAlert className="w-3.5 h-3.5 text-rose-400" />
          ) : (
            <Shield className="w-3.5 h-3.5 text-brand-400" />
          )}
        </div>
        <span className="font-semibold text-xs text-zinc-200 tracking-wide">
          AETHER <span className="text-zinc-500 font-normal">DESKTOP</span>
        </span>

        {/* Small live badge in titlebar */}
        <div className="flex items-center gap-1.5 ml-2 px-2 py-0.5 rounded-full bg-zinc-900 border border-zinc-800 text-[10px]">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              isConnected
                ? "bg-emerald-400 shadow-[0_0_8px_#34d399]"
                : isTransitioning
                ? "bg-amber-400 animate-pulse"
                : isError
                ? "bg-rose-500"
                : "bg-zinc-600"
            }`}
          />
          <span className="text-zinc-400 font-medium capitalize">
            {isConnected ? "Secure" : isTransitioning ? "Connecting" : isError ? "Alert" : "Offline"}
          </span>
        </div>
      </div>

      {/* Window Controls (Explicitly excluded from drag region) */}
      <div className="flex items-center -mr-1" data-no-drag>
        <button
          onClick={handleMinimize}
          className="w-8 h-8 flex items-center justify-center text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/60 rounded transition-colors"
          title="Minimize"
        >
          <Minus className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={handleToggleMaximize}
          className="w-8 h-8 flex items-center justify-center text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/60 rounded transition-colors"
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? <Copy className="w-3 h-3 rotate-180" /> : <Square className="w-3 h-3" />}
        </button>
        <button
          onClick={handleClose}
          className="w-8 h-8 flex items-center justify-center text-zinc-400 hover:text-white hover:bg-rose-600/90 rounded transition-colors"
          title="Close"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
    </header>
  );
};
import React, { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, Copy, X } from "lucide-react";
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
      className="h-9 bg-app-panel border-b border-app-border flex items-center justify-between px-3 select-none z-50 sticky top-0 cursor-default"
    >
      {/* Brand & Connection State Indicator */}
      <div className="flex items-center gap-2.5 pointer-events-none" data-tauri-drag-region>
        {/* Precision Brand Icon */}
        <div className="flex items-center justify-center w-4 h-4 flex-shrink-0">
          <svg viewBox="0 0 1024 1024" className="w-4 h-4" fill="none" xmlns="http://www.w3.org/2000/svg">
            <defs>
              <linearGradient id="tbShieldBg" x1="512" y1="112" x2="512" y2="912" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stopColor="#1e2330" />
                <stop offset="100%" stopColor="#0c0e12" />
              </linearGradient>
            </defs>
            <path
              d="M 512 112 C 648 112, 792 180, 836 244 C 848 420, 810 656, 512 912 C 214 656, 176 420, 188 244 C 232 180, 376 112, 512 112 Z"
              fill="url(#tbShieldBg)"
              stroke={isConnected ? "#10b981" : isError ? "#ef4444" : "#00d2ff"}
              strokeWidth="56"
              strokeLinejoin="round"
            />
            <circle cx="512" cy="512" r="130" fill="#090b0e" stroke={isConnected ? "#10b981" : "#00d2ff"} strokeWidth="40" />
            <circle cx="512" cy="512" r="60" fill={isConnected ? "#10b981" : isError ? "#ef4444" : "#00d2ff"} />
          </svg>
        </div>

        <div className="flex items-center gap-1.5 text-xs font-semibold tracking-wide text-ink-100 font-sans">
          <span>AETHER</span>
          <span className="text-ink-400 font-normal text-[11px]">DESKTOP</span>
        </div>

        {/* Live status badge */}
        <div className="flex items-center gap-1.5 ml-2 px-2 py-0.5 rounded-sm bg-app-inset border border-app-border text-[10px] font-mono">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              isConnected
                ? "bg-signal-green shadow-[0_0_6px_#10b981]"
                : isTransitioning
                ? "bg-signal-cyan animate-pulse"
                : isError
                ? "bg-signal-red"
                : "bg-ink-500"
            }`}
          />
          <span className="text-ink-300 uppercase tracking-wider">
            {isConnected ? "Connected" : isTransitioning ? "Routing" : isError ? "Alert" : "Standby"}
          </span>
        </div>
      </div>

      {/* Window Controls */}
      <div className="flex items-center -mr-1" data-no-drag>
        <button
          onClick={handleMinimize}
          className="w-7 h-7 flex items-center justify-center text-ink-400 hover:text-ink-100 hover:bg-app-surface rounded-sm transition-colors cursor-pointer"
          title="Minimize"
        >
          <Minus className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={handleToggleMaximize}
          className="w-7 h-7 flex items-center justify-center text-ink-400 hover:text-ink-100 hover:bg-app-surface rounded-sm transition-colors cursor-pointer"
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? <Copy className="w-3 h-3 rotate-180" /> : <Square className="w-3 h-3" />}
        </button>
        <button
          onClick={handleClose}
          className="w-7 h-7 flex items-center justify-center text-ink-400 hover:text-white hover:bg-rose-600/90 rounded-sm transition-colors cursor-pointer"
          title="Close"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
    </header>
  );
};
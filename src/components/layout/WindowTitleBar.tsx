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
      className="h-10 bg-background-subtle border-b border-zinc-800/80 flex items-center justify-between px-3 select-none z-50 sticky top-0 cursor-default"
    >
      {/* Brand & Connection State Indicator */}
      <div className="flex items-center gap-2.5 pointer-events-none" data-tauri-drag-region>
        {/* Original Aether Desktop Brand Mark */}
        <div className="flex items-center justify-center w-5 h-5 flex-shrink-0">
          <svg viewBox="0 0 1024 1024" className="w-5 h-5" fill="none" xmlns="http://www.w3.org/2000/svg">
            <defs>
              <linearGradient id="tbShieldBg" x1="512" y1="112" x2="512" y2="912" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stop-color="#1E1B4B" />
                <stop offset="100%" stop-color="#090D16" />
              </linearGradient>
              <linearGradient id="tbBorder" x1="188" y1="112" x2="836" y2="912" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stop-color={isConnected ? "#10B981" : isError ? "#F43F5E" : "#6366F1"} />
                <stop offset="100%" stop-color={isConnected ? "#06B6D4" : isError ? "#FB7185" : "#06B6D4"} />
              </linearGradient>
              <linearGradient id="tbRoute1" x1="280" y1="340" x2="744" y2="340" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stop-color="#818CF8" />
                <stop offset="100%" stop-color="#A855F7" />
              </linearGradient>
              <linearGradient id="tbRoute2" x1="744" y1="480" x2="340" y2="700" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stop-color="#38BDF8" />
                <stop offset="100%" stop-color="#06B6D4" />
              </linearGradient>
            </defs>
            <path
              d="M 512 112 C 648 112, 792 180, 836 244 C 848 420, 810 656, 512 912 C 214 656, 176 420, 188 244 C 232 180, 376 112, 512 112 Z"
              fill="url(#tbShieldBg)"
              stroke="url(#tbBorder)"
              strokeWidth="48"
              strokeLinejoin="round"
            />
            <path d="M 280 340 C 380 300, 440 420, 512 512 C 584 604, 644 420, 744 340" fill="none" stroke="url(#tbRoute1)" strokeWidth="64" strokeLinecap="round" />
            <path d="M 744 480 C 640 480, 580 512, 512 512 C 430 512, 380 620, 340 700" fill="none" stroke="url(#tbRoute2)" strokeWidth="64" strokeLinecap="round" />
            <circle cx="512" cy="512" r="110" fill="#080D1A" stroke="#06B6D4" strokeWidth="28" />
            <circle cx="512" cy="512" r="64" fill={isConnected ? "#34D399" : isError ? "#F43F5E" : "#38BDF8"} />
            <circle cx="512" cy="512" r="28" fill="#FFFFFF" />
          </svg>
        </div>

        <span className="font-bold text-xs text-zinc-200 tracking-wider">
          AETHER <span className="text-zinc-500 font-semibold">DESKTOP</span>
        </span>

        {/* Live badge in titlebar */}
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

      {/* Window Controls */}
      <div className="flex items-center -mr-1" data-no-drag>
        <button
          onClick={handleMinimize}
          className="w-8 h-8 flex items-center justify-center text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/60 rounded transition-colors cursor-pointer"
          title="Minimize"
        >
          <Minus className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={handleToggleMaximize}
          className="w-8 h-8 flex items-center justify-center text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/60 rounded transition-colors cursor-pointer"
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? <Copy className="w-3 h-3 rotate-180" /> : <Square className="w-3 h-3" />}
        </button>
        <button
          onClick={handleClose}
          className="w-8 h-8 flex items-center justify-center text-zinc-400 hover:text-white hover:bg-rose-600/90 rounded transition-colors cursor-pointer"
          title="Close"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
    </header>
  );
};
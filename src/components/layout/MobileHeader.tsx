import React from "react";
import { ConnectionState } from "../../types";

interface MobileHeaderProps {
  connectionState: ConnectionState;
}

export const MobileHeader: React.FC<MobileHeaderProps> = ({ connectionState }) => {
  const isConnected = connectionState === "CONNECTED";
  const isError = connectionState === "ERROR";
  const isTransitioning =
    connectionState !== "CONNECTED" &&
    connectionState !== "DISCONNECTED" &&
    connectionState !== "ERROR";

  return (
    <header className="sticky top-0 z-40 bg-app-panel/95 backdrop-blur-md border-b border-app-border px-4 py-2.5 pt-[max(0.625rem,env(safe-area-inset-top))] flex items-center justify-between select-none">
      {/* Brand & Logo */}
      <div className="flex items-center gap-2">
        <div className="flex items-center justify-center w-5 h-5 flex-shrink-0">
          <svg viewBox="0 0 1024 1024" className="w-5 h-5" fill="none" xmlns="http://www.w3.org/2000/svg">
            <defs>
              <linearGradient id="mShieldBg" x1="512" y1="112" x2="512" y2="912" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stopColor="#1e2330" />
                <stop offset="100%" stopColor="#0a0c10" />
              </linearGradient>
              <linearGradient id="mShieldBorder" x1="512" y1="112" x2="512" y2="912" gradientUnits="userSpaceOnUse">
                <stop offset="0%" stopColor="#00d2ff" />
                <stop offset="50%" stopColor="#0066ff" />
                <stop offset="100%" stopColor="#10b981" />
              </linearGradient>
            </defs>
            <path
              d="M512 112L192 256v304c0 238.4 136.4 461.6 320 512 183.6-50.4 320-273.6 320-512V256L512 112z"
              fill="url(#mShieldBg)"
              stroke="url(#mShieldBorder)"
              strokeWidth="48"
              strokeLinejoin="round"
            />
            <path
              d="M512 300L350 430l70 70 92-74 92 74 70-70-162-130z"
              fill="#00d2ff"
            />
            <circle cx="512" cy="620" r="48" fill="#10b981" />
          </svg>
        </div>
        <div className="flex items-baseline gap-1.5">
          <span className="font-mono font-bold text-sm tracking-wider text-ink-100">
            AETHER
          </span>
          <span className="text-[10px] font-mono px-1 py-0.2 rounded-xs bg-signal-cyan-dim border border-signal-cyan/30 text-signal-cyan">
            MOBILE
          </span>
        </div>
      </div>

      {/* Connection State Badge */}
      <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-app-inset border border-app-border text-[11px] font-mono">
        <span
          className={`w-2 h-2 rounded-full ${
            isConnected
              ? "bg-signal-green shadow-[0_0_6px_#10b981]"
              : isTransitioning
              ? "bg-signal-cyan animate-pulse shadow-[0_0_6px_#00d2ff]"
              : isError
              ? "bg-signal-red shadow-[0_0_6px_#ef4444]"
              : "bg-ink-500"
          }`}
        />
        <span
          className={`font-semibold tracking-wide ${
            isConnected
              ? "text-signal-green"
              : isTransitioning
              ? "text-signal-cyan"
              : isError
              ? "text-signal-red"
              : "text-ink-400"
          }`}
        >
          {isConnected
            ? "CONNECTED"
            : isTransitioning
            ? "CONNECTING..."
            : isError
            ? "ERROR"
            : "STANDBY"}
        </span>
      </div>
    </header>
  );
};

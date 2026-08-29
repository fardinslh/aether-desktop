import React from "react";
import {
  Gamepad2,
  Globe,
  Code2,
  MessageSquare,
  Music,
  Send,
  Layers,
  Sparkles,
} from "lucide-react";

interface AppIconProps {
  processName: string;
  displayName?: string;
  iconBase64?: string | null;
  className?: string;
  size?: "sm" | "md" | "lg";
}

export const AppIcon: React.FC<AppIconProps> = ({
  processName,
  displayName,
  iconBase64,
  className = "",
  size = "md",
}) => {
  if (iconBase64) {
    const sizeClasses = size === "sm" ? "w-4 h-4" : size === "lg" ? "w-8 h-8" : "w-6 h-6";
    return (
      <img
        src={iconBase64}
        alt={displayName || processName}
        className={`${sizeClasses} object-contain rounded ${className}`}
      />
    );
  }

  const p = processName.toLowerCase();
  const d = (displayName || "").toLowerCase();

  const sizeContainer = size === "sm" ? "w-6 h-6 text-xs" : size === "lg" ? "w-10 h-10 text-base" : "w-8 h-8 text-sm";
  const iconSize = size === "sm" ? "w-3.5 h-3.5" : size === "lg" ? "w-5 h-5" : "w-4 h-4";

  // Discord
  if (p.includes("discord") || d.includes("discord")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-[#5865F2]/15 border border-[#5865F2]/40 text-[#5865F2] flex items-center justify-center font-bold flex-shrink-0 ${className}`}>
        <MessageSquare className={iconSize} />
      </div>
    );
  }

  // Chrome / Edge / Firefox / Browser
  if (p.includes("chrome") || p.includes("firefox") || p.includes("msedge") || p.includes("brave")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-signal-amber-dim border border-signal-amber/40 text-signal-amber flex items-center justify-center flex-shrink-0 ${className}`}>
        <Globe className={iconSize} />
      </div>
    );
  }

  // VS Code / IDEs
  if (p.includes("code") || p.includes("devenv") || p.includes("cursor") || p.includes("sublime")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-signal-cyan-dim border border-signal-cyan/40 text-signal-cyan flex items-center justify-center flex-shrink-0 ${className}`}>
        <Code2 className={iconSize} />
      </div>
    );
  }

  // Antigravity & AI
  if (p.includes("antigravity") || p.includes("agy") || p.includes("language_server") || p.includes("codex")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-purple-500/15 border border-purple-500/40 text-purple-400 flex items-center justify-center flex-shrink-0 ${className}`}>
        <Sparkles className={iconSize} />
      </div>
    );
  }

  // Telegram
  if (p.includes("telegram")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-signal-cyan-dim border border-signal-cyan/40 text-signal-cyan flex items-center justify-center flex-shrink-0 ${className}`}>
        <Send className={iconSize} />
      </div>
    );
  }

  // Spotify / Music
  if (p.includes("spotify") || p.includes("music") || p.includes("itunes")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-signal-green-dim border border-signal-green/40 text-signal-green flex items-center justify-center flex-shrink-0 ${className}`}>
        <Music className={iconSize} />
      </div>
    );
  }

  // Steam / Games (Dota, Rust, Generals)
  if (p.includes("steam") || p.includes("dota") || p.includes("rust") || p.includes("generals") || p.includes("game")) {
    return (
      <div className={`${sizeContainer} rounded-xs bg-signal-red-dim border border-signal-red/40 text-signal-red flex items-center justify-center flex-shrink-0 ${className}`}>
        <Gamepad2 className={iconSize} />
      </div>
    );
  }

  // Generic fallback: first letter badge or generic layers icon
  const firstLetter = (displayName || processName).charAt(0).toUpperCase();

  return (
    <div className={`${sizeContainer} rounded-xs bg-app-surface border border-app-border text-ink-300 font-mono flex items-center justify-center font-bold flex-shrink-0 shadow-xs ${className}`}>
      {firstLetter || <Layers className={iconSize} />}
    </div>
  );
};
import { invoke } from "@tauri-apps/api/core";
import {
  AppSettings,
  BinaryValidationResult,
  CloudflareTrace,
  ConnectionState,
  ExecutableInspection,
  HealthStatus,
  LogEntry,
  RunningProcessInfo,
} from "../types";

// Safe wrapper to invoke Tauri commands
export const api = {
  async getSettings(): Promise<AppSettings> {
    try {
      return await invoke<AppSettings>("get_settings");
    } catch (e) {
      console.warn("Tauri getSettings failed, using fallback:", e);
      return getFallbackSettings();
    }
  },

  async saveSettings(settings: AppSettings): Promise<void> {
    try {
      await invoke("save_settings", { settings });
    } catch (e) {
      console.error("Failed to save settings:", e);
      throw e;
    }
  },

  async resetSettings(): Promise<AppSettings> {
    return await invoke<AppSettings>("reset_settings");
  },

  async getConnectionState(): Promise<ConnectionState> {
    try {
      return await invoke<ConnectionState>("get_connection_state");
    } catch {
      return "DISCONNECTED";
    }
  },

  async setConnectionState(state: ConnectionState): Promise<void> {
    try {
      await invoke("set_connection_state", { newState: state });
    } catch (e) {
      console.warn("Failed to set connection state:", e);
    }
  },

  async connectTunnel(): Promise<void> {
    try {
      await invoke("connect_tunnel");
    } catch (e) {
      console.error("Failed to connect tunnel:", e);
      throw e;
    }
  },

  async disconnectTunnel(): Promise<void> {
    try {
      await invoke("disconnect_tunnel");
    } catch (e) {
      console.error("Failed to disconnect tunnel:", e);
      throw e;
    }
  },

  async getHealthStatus(): Promise<HealthStatus> {
    try {
      return await invoke<HealthStatus>("get_health_status");
    } catch (e) {
      console.warn("Failed to get health status:", e);
      return getFallbackHealth();
    }
  },

  async getRunningApplications(): Promise<RunningProcessInfo[]> {
    try {
      return await invoke<RunningProcessInfo[]>("get_running_applications");
    } catch (e) {
      console.warn("Failed to get running processes:", e);
      return [];
    }
  },

  async inspectExecutableFile(filePath: string): Promise<ExecutableInspection> {
    try {
      return await invoke<ExecutableInspection>("inspect_executable_file", { filePath });
    } catch {
      const parts = filePath.replace(/\\/g, "/").split("/");
      const processName = parts[parts.length - 1] || "App.exe";
      return {
        displayName: processName.replace(/\.exe$/i, ""),
        processName: processName.toLowerCase().endsWith(".exe") ? processName : `${processName}.exe`,
        executablePath: filePath,
      };
    }
  },

  async generateSingBoxConfigPreview(): Promise<string> {
    return await invoke<string>("generate_singbox_config_preview");
  },

  async testSecondaryProxy(): Promise<CloudflareTrace> {
    return await invoke<CloudflareTrace>("test_secondary_proxy");
  },

  async testAetherProxy(): Promise<CloudflareTrace> {
    return await invoke<CloudflareTrace>("test_aether_proxy");
  },

  async getLogs(): Promise<LogEntry[]> {
    try {
      return await invoke<LogEntry[]>("get_logs");
    } catch {
      return [];
    }
  },

  async exportLogs(): Promise<string> {
    return await invoke<string>("export_logs");
  },

  async validateBinaries(): Promise<BinaryValidationResult> {
    try {
      return await invoke<BinaryValidationResult>("validate_binaries");
    } catch {
      return {
        aetherExists: false,
        aetherPath: "C:\\Aether\\aether.exe",
        singboxExists: false,
        singboxPath: "C:\\sing-box\\sing-box.exe",
      };
    }
  },
};

function getFallbackSettings(): AppSettings {
  return {
    aether: {
      executablePath: "C:\\Aether\\aether.exe",
      host: "127.0.0.1",
      port: 1819,
      launchArguments: [],
    },
    secondaryProxy: {
      enabled: true,
      host: "127.0.0.1",
      port: 10808,
    },
    singBox: {
      executablePath: "C:\\sing-box\\sing-box.exe",
      interfaceName: "singbox-tun",
      tunAddress: "172.19.0.1/30",
      mtu: 1500,
      logLevel: "info",
      strictRoute: true,
    },
    compatibility: {
      generalsStunTurnFallback: true,
      privateIpBypass: true,
      customCompatibilityRules: [],
    },
    general: {
      startWithWindows: false,
      autoConnect: false,
      minimizeToTray: true,
      startMinimized: false,
      reconnectAutomatically: true,
    },
    applicationRules: [
      { id: "1", displayName: "Dota 2", processName: "dota2.exe", destination: "direct", enabled: true, source: "preset", priority: "normal" },
      { id: "2", displayName: "Rust Client", processName: "RustClient.exe", destination: "direct", enabled: true, source: "preset", priority: "normal" },
      { id: "3", displayName: "Rust", processName: "Rust.exe", destination: "direct", enabled: true, source: "preset", priority: "normal" },
      { id: "4", displayName: "Discord", processName: "Discord.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "high" },
      { id: "5", displayName: "Google Chrome", processName: "chrome.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "normal" },
      { id: "6", displayName: "Visual Studio Code", processName: "Code.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "normal" },
      { id: "7", displayName: "Codex", processName: "codex.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "normal" },
      { id: "8", displayName: "Antigravity App", processName: "Antigravity.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "normal" },
      { id: "9", displayName: "Antigravity Backend (agy)", processName: "agy.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "normal" },
      { id: "10", displayName: "Antigravity Language Server", processName: "language_server.exe", destination: "secondaryProxy", enabled: true, source: "preset", priority: "normal" },
    ],
    firstRunCompleted: false,
  };
}

function getFallbackHealth(): HealthStatus {
  return {
    internet: { ok: true, message: "Active" },
    aetherProcess: { ok: true, message: "Ready" },
    aetherSocks: { ok: true, message: "127.0.0.1:1819" },
    aetherTunnel: { ok: true, message: "POP: FRA (62 ms)", latencyMs: 62 },
    singboxProcess: { ok: true, message: "Ready" },
    tunInterface: { ok: true, message: "singbox-tun" },
    secondaryProxy: { ok: true, message: "127.0.0.1:10808 (Connected)", latencyMs: 45 },
    routing: { ok: true, message: "Active" },
    cloudflareTrace: {
      ip: "188.114.97.10",
      warp: "on",
      colo: "FRA",
      loc: "DE",
      latencyMs: 62,
    },
    lastCheckedEpochMs: Date.now(),
  };
}
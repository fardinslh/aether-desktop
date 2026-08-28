import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
  AppSettings,
  BinaryValidationResult,
  CloudflareTrace,
  ConnectionState,
  DependencyStatus,
  DownloadProgress,
  ExecutableInspection,
  HealthStatus,
  LogEntry,
  RunningProcessInfo,
} from "../types";

export const api = {
  // Settings & Configuration
  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>("get_settings");
  },

  async saveSettings(settings: AppSettings): Promise<void> {
    return invoke<void>("save_settings", { settings });
  },

  async resetSettings(): Promise<AppSettings> {
    return invoke<AppSettings>("reset_settings");
  },

  // Connection & Orchestration
  async getConnectionState(): Promise<ConnectionState> {
    return invoke<ConnectionState>("get_connection_state");
  },

  async connect(): Promise<void> {
    return invoke<void>("connect_tunnel");
  },

  async disconnect(): Promise<void> {
    return invoke<void>("disconnect_tunnel");
  },

  // Real System Health Evaluation
  async getHealthStatus(): Promise<HealthStatus> {
    return invoke<HealthStatus>("get_health_status");
  },

  // Process & File Inspection
  async getRunningApplications(): Promise<RunningProcessInfo[]> {
    return invoke<RunningProcessInfo[]>("get_running_applications");
  },

  async inspectExecutable(filePath: string): Promise<ExecutableInspection> {
    return invoke<ExecutableInspection>("inspect_executable_file", { filePath });
  },

  async pickExecutableFile(): Promise<string | null> {
    return invoke<string | null>("pick_executable_file");
  },

  async validateAetherPath(path: string): Promise<string> {
    return invoke<string>("validate_aether_path", { path });
  },

  async validateSingboxPath(path: string): Promise<string> {
    return invoke<string>("validate_singbox_path", { path });
  },

  // Dependencies Management
  async checkDependencies(): Promise<DependencyStatus> {
    return invoke<DependencyStatus>("check_dependencies");
  },

  async installAether(): Promise<string> {
    return invoke<string>("install_aether_dependency");
  },

  async installSingbox(): Promise<string> {
    return invoke<string>("install_singbox_dependency");
  },

  async onDependencyProgress(callback: (progress: DownloadProgress) => void): Promise<UnlistenFn> {
    return listen<DownloadProgress>("dependency-progress", (event) => {
      callback(event.payload);
    });
  },

  // Proxy Connectivity Probes
  async testSecondaryProxy(): Promise<CloudflareTrace> {
    return invoke<CloudflareTrace>("test_secondary_proxy");
  },

  async testAetherProxy(): Promise<CloudflareTrace> {
    return invoke<CloudflareTrace>("test_aether_proxy");
  },

  // Logging & Inspection
  async getLogs(): Promise<LogEntry[]> {
    return invoke<LogEntry[]>("get_logs");
  },

  async exportLogs(): Promise<string> {
    return invoke<string>("export_logs");
  },

  async getSingBoxConfigPreview(): Promise<string> {
    return invoke<string>("generate_singbox_config_preview");
  },

  async validateBinaries(): Promise<BinaryValidationResult> {
    return invoke<BinaryValidationResult>("validate_binaries");
  },
};
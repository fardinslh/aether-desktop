export type ConnectionState =
  | "DISCONNECTED"
  | "STARTING_AETHER"
  | "WAITING_FOR_AETHER"
  | "TESTING_AETHER"
  | "STARTING_ROUTER"
  | "TESTING_ROUTING"
  | "CONNECTED"
  | "RECONNECTING"
  | "DISCONNECTING"
  | "ERROR";

export type RouteDestination = "direct" | "secondaryProxy" | "aether";

export type RuleSource = "preset" | "user";

export type RulePriority = "normal" | "high";

export interface ApplicationRule {
  id: string;
  displayName: string;
  executablePath?: string | null;
  processName: string;
  destination: RouteDestination;
  enabled: boolean;
  source: RuleSource;
  priority: RulePriority;
  iconBase64?: string | null;
  // Legacy compatibility fields
  name?: string;
  route?: RouteDestination;
  isPreset?: boolean;
}

export interface AetherSettings {
  executablePath: string;
  host: string;
  port: number;
  launchArguments: string[];
}

export interface SecondaryProxySettings {
  enabled: boolean;
  host: string;
  port: number;
}

export interface SingBoxSettings {
  executablePath: string;
  interfaceName: string;
  tunAddress: string;
  mtu: number;
  logLevel: string;
  strictRoute: boolean;
}

export type CompatibilityScope = "appScoped" | "globalFallback";

export type NetworkProtocol = "tcp" | "udp" | "both";

export interface CompatibilityRule {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  processNames?: string[];
  ports?: number[];
  network?: NetworkProtocol;
  destination: RouteDestination;
  scope: CompatibilityScope;
}

export interface CompatibilitySettings {
  generalsStunTurnFallback: boolean;
  privateIpBypass: boolean;
  customCompatibilityRules: CompatibilityRule[];
}

export interface GeneralSettings {
  startWithWindows: boolean;
  autoConnect: boolean;
  minimizeToTray: boolean;
  startMinimized: boolean;
  reconnectAutomatically: boolean;
}

export interface AppSettings {
  aether: AetherSettings;
  secondaryProxy: SecondaryProxySettings;
  singBox: SingBoxSettings;
  compatibility: CompatibilitySettings;
  general: GeneralSettings;
  applicationRules: ApplicationRule[];
  firstRunCompleted: boolean;
}

export interface ServiceHealth {
  ok: boolean;
  message: string;
  latencyMs?: number | null;
}

export interface CloudflareTrace {
  ip: string;
  warp: string;
  colo: string;
  loc: string;
  latencyMs: number;
}

export interface HealthStatus {
  internet: ServiceHealth;
  aetherProcess: ServiceHealth;
  aetherSocks: ServiceHealth;
  aetherTunnel: ServiceHealth;
  singboxProcess: ServiceHealth;
  tunInterface: ServiceHealth;
  secondaryProxy: ServiceHealth;
  routing: ServiceHealth;
  cloudflareTrace?: CloudflareTrace | null;
  lastCheckedEpochMs: number;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: string;
  source: string;
  message: string;
}

export interface RunningProcessInfo {
  name: string;
  processName: string;
  executablePath?: string | null;
  pid: number;
  iconBase64?: string | null;
}

export interface ExecutableInspection {
  displayName: string;
  processName: string;
  executablePath: string;
  iconBase64?: string | null;
}

export interface BinaryValidationResult {
  aetherExists: boolean;
  aetherPath: string;
  singboxExists: boolean;
  singboxPath: string;
}

export interface DependencyStatus {
  aetherInstalled: boolean;
  aetherPath: string;
  aetherVersion?: string | null;
  singboxInstalled: boolean;
  singboxPath: string;
  singboxVersion?: string | null;
}

export interface DownloadProgress {
  component: string;
  status: string;
  percent: number;
  downloadedBytes: number;
  totalBytes: number;
}
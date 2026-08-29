# Architecture & Component Model

## 1. System Architecture Diagram

```mermaid
flowchart TB
    subgraph Frontend ["Frontend (React 18 / TypeScript / Tailwind)"]
        UI[WindowTitleBar & Navbar]
        Hero[ConnectionHero / Topology Core]
        Rack[StatusOverview / Telemetry Rack]
        Matrix[ApplicationRoutingView / Route Matrix]
        Diag[DiagnosticsView / Network Console]
        SettingsUI[SettingsView / Subsystems]
        API[services/api.ts IPC Client]
    end

    subgraph TauriBridge ["Tauri IPC Command Layer (src-tauri/src/commands.rs)"]
        Cmds[Tauri Invocations & Event Emitters]
    end

    subgraph BackendCore ["Rust Backend Core (src-tauri/src/)"]
        Orch[ConnectionOrchestrator\nprocess/orchestrator.rs]
        Lock[op_lock: Mutex<()> / Atomic State]
        AetherRun[AetherRunner\nprocess/aether.rs]
        SingBoxRun[SingBoxRunner\nprocess/singbox.rs]
        Probe[HealthProber\nprocess/probe.rs]
        Gen[SingBoxConfigGenerator\nrouting/generator.rs]
        Storage[SettingsStorage\nsettings/storage.rs]
        Deps[DependencyManager\nprocess/deps.rs]
    end

    subgraph WindowsKernel ["Windows System & Child Processes"]
        AetherProc[aether.exe (SOCKS5 127.0.0.1:1819)]
        SingBoxProc[sing-box.exe (Wintun Engine)]
        WintunAdapter[Wintun Device (singbox-tun)]
        IPHelper[Windows IP Helper API / NetIO]
    end

    API <--> Cmds
    Cmds <--> Orch
    Cmds <--> Storage
    Cmds <--> Deps
    Orch --> Lock
    Orch --> AetherRun
    Orch --> SingBoxRun
    Orch --> Probe
    SingBoxRun --> Gen
    AetherRun --> AetherProc
    SingBoxRun --> SingBoxProc
    SingBoxProc --> WintunAdapter
    Probe --> IPHelper
    Probe --> AetherProc
```

---

## 2. Major Modules & Authoritative Responsibilities

### 2.1 Frontend Layer (`src/`)
- **`App.tsx`**: Top-level layout container, tab router (`dashboard`, `routing`, `diagnostics`, `settings`), and first-run wizard trigger.
- **`services/api.ts`**: Type-safe Tauri IPC wrapper invoking Rust backend commands.
- **`features/connection/ConnectionHero.tsx`**: Authoritative visual representation of the **Routing Core**, displaying WAN Gateway status, signal transport lines, connection trigger button, and 5-stage progress indicator.
- **`features/connection/StatusOverview.tsx`**: Subsystem telemetry rack displaying operational states and metrics for Egress, Aether Daemon, TUN Router, and Secondary SOCKS.
- **`features/routing/ApplicationRoutingView.tsx`**: High-density human Route Matrix managing deterministic application steering rules (`Direct`, `Secondary`, `Aether`).
- **`features/diagnostics/DiagnosticsView.tsx`**: Monospace network event stream console with subsystem filter pills and sing-box JSON configuration inspector.
- **`features/settings/SettingsView.tsx`**: Configuration interface for core engines, SOCKS endpoints, Wintun parameters, and legacy bypass policies.

### 2.2 Backend Process Orchestration (`src-tauri/src/process/`)
- **`orchestrator.rs` (`ConnectionOrchestrator`)**:
  - **Authoritative for Connection Lifecycle State Machine**: Controls transitions between `Disconnected`, `StartingAether`, `StartingSingBox`, `VerifyingRouting`, `Connected`, `Disconnecting`, and `Error`.
  - **Operation Lock (`op_lock`)**: An `Arc<Mutex<()>>` ensuring exactly one connect or disconnect operation can execute at any time. Lock acquisition occurs **before** any state mutation to prevent state desynchronization.
  - **State Event Dispatch**: Emits `connection-state-changed` events across Tauri IPC upon every state transition.
- **`aether.rs` (`AetherRunner`)**:
  - **Authoritative for Aether Daemon Lifecycle**: Spawns `aether.exe` with CLI flags (protocol `--wg`/`--masque`, scan mode, IP mode, port 1819).
  - Verifies local TCP socket availability on `127.0.0.1:1819` and Cloudflare trace connectivity via SOCKS5 before declaring Aether operational.
  - Gracefully terminates child process with job object / signal handling.
- **`singbox.rs` (`SingBoxRunner`)**:
  - **Authoritative for sing-box & Wintun Routing**: Generates dynamic JSON ruleset via `routing/generator.rs`, persists temporary configuration, and executes `sing-box.exe run`.
  - Executes **3-Stage Post-Startup Verification**:
    1. Child process health and Wintun adapter detection in network stack.
    2. IP-literal HTTPS query (`104.16.124.96/cdn-cgi/trace`) verifying tunnel transport and system egress IP match against Aether SOCKS.
    3. Mandatory Windows DNS resolution and Hostname HTTPS query (`https://www.cloudflare.com/cdn-cgi/trace`).
- **`probe.rs` (`HealthProber`)**:
  - **Authoritative for Native Network Telemetry**:
    - Uses native Windows IP Helper API (`GetAdaptersAddresses`) to query adapter FriendlyName, description, and unicast addresses matching the configured TUN subnet (`172.19.0.1/30`).
    - Executes IP-literal direct transport checks and DNS resolution tests.
    - Measures ping latency and retrieves Cloudflare POP telemetry (`colo`, `ip`, `loc`).

### 2.3 Routing Generation & Storage (`src-tauri/src/routing/` & `settings/`)
- **`routing/generator.rs` (`SingBoxConfigGenerator`)**:
  - **Authoritative for sing-box JSON Format**: Constructs compliant sing-box v1.11+ schema containing inbounds (`type: "tun"`, `interface_name: "singbox-tun"`, `strict_route: true`, `auto_route: true`), outbounds (`direct`, `aether-socks`, `secondary-socks`, `dns-out`), DNS blocks, and deterministic route rules.
- **`settings/storage.rs` (`SettingsStorage`)**:
  - **Authoritative for Configuration Persistence**: Loads and atomically writes `AppSettings` to `%APPDATA%/AetherDesktop/settings.json`.
- **`process/deps.rs` (`DependencyManager`)**:
  - **Authoritative for Engine Binaries**: Validates presence and versions of `aether.exe` and `sing-box.exe`; manages background download, progress events, and file system unpacking.

---

## 3. Important File Directory Map

```text
aether-desktop/
├── .project-memory/                 # Canonical durable project memory system
├── src/                             # React TypeScript Frontend
│   ├── App.tsx                      # Top-level UI router and shell
│   ├── index.css                    # Tailwind root and precision scrollbars
│   ├── components/                  # Layout & common components
│   │   ├── AppIcon.tsx              # Application icon renderer & fallbacks
│   │   └── layout/
│   │       ├── Navbar.tsx           # Desktop utility tab navigation
│   │       └── WindowTitleBar.tsx   # Native Windows 11 title bar & status pip
│   ├── features/
│   │   ├── connection/              # Hero & status rack components
│   │   ├── routing/                 # Application routing matrix & modals
│   │   ├── diagnostics/             # System console & log viewer
│   │   ├── settings/                # Subsystem configuration views
│   │   └── wizard/                  # First-run dependency installer wizard
│   └── services/
│       └── api.ts                   # Tauri IPC command client
├── src-tauri/                       # Rust Backend Core
│   ├── Cargo.toml                   # Rust dependencies & metadata
│   ├── build.rs                     # Build script embedding admin manifest
│   ├── windows-app-manifest.xml     # UAC requireAdministrator manifest
│   ├── src/
│   │   ├── lib.rs                   # Tauri application setup and plugin registry
│   │   ├── main.rs                  # Native application entry point
│   │   ├── commands.rs              # Tauri IPC command definitions
│   │   ├── process/                 # Process supervision, probes, orchestrator
│   │   │   ├── orchestrator.rs      # Central connection state machine
│   │   │   ├── aether.rs            # Aether daemon runner
│   │   │   ├── singbox.rs           # sing-box TUN runner
│   │   │   ├── probe.rs             # IP Helper & HTTP diagnostics prober
│   │   │   └── deps.rs              # Dependency downloader & validator
│   │   ├── routing/                 # sing-box JSON configuration generation
│   │   │   └── generator.rs         # Rule generator and route compiler
│   │   ├── settings/                # Settings storage and default config
│   │   └── logging/                 # Structured logging & ring buffer
│   └── tests/                       # Rust integration tests
│       └── generator_test.rs        # Configuration generator test suite
└── tailwind.config.js               # Precision design system tokens
```

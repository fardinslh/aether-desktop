# Architectural Decision Records (ADRs)

This document records the major architectural decisions for Aether Desktop, including the context, rationale, and consequences of each choice.

---

## ADR-001 — sing-box Owns System TUN Routing (Separated from Aether Daemon)

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: Separate the local VPN proxy daemon (`aether.exe`) from the kernel TUN routing layer (`sing-box.exe`). Aether runs strictly as a SOCKS5 proxy on `127.0.0.1:1819`, while sing-box creates the Wintun adapter and manages per-process routing rules.
- **Reason**:
  1. Aether specializes in resilient egress transport (WARP/WireGuard/MASQUE) but does not natively provide per-application routing matrices on Windows.
  2. sing-box has industry-leading Wintun support, robust process-name filtering on Windows, and flexible multi-outbound multiplexing (`direct`, `aether-socks`, `secondary-socks`).
- **Consequences**:
  - Requires orchestrating two child processes instead of one.
  - Grants complete flexibility to route applications across Direct, Secondary, or Aether paths simultaneously.

---

## ADR-002 — Native Windows IP Helper API for Adapter Detection

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: Replace cross-platform `sysinfo::Networks` inspection with direct Win32 IP Helper (`GetAdaptersAddresses`) calls in `HealthProber`.
- **Reason**:
  - On Windows 10/11, `sysinfo` relies on cached interface lists that frequently fail to register newly instantiated Wintun virtual adapters in the first 5–10 seconds.
  - `GetAdaptersAddresses` queries the kernel network stack directly, checking FriendlyName, AdapterName, and exact unicast IPv4 addresses (`172.19.0.1/30`).
- **Consequences**:
  - Eliminates false-negative TUN detection timeouts on Windows.
  - Introduces a small Windows-specific foreign function interface (FFI) block in `probe.rs`.

---

## ADR-003 — Strict Route with Inbound `hijack-dns` for Windows DNS

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: Keep `strict_route: true` enabled on the sing-box TUN inbound and define an explicit DNS block with inbound `hijack-dns` pointing to remote DNS over Aether SOCKS.
- **Reason**:
  - Windows multi-homed DNS resolution leaks DNS queries through physical Wi-Fi/Ethernet adapters when `strict_route` is disabled.
  - When `strict_route` is enabled, LAN DNS queries (`192.168.x.x:53`) cannot reach physical interfaces. `hijack-dns` intercepts all port 53 traffic at the TUN boundary and safely resolves it remotely.
- **Consequences**:
  - Completely prevents Windows DNS leaks.
  - Eliminates DNS resolution deadlock during tunnel startup.

---

## ADR-004 — Three-Stage Mandatory Egress & Hostname HTTPS Verification

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: The backend refuses to transition to `Connected` unless three progressive verification stages pass:
  1. Process alive + Wintun adapter present with correct unicast IP.
  2. IP-literal HTTPS query succeeds and egress public IP matches Aether SOCKS IP.
  3. Windows DNS resolves and hostname HTTPS query succeeds.
- **Reason**:
  - Merely checking process existence creates phantom connections where the UI indicates connected while the user has broken routing or DNS.
  - Verifying the egress IP against Aether's SOCKS IP guarantees that traffic is genuinely routing through the encrypted tunnel.
- **Consequences**:
  - Startup verification takes ~2–4 seconds longer, but guarantees 100% genuine connectivity upon reaching `Connected`.

---

## ADR-005 — Embedded UAC Administrator Manifest

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: Embed `requireAdministrator` directly into `aether-desktop.exe` via `tauri-build` `windows_attributes` and `windows-app-manifest.xml`.
- **Reason**:
  - Creating Wintun adapters and injecting default routes into the Windows IP routing table requires native administrator privileges.
  - Running without elevation causes Wintun creation to fail (`configure tun interface: Access is denied`).
- **Consequences**:
  - The application always prompts for standard Windows UAC confirmation upon launch.

---

## ADR-006 — Pre-Mutation Operation Lock (`op_lock`) Acquisition

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: Acquire the backend `op_lock.try_lock()` **before** altering the internal state machine or emitting state change events over IPC.
- **Reason**:
  - If state was set to `StartingAether` before acquiring the lock, a rapid second click would fail to acquire the lock and leave the backend in a corrupted transitional state.
- **Consequences**:
  - Guarantees transactional consistency across all connection operations.

---

## ADR-007 — Precision Desktop Network Control Utility Design System

- **Status**: Accepted & Implemented `[REAL-WINDOWS-TESTED]`
- **Decision**: Adopt a specialized network instrumentation visual identity (graphite/charcoal layers `#0c0e12`/`#13171f`, cool cyan `#00d2ff`, emerald signal green `#10b981`, amber `#f59e0b`, monospace telemetry, compact 4–6px radiuses, Route Matrix) instead of generic dark SaaS dashboard templates.
- **Reason**:
  - Aether Desktop is a technical utility for routing and tunnel management. Its visual language should communicate precision, transparency, and deterministic traffic flow.
- **Consequences**:
  - Establishes a distinctive, recognizable product identity.

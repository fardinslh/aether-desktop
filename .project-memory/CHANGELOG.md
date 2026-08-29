# AI Project Milestone Changelog

This changelog records key architecture, networking, and UX milestones. Newest entries appear first.

---

## 2026-08-29

### Changed
- Complete visual redesign into a Precision Windows Network Control Utility.
- Implemented Topology Routing Core controller, human Route Matrix with signal rails, monospace network console, and Subsystem Telemetry Rack.
- Applied graphite/charcoal surface hierarchy (`#0c0e12`, `#13171f`, `#181d26`, `#090b0e`) and signal tokens (cyan `#00d2ff`, green `#10b981`, amber `#f59e0b`, red `#ef4444`).
- Fixed operation lock ordering bug: `ConnectionOrchestrator::connect()` now acquires `self.op_lock.try_lock()` prior to state mutation.

### Why
- Elevate product identity from generic dark SaaS templates into a purpose-built desktop network appliance.
- Eliminate phantom `StartingAether` backend state on lock contention.

### Verified
- `[REAL-WINDOWS-TESTED]` Production desktop build packaged successfully (`Aether-Desktop-Setup.exe` and MSI bundle).
- `[REAL-WINDOWS-TESTED]` Frontend compiles with 0 TypeScript/Vite errors; all routing and connection views verified.

---

## 2026-08-29 (Earlier)

### Changed
- Resolved frontend polling race condition by introducing epoch counters and sequential state fetching.
- Enforced mandatory hostname HTTPS resolution in `SingBoxRunner::verify_router_and_egress()`.

### Why
- The UI exhibited rapid connect/disconnect bounce cycles due to out-of-order poll responses from backend endpoints.
- Stage 3 egress verification previously logged warnings on hostname failure instead of failing the connection attempt, violating acceptance criteria.

### Verified
- `[REAL-WINDOWS-TESTED]` Connect button transitions smoothly from `StartingAether` $\rightarrow$ `StartingSingBox` $\rightarrow$ `VerifyingRouting` $\rightarrow$ `Connected` in ~3–4 seconds without bounce.
- `[REAL-WINDOWS-TESTED]` Real internet traffic verified through Wintun adapter.

---

## 2026-08-29 (Initial Morning)

### Changed
- Resolved Windows TUN strict-route DNS deadlock by configuring typed `action: "hijack-dns"` in sing-box routing rules.
- Replaced unreliable `sysinfo` TUN detection with native Windows IP Helper API (`GetAdaptersAddresses`).
- Embedded `requireAdministrator` manifest into production executable via Tauri build attributes.

### Why
- On Windows with `strict_route: true`, sing-box blocks DNS leaks on physical adapters. Generic private IP rules caused LAN DNS queries (`192.168.x.x:53`) to stall.
- Newly instantiated Wintun adapters were not registered promptly by `sysinfo`, causing false-negative TUN detection timeouts.
- Wintun creation failed without administrator elevation.

### Verified
- `[REAL-WINDOWS-TESTED]` First end-to-end real connection reached `CONNECTED` status on physical Windows machine with live routing.

# Active Known Issues & Investigations

This document tracks **active, unresolved issues and edge cases**. Once an issue is genuinely fixed and verified, it is removed from this document and recorded in `CHANGELOG.md`.

---

## ISSUE-001 — Orphaned Wintun Interface on Ungraceful Process Kill

- **Status**: `[KNOWN ISSUE]` / Edge Case
- **Symptoms**: If `aether-desktop.exe` is forcefully killed via Task Manager (`End Process Tree`) or a hard system crash occurs during an active connection, the `singbox-tun` adapter or Windows default route metric may occasionally persist until the next system reboot or adapter reset.
- **Known Evidence**:
  - Normal graceful disconnect (`onDisconnect`, window close with tray exit) properly terminates `sing-box.exe` and tears down the adapter cleanly.
  - Windows Wintun driver retains virtual adapters until the owning handle is closed by the OS kernel.
- **Relevant Files**:
  - `src-tauri/src/process/singbox.rs`
  - `src-tauri/src/process/orchestrator.rs`
- **Safe Next Investigation**:
  - Implement a startup cleanup routine in `SingBoxRunner::new()` or `main.rs` that checks for and flushes any stale `singbox-tun` interfaces before creating a new tunnel session.
  - Explore Windows Job Objects with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` to ensure child processes are unconditionally reaped if the parent dies unexpectedly.

---

## ISSUE-002 — Network Adapter Flap During Sudden Laptop Sleep/Resume

- **Status**: `[KNOWN ISSUE]` / Low Severity
- **Symptoms**: When a laptop with an active TUN connection enters deep sleep and resumes on a different Wi-Fi network, the physical gateway IP changes while the Wintun adapter still references the previous default gateway.
- **Known Evidence**:
  - Automatic reconnection currently polls health every 3 seconds, but the transient state during the first 1–2 seconds of wake may report socket timeout before re-establishing Aether.
- **Relevant Files**:
  - `src-tauri/src/process/orchestrator.rs`
  - `src-tauri/src/process/probe.rs`
- **Safe Next Investigation**:
  - Register a Windows power event listener (`WM_POWERBROADCAST` / `PBT_APMRESUMEAUTOMATIC`) to proactively trigger a clean reconnect cycle upon system resume.

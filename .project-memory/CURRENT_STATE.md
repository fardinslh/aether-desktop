# Current Project State

**Last Updated**: 2026-08-29

---

## 1. Quick Summary
Aether Desktop is fully functional, with verified real-Windows networking and a complete precision visual redesign. The application successfully reaches the `CONNECTED` state, routes real internet traffic through the Wintun adapter, enforces strict DNS privacy, and presents a dedicated network instrumentation desktop interface.

---

## 2. Status by Functional Area

| Subsystem | Status | Evidence / Verification |
| :--- | :--- | :--- |
| **Aether Core Daemon** | `[REAL-WINDOWS-TESTED]` | Spawns correctly on elevated Windows runtime, verifies SOCKS5 endpoint on `127.0.0.1:1819`, retrieves Cloudflare trace. |
| **sing-box Wintun Engine** | `[REAL-WINDOWS-TESTED]` | Generates valid sing-box v1.11+ config, creates `singbox-tun` adapter (`172.19.0.1/30`), binds system stack. |
| **Windows TUN Detection** | `[REAL-WINDOWS-TESTED]` | Native Windows IP Helper API (`GetAdaptersAddresses`) reliably detects Wintun adapter and verifies unicast address within bounded timeout. |
| **DNS Hijack & Resolution** | `[REAL-WINDOWS-TESTED]` | Intercepts port 53 traffic on TUN and routes DNS queries via Aether tunnel with `strict_route: true` without LAN deadlock. |
| **Egress Verification** | `[REAL-WINDOWS-TESTED]` | 3-stage validation enforces process health, IP-literal HTTPS match against Aether IP, and mandatory hostname HTTPS before declaring `CONNECTED`. |
| **Connection Lifecycle** | `[REAL-WINDOWS-TESTED]` | Operation lock ordering and frontend epoch guards eliminate connection bounce races. |
| **Visual Design System** | `[IMPLEMENTED]` | Redesigned with graphite/charcoal surfaces, cool cyan/green/amber signal tokens, Topology Routing Core, and Route Matrix. |
| **Installer & Packaging** | `[REAL-WINDOWS-TESTED]` | Production NSIS setup (`Aether-Desktop-Setup.exe`) and MSI installer compile cleanly with embedded UAC administrator manifest. |

---

## 3. Latest Major Milestones & Fixes

1. **Precision Visual Redesign Pass** (`7c5d6cb`):
   - Replaced generic SaaS styling with a precision desktop network control utility.
   - Built Topology Routing Core controller, human Route Matrix, and monospace network console.
2. **Operation Lock Ordering Fix** (`5567ae2`):
   - Backend `ConnectionOrchestrator::connect()` now acquires `self.op_lock.try_lock()` **before** state mutation to prevent phantom `StartingAether` states.
3. **Frontend Polling Race & Epoch Guard** (`861c6be`):
   - Implemented epoch counter and sequential polling to eliminate UI connect/disconnect bounce cycles.
4. **Mandatory Hostname HTTPS Verification** (`fbc73ec`):
   - Stage 3 egress probe requires successful hostname HTTPS resolution before declaring `CONNECTED`.
5. **Windows Strict-Route DNS Deadlock Fix** (`2839881`):
   - Configured typed `hijack-dns` block to route Windows TUN DNS queries over Aether SOCKS.

---

## 4. Current Active Work & Next Recommended Tasks

- **Active Work**: Project memory stabilization and documentation preservation.
- **Recommended Next Tasks**:
  1. **Physical Windows Validation**: Install [`Aether-Desktop-Setup.exe`](../Aether-Desktop-Setup.exe) on target physical Windows machine to verify UX fluidity and visual fidelity under live high-DPI scaling.
  2. **Application Preset Expansion**: Add preset definitions for additional developer tools (e.g. Docker, WSL2 localhost bypass, JetBrains IDEs) and popular multiplayer titles.
  3. **Auto-Updater Integration**: Wire up Tauri v2 updater plugin with GitHub releases for seamless background updates.

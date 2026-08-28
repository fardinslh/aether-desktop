# Aether Desktop

> A production-quality Windows desktop VPN and network routing manager wrapping **Aether**, **sing-box**, and an optional **Secondary Proxy (Xray/V2Ray)**.

---

## 🌟 Overview

**Aether Desktop** is a desktop application built with **Tauri v2**, **Rust**, **React**, and **TypeScript** that provides transparent system-wide TUN routing on Windows. It orchestrates a multi-tier proxy architecture designed for gaming, developer workflows, and privacy:

1. **Aether** (Primary Tunnel):
   - Local SOCKS5 proxy (default: `127.0.0.1:1819`).
   - Serves as the default outbound for all general system traffic.
2. **sing-box** (Windows TUN & Routing Engine):
   - Creates and manages the system TUN interface (`singbox-tun`).
   - Automatically generates route rules enforcing strict 8-tier precedence.
3. **Secondary Proxy** (Optional Xray / V2Ray / SOCKS5):
   - Local SOCKS5 endpoint (default: `127.0.0.1:10808`).
   - Routes applications requiring specific geographic egress (such as Antigravity, VS Code, Chrome, ChatGPT, Discord).

---

## 🏗️ Architecture & 8-Tier Rule Precedence

Aether Desktop enforces a verified **8-Tier Routing Precedence Model** to eliminate NAT conflicts, Discord voice connection freezes, and gaming matchmaking drops:

```text
[ Incoming System Network Traffic (singbox-tun) ]
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. Core Loop Prevention                                    │ ──► DIRECT
│    (aether.exe, xray.exe, v2ray.exe, v2rayN.exe)            │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. High-Priority Application Overrides                      │ ──► SECONDARY PROXY
│    (Discord.exe -> avoids STUN port conflict)              │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Global STUN/TURN Compatibility Fallback                  │ ──► DIRECT
│    (Ports 3478, 5349 -> fixes C&C Generals Online STUN)     │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Normal Direct Applications                               │ ──► DIRECT
│    (dota2.exe, RustClient.exe, Rust.exe)                    │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Normal Secondary Proxy Applications                      │ ──► SECONDARY PROXY
│    (chrome.exe, Code.exe, Antigravity.exe, agy.exe, etc.)   │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 6. Normal Explicit Aether Applications                      │ ──► AETHER
│    (User-assigned Aether rules)                             │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 7. Private LAN Bypass                                       │ ──► DIRECT
│    (192.168.0.0/16, 10.0.0.0/8, 172.16.0.0/12)             │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ 8. Final Outbound (Default Catch-all)                       │ ──► AETHER
└─────────────────────────────────────────────────────────────┘
```

---

## ✨ Features

- **🛡️ Single-Prompt UAC Elevation**:
  Embedded Windows application manifest requesting `requireAdministrator` alongside `Microsoft.Windows.Common-Controls` v6. Prompts UAC once on launch; all child network processes inherit elevated permissions without repeated dialogs.
- **⚡ Sub-Second Live Rule Apply**:
  Modify application routing rules dynamically while connected. Config is pre-validated with `sing-box check` and reloaded transparently.
- **🎮 Game & AI Coexistence**:
  Discord Voice WebRTC works without "Connecting..." lockups while *Command & Conquer Generals Online* peer-to-peer STUN matchmaking remains direct.
- **🔍 Dynamic App Discovery**:
  Add any `.exe` by picking from running GUI processes or browsing the filesystem, with automatic duplicate detection and customizable routing destination.
- **🎛️ Advanced Priority Selector**:
  Optionally promote custom rules to `High Priority Override` to bypass global port fallbacks when required.
- **🩺 Integrated Health Probing**:
  Active background latency and endpoint verification querying SOCKS5 proxies against Cloudflare CDN edge nodes.
- **📋 Zero-Window Process Execution**:
  `aether.exe` and `sing-box.exe` run with Windows `CREATE_NO_WINDOW` (0x08000000); all stdout/stderr logs stream to an in-memory circular ring buffer viewable in the UI.

---

## 🚀 Getting Started

### Prerequisites

- **Windows 10 / 11** (64-bit)
- **Node.js** 18+ and **npm**
- **Rust Toolchain** (1.80+ / `x86_64-pc-windows-msvc` or `x86_64-pc-windows-gnu`)
- **Microsoft Edge WebView2 Runtime** (installed by default on Windows 10/11)

> **Note**: `aether.exe` and `sing-box.exe` executables are user-supplied and are **not** bundled into the git repository. You can specify their local paths during initial setup or in the Settings tab.

### Installation & Development

1. **Clone the repository**:
   ```powershell
   git clone https://github.com/fardinslh/aether-desktop.git
   cd aether-desktop
   ```

2. **Install frontend dependencies**:
   ```powershell
   npm install
   ```

3. **Run in development mode**:
   ```powershell
   npm run tauri dev
   ```

4. **Run the 10-Scenario Routing Regression Suite**:
   ```powershell
   cd src-tauri
   cargo run --bin test_generator
   ```

5. **Build the Production Executable**:
   ```powershell
   npm run tauri build
   ```
   The compiled `.exe` will be generated in `src-tauri/target/release/`.

---

## 📁 Project Structure

```text
aether-desktop/
├── src/                         # React + TypeScript Frontend
│   ├── components/              # Shared UI (Header, Navigation, Badge, HeroCard)
│   ├── features/
│   │   ├── dashboard/           # Connection hero, latency metrics, quick toggles
│   │   ├── routing/             # Application routing manager & rule modals
│   │   ├── settings/            # Binary paths, SOCKS endpoints, general config
│   │   ├── diagnostics/         # Live log streaming, network health inspector
│   │   └── wizard/              # First-run setup wizard
│   ├── stores/                  # Zustand application state store
│   ├── services/                # Tauri IPC command wrappers
│   └── types/                   # TypeScript interfaces matching Rust models
├── src-tauri/                   # Rust Backend (Tauri v2)
│   ├── src/
│   │   ├── commands/            # Tauri IPC command handlers
│   │   ├── health/              # SOCKS5 connectivity & Cloudflare trace prober
│   │   ├── logging/             # Thread-safe ring buffer logger
│   │   ├── models/              # Rule, Setting, and Health data structures
│   │   ├── process/             # Child process runners & state machine orchestrator
│   │   ├── routing/             # 8-tier sing-box config generator & presets
│   │   ├── settings/            # Persistent JSON settings storage
│   │   ├── bin/                 # Standalone test runner and config dump utilities
│   │   └── lib.rs               # App entrypoint & exit cleanup handlers
│   ├── build.rs                 # Custom Windows manifest with requireAdministrator + Comctl32 v6
│   ├── Cargo.toml               # Rust package dependencies
│   └── tauri.conf.json          # Tauri application configuration
├── package.json
└── README.md
```

---

## 📜 Third-Party Notices & Licensing

- **sing-box**: [GPLv3 License](https://github.com/SagerNet/sing-box) — Used as an external child process router; not linked into the desktop binary.
- **Tauri**: [Apache 2.0 / MIT License](https://tauri.app)
- **React**: [MIT License](https://reactjs.org)
- **Lucide Icons**: [ISC License](https://lucide.dev)
- **Tailwind CSS**: [MIT License](https://tailwindcss.com)

See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) for full license texts and notices.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
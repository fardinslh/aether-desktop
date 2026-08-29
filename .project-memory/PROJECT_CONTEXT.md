# Project Context & Product Definition

## 1. Product Overview
**Aether Desktop** is a premium, specialized Windows network routing and VPN control utility. It provides deterministic, per-application split tunneling and censorship-resistant egress routing for power users, gamers, voice chat applications, and AI developer workflows.

Instead of presenting the user with an undifferentiated "all-or-nothing" VPN toggle, Aether Desktop acts as an intelligent local routing orchestrator. It manages a dual-engine architecture:
1. **Aether Core Daemon (`aether.exe`)**: A high-performance proxy daemon providing clean, resilient egress transport via Cloudflare Warp/WireGuard/MASQUE over a local SOCKS5 endpoint (`127.0.0.1:1819`).
2. **sing-box TUN Engine (`sing-box.exe`)**: A system-level Wintun router creating a dedicated virtual network adapter (`singbox-tun` @ `172.19.0.1/30`), enforcing kernel-level per-process routing rules, strict DNS isolation, and fallback policies.
3. **Secondary SOCKS Proxy (Optional)**: Seamless integration with external proxies (such as v2rayN / Xray on `127.0.0.1:10808`) for targeted applications (e.g., Discord Voice, AI coding tools, web browsers).

---

## 2. Core Tech Stack

| Layer | Technologies | Key Responsibilities |
| :--- | :--- | :--- |
| **OS Target** | Windows 10 / Windows 11 (x64) | Requires Administrator elevation (UAC) for Wintun adapter creation and IP Helper routing table management. |
| **Desktop Shell** | Tauri v2 (`@tauri-apps/api` v2.2+, `tauri` v2.11+) | Native Windows window management, IPC command bridge, tray lifecycle, process spawning, file dialogues. |
| **Backend Runtime** | Rust (Edition 2021) | Safe process orchestration (`AetherRunner`, `SingBoxRunner`), atomic lifecycle state machine, native Windows IP Helper API probes, config generation, structured logging. |
| **Frontend Framework** | React 18, TypeScript 5, Vite 6 | State management (custom lightweight stores), responsive instrumentation UI, real-time telemetry polling, modal managers. |
| **Styling & Design System** | Tailwind CSS 3.4 (Custom Tokens) | Precision network instrumentation theme (graphite/charcoal layered surfaces, cool cyan, signal green, amber, red, monospace telemetry). |
| **Network Core Binaries** | `aether.exe` & `sing-box.exe` (Wintun) | Managed child processes executed with isolated IPC pipes and health supervision. |

---

## 3. Product Goals

1. **Deterministic Traffic Isolation**: Ensure process-level routing is strictly adhered to (e.g., Discord Voice never drops due to VPN IP churn; games route Direct with zero latency impact).
2. **Robust Connection Lifecycle**: Zero tolerance for half-connected or phantom states. The UI only declares `CONNECTED` when the entire verification chain (process $\rightarrow$ TUN adapter $\rightarrow$ IP egress $\rightarrow$ DNS hijack $\rightarrow$ Hostname HTTPS) succeeds.
3. **Zero Configuration Friction**: Auto-discovery and automatic downloading of dependencies (`aether.exe`, `sing-box.exe`, Wintun drivers) upon first run.
4. **Appliance-Grade Reliability**: Graceful recovery from network disconnects, adapter resets, and system sleep without leaking unencrypted DNS or dangling system routes.
5. **Human-Centric Network Telemetry**: Transform cryptic routing tables into an intuitive "Route Matrix" and topological visual highway that clearly informs the user which route each process is taking.

---

## 4. Product Non-Goals

- **Not a Generic SaaS / Consumer VPN**: Aether Desktop is not intended to be a multi-server consumer VPN with 50 country flags and glossy animations. It is a precision desktop network appliance.
- **Not a Browser Extension**: All routing occurs at the Windows network stack / Wintun level, steering system-wide `.exe` processes rather than browser-only traffic.
- **Not a Protocol Hacker Sandbox**: While advanced flags are accessible, the primary interface abstracts raw JSON/config manipulation into deterministic, type-safe settings and visual routing rules.

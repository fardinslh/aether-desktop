# Authoritative Networking Architecture & Invariants

> **CRITICAL WARNING FOR AI AGENTS**:
> The networking pipeline described in this document represents hard-won, real-Windows-tested stability.
> **DO NOT** casually remove verification steps, disable `strict_route`, weaken DNS hijack rules, or replace native Windows IP Helper adapter detection. Any modification to this file or the underlying networking code must adhere strictly to these invariants.

---

## 1. Core Endpoints & Network Topology

```
[Applications & Games]
         │
         ▼
[Wintun Device: 'singbox-tun' (172.19.0.1/30)]
         │
         ├── Port 53 UDP/TCP ──► [sing-box DNS Engine (Hijack-DNS)] ──► [Aether SOCKS5: 1819] ──► Remote DNS (1.1.1.1)
         │
         ├── Process Routing Rules:
         │     ├── Proxy Cores (aether.exe, sing-box.exe, etc.) ──────► [DIRECT Outbound] ──► Physical WAN Gateway
         │     ├── High Priority Rules (Discord Voice WebRTC) ────────► [Target Outbound: Secondary / Direct]
         │     ├── Compatibility Fallbacks (STUN/TURN 3478, 5349) ────► [DIRECT Outbound]
         │     ├── Application Rules (Chrome, Steam, etc.) ───────────► [Secondary: 10808 / Direct / Aether]
         │     └── Private LAN Subnets (192.168.0.0/16, 10.0.0.0/8) ──► [DIRECT Outbound]
         │
         └── Final Catch-All / Default Route ────────────────────────► [Aether SOCKS5 Outbound: 1819] ──► Cloudflare Egress
```

### Key Network Parameters:
- **Aether Local Proxy**: `127.0.0.1:1819` (SOCKS5 protocol).
- **Secondary Local Proxy**: `127.0.0.1:10808` (SOCKS5 protocol, optional v2rayN/Xray).
- **sing-box TUN Adapter**:
  - Interface Name: `singbox-tun`
  - TUN IPv4 Address: `172.19.0.1/30`
  - MTU: `1500` (configurable, default 1500)
  - Stack: `system` (Windows native TCP/IP stack integration)
  - `auto_route: true`
  - `strict_route: true` (mandatory on Windows to isolate DNS and routing leaks)

---

## 2. Deterministic Routing Precedence

The sing-box routing engine evaluates rules in strict sequential order. The generator in `src-tauri/src/routing/generator.rs` compiles rules in this exact order:

```
┌────┬────────────────────────────────────────────────────────────────────────┐
│ #  │ Routing Evaluation Layer & Purpose                                     │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 1  │ DNS Infrastructure / Inbound Hijack Rule                              │
│    │ Inbound: singbox-tun, Port: 53 -> action: hijack-dns                   │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 2  │ Proxy Core Self-Loop Prevention -> DIRECT                             │
│    │ Prevents aether.exe, sing-box.exe, xray.exe, v2ray.exe from looping   │
│    │ back into the TUN interface.                                           │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 3  │ High-Priority Application Overrides -> Assigned Target Outbound        │
│    │ Explicit overrides (e.g. Discord.exe prioritized to Secondary/Direct   │
│    │ before STUN/TURN fallback rules can capture its voice WebRTC traffic). │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 4  │ Global Compatibility Fallback Ports -> DIRECT                          │
│    │ When enabled, unassigned STUN/TURN traffic on ports 3478, 5349 routes  │
│    │ Direct for legacy games (e.g. Generals Online).                        │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 5  │ Standard Application Routing Rules -> Target Outbound                  │
│    │ Per-process rules steering traffic to DIRECT, SECONDARY, or AETHER.    │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 6  │ Private LAN Bypass -> DIRECT                                           │
│    │ When enabled, private IP ranges (192.168.0.0/16, 10.0.0.0/8, etc.)    │
│    │ bypass the tunnel so local LAN/NAS/routers remain reachable.           │
├────┼────────────────────────────────────────────────────────────────────────┤
│ 7  │ Final Default Catch-All Rule -> Aether SOCKS5 Outbound                │
│    │ All remaining outbound system traffic routes through Aether (1819).    │
└────┴────────────────────────────────────────────────────────────────────────┘
```

---

## 3. High-Risk Architectural Invariants

### ⚠️ Invariant 1: Windows DNS Strict-Route & Hijack-DNS
- **Why it matters**: On Windows, when `strict_route: true` is active, non-TUN interfaces are blocked from DNS leaks. If DNS requests (`192.168.x.x:53`) match a generic "private IP $\rightarrow$ direct" rule, Windows DNS queries will fail completely because the physical interface cannot answer them while strict route is engaged.
- **The Solution**: In sing-box v1.11+, DNS requests entering the TUN **MUST** be intercepted with `action: "hijack-dns"` and forwarded over the remote DNS server (`https://1.1.1.1/dns-query` or `1.1.1.1`) routed through `aether-socks`.
- **DO NOT** remove the DNS block or replace typed hijack-dns with raw route rules.

### ⚠️ Invariant 2: Authoritative Egress IP Verification
- **Why it matters**: Simply checking if `sing-box.exe` is running is insufficient. A crashed Wintun driver or broken routing table can leave the UI saying "Connected" while the user has zero internet access or is leaking traffic unencrypted.
- **Verification Protocol (`verify_router_and_egress`)**:
  1. **Stage 1 (Process & Adapter)**: Child process alive + `GetAdaptersAddresses` finds `singbox-tun` with unicast IP `172.19.0.1`.
  2. **Stage 2 (Direct IP-Literal HTTPS)**: Query `https://104.16.124.96/cdn-cgi/trace` directly. Extract system public IP and verify it **exactly matches** the public IP reported by Aether SOCKS (`127.0.0.1:1819`).
  3. **Stage 3 (DNS Resolution & Hostname HTTPS)**: Resolve `cloudflare.com` and query `https://www.cloudflare.com/cdn-cgi/trace`. If hostname HTTPS fails after bounded retries, the connection attempt is aborted and transitioned to `Error`.
- **DO NOT** make hostname HTTPS optional. `CONNECTED` state strictly requires all stages to pass.

### ⚠️ Invariant 3: Native Windows IP Helper Adapter Detection
- **Why it matters**: `sysinfo::Networks` is unreliable on Windows for freshly created Wintun virtual adapters and often reports stale cached adapter tables.
- **The Solution**: `HealthProber::check_tun_interface_exists()` uses the native Windows IP Helper API (`GetAdaptersAddresses`) to query adapter FriendlyName, description, and unicast IP addresses.

### ⚠️ Invariant 4: Mandatory UAC Administrator Elevation
- **Why it matters**: Wintun adapter initialization, default gateway injection, and routing table configuration require native Windows `SeNetworkConfiguration` administrator privileges. Running without elevation produces `configure tun interface: Access is denied`.
- **The Solution**: `windows-app-manifest.xml` embeds `requireAdministrator` via `build.rs` / `tauri-build` `windows_attributes`. The application must always execute with elevated permissions.

### ⚠️ Invariant 5: Operation Lock Ordering
- **Why it matters**: In `ConnectionOrchestrator::connect()`, acquiring `self.op_lock.try_lock()` **must occur before** mutating backend state or emitting events.
- If lock acquisition fails, the function immediately returns `"Connection operation already in progress"` without transitioning state to `StartingAether`.

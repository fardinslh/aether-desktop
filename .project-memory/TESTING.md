# Testing & Verification Guide

This document outlines the testing protocols, build commands, and acceptance criteria for Aether Desktop.

---

## 1. Test Levels & Reliability Hierarchy

| Test Level | Reliability for Windows Networking | Description |
| :--- | :--- | :--- |
| **Unit Tests** | ⭐ Low (Logic only) | Tests JSON serialization, rule sorting, priority precedence, and state enum transitions in memory. |
| **Mocked Integration Tests** | ⭐⭐ Moderate (Flow only) | Simulates child process stdout and mocked HTTP responses. Does **NOT** test Windows Wintun driver, UAC elevation, or kernel IP routing. |
| **Real Windows Acceptance Tests** | ⭐⭐⭐⭐⭐ Authoritative (100%) | Physical execution on Windows 10/11 with elevated UAC, creating real Wintun adapters, routing live TCP/UDP traffic, and querying real DNS. |

> **RULE FOR AI AGENTS**:
> Mocked and unit tests passing does **NOT** prove that Windows networking is functional. Never mark a network or routing feature as complete without `[REAL-WINDOWS-TESTED]` evidence.

---

## 2. Real Connection Chain Acceptance Criteria

For any connection attempt to be considered successful, the following sequence **MUST** complete without errors:

```
1. [Aether Process Spawned]
   └── SOCKS5 listener bound on 127.0.0.1:1819
       └── Cloudflare trace query through SOCKS5 returns public IP (IP_aether)

2. [sing-box Process Spawned with Wintun]
   └── Native IP Helper API finds 'singbox-tun' adapter with 172.19.0.1/30

3. [Stage 1: Direct System IP-Literal HTTPS Egress]
   └── Query https://104.16.124.96/cdn-cgi/trace directly through Windows network stack
       └── Extracted public IP (IP_system) must EXACTLY MATCH IP_aether

4. [Stage 2: Windows DNS Hijack Resolution]
   └── Resolve domain 'cloudflare.com' through TUN interface port 53 hijack

5. [Stage 3: Hostname HTTPS Egress Probe]
   └── Query https://www.cloudflare.com/cdn-cgi/trace
       └── Returns HTTP 200 OK with valid trace body

6. [STATE: CONNECTED Declared]
   └── Backend transitions state to Connected and emits IPC event to UI
```

---

## 3. Build & Test Commands

### 3.1 Frontend Development & Build
```powershell
# Run Vite development dev server (hot reload UI)
npm run dev

# Compile TypeScript & Build Frontend Production Bundle
npm run build
```

### 3.2 Backend Compilation & Tests
```powershell
# Run generator unit tests
cargo test --test generator_test

# Test-generator standalone binary (requires elevation)
cargo run --bin test_generator
```

*Note on Windows Rust Toolchains*:
- When using `LLVM-MinGW (UCRT)`, ensure the active Rust toolchain is `stable-x86_64-pc-windows-gnullvm`.
- Ensure the LLVM-MinGW `bin` directory is in your `$env:PATH`.

### 3.3 Production Packaging (Desktop Installer)
```powershell
# Build both NSIS Setup and MSI Installers (Release Mode)
npm run desktop:build
```

**Installer Output Locations**:
- **NSIS Setup Executable**: `src-tauri/target/release/bundle/nsis/Aether Desktop_0.1.0_x64-setup.exe`
- **Root Release Executable**: `Aether-Desktop-Setup.exe`
- **MSI Installer Package**: `src-tauri/target/release/bundle/msi/Aether Desktop_0.1.0_x64_en-US.msi`
- **Raw Application Binary**: `src-tauri/target/release/aether-desktop.exe`

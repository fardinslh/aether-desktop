# Instructions for AI Coding Agents

Welcome, AI Agent (Gemini, Claude, Codex, ChatGPT, or another model).

This repository contains **Aether Desktop**, a precision Windows network routing and VPN control utility.

---

## 🛑 Essential Protocol Before Modifying Code

Before making any non-trivial changes, executing plans, or refactoring:

1. **Read the Project Memory Index**:
   Open and read [`.project-memory/README.md`](./.project-memory/README.md).
2. **Review Current Project State**:
   Read [`.project-memory/CURRENT_STATE.md`](./.project-memory/CURRENT_STATE.md) to understand what currently works and what is active.
3. **Review Network Invariants**:
   If touching any networking, routing, DNS, or process supervisor code, **MUST READ** [`.project-memory/NETWORKING.md`](./.project-memory/NETWORKING.md).
4. **Inspect On-Disk Source First**:
   Always verify the current state of files directly on disk before assuming documentation or memory is up to date.
5. **Preserve Known-Good Networking**:
   Do **NOT** casually weaken egress verification, remove `strict_route`, bypass DNS hijack rules, or replace native Windows IP Helper detection.
6. **Maintain Project Memory**:
   After implementing features, fixing bugs, or verifying test results, update the relevant files in `.project-memory/` (e.g. `CURRENT_STATE.md`, `KNOWN_ISSUES.md`, `CHANGELOG.md`).

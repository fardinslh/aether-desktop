# Aether Desktop — Project Memory System

Welcome to the durable, local project memory for **Aether Desktop**.

This directory (`.project-memory/`) is the canonical source of truth for the project's architecture, networking mechanics, decision log, testing protocols, and live state. It exists so that any AI coding assistant or engineer can step into this repository at any time, understand the codebase deeply, and make safe, verified modifications without requiring past chat transcripts or account-specific memory.

---

## 📚 Memory Directory Index

| File | Purpose | When to Read |
| :--- | :--- | :--- |
| [`PROJECT_CONTEXT.md`](./PROJECT_CONTEXT.md) | High-level product definition, target environment, tech stack, goals & non-goals. | Onboarding & understanding project scope. |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Component architecture, lifecycle ownership, backend/frontend boundaries, and file directory map. | Before modifying application structure or process lifecycle. |
| [`NETWORKING.md`](./NETWORKING.md) | **CRITICAL**: The authoritative networking architecture, TUN routing precedence, DNS rules, SOCKS chains, and high-risk invariants. | **MUST READ** before touching routing, sing-box generation, Aether flags, or network probes. |
| [`CURRENT_STATE.md`](./CURRENT_STATE.md) | Snapshot of what currently works, real Windows test results, recent fixes, and next recommended tasks. | At the beginning of every session. |
| [`DECISIONS.md`](./DECISIONS.md) | Architectural Decision Records (ADRs) explaining *why* key architectural choices were made. | When proposing or questioning structural decisions. |
| [`KNOWN_ISSUES.md`](./KNOWN_ISSUES.md) | Active, confirmed issues and safe investigation paths (no fixed bug graveyards). | When debugging or looking for areas needing improvement. |
| [`TESTING.md`](./TESTING.md) | Build instructions, unit/mock testing, test-generator verification, and real Windows test protocols. | Before and after making any code changes. |
| [`CHANGELOG.md`](./CHANGELOG.md) | High-level milestone history documenting meaningful changes, rationale, and verification status. | To review what was accomplished in recent milestones. |

---

## 🏷️ Evidence Status Standards

To maintain absolute clarity, all claims in this memory system use standard evidence labels:

- `[REAL-WINDOWS-TESTED]`: Verified on actual physical Windows 10/11 operating systems with live network traffic and actual TUN adapters.
- `[MOCK-TESTED]`: Verified via automated unit/integration tests or mocked responses.
- `[IMPLEMENTED]`: Code is written and compiles cleanly, but awaits real-environment Windows verification.
- `[CONFIRMED]`: Behavior or root cause reproduced and verified by diagnostics.
- `[PLANNED]`: Future roadmap item or intended architectural evolution.
- `[KNOWN ISSUE]`: Active, confirmed bug or limitation requiring resolution.

---

## 🤖 Self-Maintenance Rules for AI Agents

Every AI agent (Gemini, Codex, Claude, ChatGPT, or custom models) operating in this repository **MUST** adhere to the following maintenance protocol:

1. **Read Before Writing**:
   - Always read [`CURRENT_STATE.md`](./CURRENT_STATE.md) and [`NETWORKING.md`](./NETWORKING.md) before planning or executing tasks.
   - Inspect the actual source code on disk before assuming documentation is completely up to date.
2. **Preserve Known-Good Networking**:
   - Never weaken routing verification, remove strict route isolation, or bypass DNS hijack rules without explicit user instruction.
   - Treat all items marked `HIGH-RISK` in [`NETWORKING.md`](./NETWORKING.md) as strict invariants.
3. **Update Memory Upon Meaningful Changes**:
   - Update [`CURRENT_STATE.md`](./CURRENT_STATE.md) after implementing features, fixing bugs, or receiving test results.
   - Update [`KNOWN_ISSUES.md`](./KNOWN_ISSUES.md) when an issue is resolved (remove it) or a new genuine defect is uncovered.
   - Add a dated entry to [`CHANGELOG.md`](./CHANGELOG.md) for meaningful milestones.
   - Add an ADR in [`DECISIONS.md`](./DECISIONS.md) only when making significant architectural changes.
   - Update [`NETWORKING.md`](./NETWORKING.md) only if the actual network pipeline or routing generator is modified.
4. **No Hallucinated Testing**:
   - Never label an unverified code change as `[REAL-WINDOWS-TESTED]`. Mock tests do NOT prove real Windows networking.
5. **Zero Secret Policy**:
   - Never commit API keys, personal credentials, private tokens, or sensitive network credentials to project memory.

# ADR-0013: Explicit desktop runtime sessions and manual scans

- Status: Authorized by M2B; source review required before M2C/M3
- Date: 2026-10-14

## Context

M1C and M2A validation and source review are accepted. M2B authorizes desktop execution through the existing runtime, without changing the accepted ST, type, IR, TON, clock, budget or transaction semantics. Compile must remain pure and failed replacement must preserve the loaded program.

## Decision

Use a headless desktop-session application crate. It depends on app-contract for DTOs, st-frontend for bounded compilation and plc-runtime/plc-types for the existing executable engine. Core and language crates never depend on the application service. Tauri owns one service and dispatches blocking work off the UI thread.

Prepare compilation, validation, lowering and Runtime construction before locking. A single std::sync::Mutex serializes session replacement, input staging, scan, reset, unload and snapshot observation. Only successful replacement advances a checked monotonic u64 session generation. Every subsequent command names that generation; stale commands fail before touching Runtime. Poisoning fails closed with an application error. No unsafe code or asynchronous lock is needed.

Rust defines the fixed Alpha defaults once: 100,000 microseconds and 10,000 operations. Load accepts source, not browser IR or task configuration. Each mutation returns one complete snapshot built under the same lock. Inputs display staged values; output/local and TON Q/ET come from committed runtime observation. TIME, counters, budgets and opaque IDs use canonical decimal strings. REAL uses the existing checked finite binary32 wrapper. No activation epoch or executable representation crosses IPC.

React associates a loaded session with the exact editor revision. Editing immediately disables execution and marks it stale while keeping the backend session for safe replacement/unload. Local monotonic request generations discard old responses; backend generation checks remain authoritative. Explicit Compile success is required before Load/Single Scan in the UI, but Rust independently compiles every Load. Runtime faults are snapshot diagnostics with core provenance, separate from transport errors and compiler markers. Reset invokes the unchanged Runtime::reset and visibly clears inputs too.

## Alternatives and consequences

Putting ownership in React, IPC, the ST frontend or core changes their roles and is rejected. A background task/actor, cancellation framework, per-variable polling or multi-session registry adds unrequested complexity. A mutex over the bounded synchronous engine is sufficient. Source association is a UI rule; generation identity is the backend mutation boundary. Compile does not invalidate or create a runtime. Failed compile/load and invalid input leave the old session intact.

IPC advances to v4 with generated types and strict request/response checks. The core-info capability reports manual-single-scan; it does not imply a program is loaded. Run/Pause/Stop, pacing, processes, networking, recording, grading and persistence remain unavailable. Existing foundations and historical validation evidence remain unchanged. Independent Windows/Linux tests and actual interactive Windows WebView2 smoke are required before M2B PASS.

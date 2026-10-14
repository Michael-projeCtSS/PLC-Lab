# ADR-0004: Sandboxed PLC student-code execution

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-008, FND-009

## Context

Faulty student programs must stop their simulated PLC task without freezing the desktop application or accessing the host OS.

## Decision

Run student programs only in the Rust PLC VM over validated Canonical PLC IR. Provide execution budgets per scan, bounded call depth and allocation, and checked memory access. Student code receives no filesystem, sockets, process launching, native code, JavaScript evaluation, or OS capabilities. Standard blocks execute inside the same bounded semantics.

The later runtime must execute outside the UI thread, accept cancellation at bounded intervals, and emit typed faults with source locations. A wall-clock timeout alone is insufficient; deterministic instruction/work budgets are required. IR and project validation are required even when files were created locally. Tauri capabilities restrict host access but are not the PLC VM sandbox.

## Alternatives

eval, generated native binaries, and separate language engines violate the contract. Unbounded loops with a UI watchdog can still exhaust memory or stall execution. Reject these alternatives.

## Consequences and migration

Budget exhaustion, recursion, invalid access, and resource limits require negative and integration tests before execution can ship. Limits and partial-scan fault behavior must be designed in M1. No executable student artifacts exist to migrate.

## Sprint boundary

Expose no run command. Disable all execution controls. Implement no VM or student-code evaluation.

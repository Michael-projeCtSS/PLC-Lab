# ADR-0006: Trace-first architecture

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-007, FND-009

## Context

Visualization, diagnostics, replay, and behavioral grading need consistent evidence of what executed and when.

## Decision

trace-core will define canonical ordered events and snapshots independent of UI and transport. Preserve simulation timestamp, scan number, input/output/variable changes, executed source locations, runtime faults, process events, and network events. Execution and simulation produce trace evidence; UI and grading consume it without re-evaluating PLC logic.

Design trace ordering, schema version, retention bounds, and overflow handling with M1. A future overflow cannot silently remove evidence needed for grading. Canonical comparison excludes wall-clock/UI transport metadata and includes deterministic ordering. Source mapping is required from the first executable IR.

## Alternatives

Console logs are unstructured and insufficient for replay. Adding traces after a VM is complete risks losing source mapping and causality. UI-generated traces report rendering rather than authoritative execution. Reject these alternatives.

## Consequences and migration

Runtime, process, and network APIs must integrate trace recording from their first implementations. Grading must evaluate event sequences and timing, not just final values. No trace artifacts exist to migrate.

## Sprint boundary

Reserve trace-core and Timeline, document tests, but emit no invented scans or variable transitions.

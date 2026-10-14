# ADR-0003: Deterministic SimulationClock

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-007, FND-009

## Context

Timer and process behavior must be independent of machine speed, window focus, and rendering frame rate.

## Decision

simulation-core will own a SimulationClock abstraction based on checked, nonnegative integer virtual time. Plan microsecond ticks with a u64 range; transport large ticks as decimal strings to avoid JavaScript integer precision loss. No clock implementation exists in Sprint 0.

Provide paused, single-scan, real-time, and accelerated modes. Real-time wall-clock pacing may decide when to request work, but cannot determine the virtual delta or PLC semantics. Advance virtual time once after a completed scan. A fault must have an explicit policy for partial state and time advancement, specified before runtime implementation.

Same program, initial state, ordered stimuli, seed, and timing configuration must yield the same canonical trace. The scheduler must eventually specify tie-breaking for equal-time events, checked overflow behavior, and seeded random fault generation.

## Alternatives

System time introduces nondeterminism. Frame-driven simulation depends on UI performance. Floating-point authoritative timestamps complicate boundary comparisons. Reject these approaches.

## Consequences and migration

Clock and scheduler tests need boundary and overflow cases. Rendering interpolations must never feed state back into models. There is no timing implementation to migrate.

## Sprint boundary

Document the abstraction and reserve its crate; implement no scheduling or timers.

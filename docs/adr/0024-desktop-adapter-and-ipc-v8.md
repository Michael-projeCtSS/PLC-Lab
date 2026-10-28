# ADR-0024: Desktop application adapter and IPC contract v8

- Status: Accepted for the product rebuild (supersedes ADR-0013 and ADR-0016 session/controller details)
- Date: 2026-10-27
- Input: audit `feedback.md` findings B1, H2 (god crate), M-level IPC findings

## Context

The previous `desktop-session` mixed compilation, process stepping, trace recording,
DTO projection and the execution controller, and its control flow compared IPC error
strings. The rebuilt engine (ADR-0020) needs an application layer that is thin, typed
and testable without a host.

## Decision

- `desktop-session` is an **application adapter**: one public method per use case
  (knowledge, curriculum, projects, workspace lifecycle, Build & Load, PLC mode, simulation
  control, operation, projections, troubleshooting, grading). It owns the single open
  workspace, the library, the project store and progress, and contains no PLC, process,
  network or grading semantics.
- Internal errors are a typed `AppError` (wrapping `EngineError` and `ProjectError`);
  mapping to `IpcError` codes happens only at the public boundary.
- Continuous simulation uses an opaque `RunLease` (workspace id + run id). The host calls
  `tick(lease)` and receives a typed `TickOutcome` (`Stale` or `Advanced` with the next
  wall-clock period); it never inspects error codes. One cycle per wake, no catch-up.
- Build & Load in Basic mode is one backend transaction (compile, validate, link, download,
  PLC RUN, simulation run); request revisions order concurrent loads (`SUPERSEDED`).
- Grading copies the inputs, releases the state lock and runs an isolated assessment.
- IPC v8: DTO-only `app-contract` (serde, serde_json, ts-rs), `deny_unknown_fields`,
  contract version in every request, exact u64 as decimal strings, narrowed TypeScript
  unions. The frontend validates every response with combinators typed against the
  generated declarations.

## Alternatives considered

Exposing engine types through IPC (rejected: couples UI to internals); multiple
workspaces (deferred: no product need yet); a frontend timer driving ticks (rejected:
UI must not drive simulation time).

## Consequences

The host is a list of command adapters plus a pacing loop. Adding a use case means a DTO,
an adapter method, a command and a validator; architecture tests keep the dependency
direction.

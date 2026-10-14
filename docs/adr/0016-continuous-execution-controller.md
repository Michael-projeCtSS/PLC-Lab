# ADR-0016: Deterministic continuous execution controller

- Status: Authorized by M4B; technical-lead source review required after validation
- Date: 2026-10-14

## Context

M0–M4A implementation, independent validation and source review are accepted. M4A source 89383b24e49ffb2782db481a635e6d75cf13f3a3 is the semantic baseline. M4B authorizes 1× Run/Pause/Resume/Stop above the existing runtime, process and trace. It supersedes their earlier manual-only application restrictions without changing core semantics.

## Decision

Keep three layers: deterministic desktop-session control, asynchronous native host pacing, React presentation. Application Mode is STOPPED/RUNNING/PAUSED, separate from RuntimeState READY/FAULTED. One internal execute_one_cycle calls the accepted Runtime::single_scan, advances the process only on success, restages next-scan sensors and records a coherent boundary. Manual scans and authorized continuous ticks call this same function under the existing mutex.

Run/Resume issues a private RunLease(session ID, checked monotonic epoch). Only the mode-changing caller receives driver ownership. Duplicate Run returns a snapshot without a second driver/event. Pause/Stop invalidate authorization under the mutex; Resume allocates a fresh epoch. No old worker can scan after a confirmed Pause/Stop. Old workers may finish waiting and exit on a stale lease. Correctness needs no task cancellation race or timer accuracy. Exhaustion refuses Run/Resume before changing state/history.

The only production pacing exception is apps/desktop/src-tauri/src/execution_driver.rs. It uses Tokio monotonic sleep before the first and every next tick. Each wake attempts one bounded cycle through the blocking pool; it never catches up. The configured cycle period is a read-only pacing duration, never derived from elapsed host time. A host delay of 137 ms still advances the simulation by its configured 100 ms. desktop-session contains no clock, duration, sleep, Tokio or Tauri dependency.

Pause preserves the application association but disallows manual scans and topology/editor/reset operations. Stop preserves exactly the same PLC/process state and enables manual scans/edit/reload. USER input staging remains serialized and available in all modes; PROCESS sensors remain protected. Load checks STOPPED both when reserving its intent and immediately before commit. Latest-intent ordering, including a failed newer intent, is preserved.

Runtime faults halt/invalidate before recording the single ScanFaulted observation; no automatic ExecutionStopped row is fabricated. Internal runtime diagnostics keep their APP category. Process/observation/host-worker failures fail closed, preserve the last state/history, surface an application error and require explicit replacement/unload where the session cannot be observed. No retry loop or automatic reset exists.

IPC migrates v6→v7. Live and historical observations include executionMode and a read-only executionRunId. The ID only correlates wake notifications; it cannot authorize a tick and is never accepted in a control request. Commands accept version/session only. The host emits an identity-only ExecutionWake; React rereads snapshots and incremental trace pages. One in-flight reader and one dirty bit per channel bound work. Session/source/request/run guards discard obsolete signals/replies. History selection remains pinned by TraceSequenceId, with an explicit eviction notice.

Only core:event:allow-listen and core:event:allow-unlisten are added to the main local window capability. Browser emission, OS plugins, filesystem/network access and remote origins remain unavailable. Tokio time is a direct host-only dependency already present transitively.

## Alternatives and migration

Browser timers, wall-clock-derived PlcTime, a second execution engine, catch-up bursts, optimistic mode prediction and cancellation-only authorization violate determinism or ownership. A generic scheduler/actor framework adds unnecessary scope. No persisted schema exists. A default-off trace-core test-support feature provides only a near-exhaustion constructor for session saturation tests; it is not a production persistence API. Accepted trace retention/append semantics are unchanged.

See [CONTINUOUS_EXECUTION.md](../spec/CONTINUOUS_EXECUTION.md). Stop after M4B validation for architectural review; M5 is not authorized.

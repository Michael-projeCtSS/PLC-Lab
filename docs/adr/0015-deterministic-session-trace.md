# ADR-0015: Deterministic session boundary trace

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [ADR-0019](0019-causal-observation-and-compact-trace.md). Kept for engineering history.

- Status: Authorized by M4A; technical-lead source review required after validation
- Date: 2026-10-14

## Context

M1/M2/M3A semantics and M3A source review at c72a880d39ca442a49f93dce6351ada56f3a4c33 are accepted. Manual execution now needs canonical history before continuous execution. ADR-0006 reserved trace-core; M4A implements boundary observations without instruction-level instrumentation.

## Decision

TRACE IS OBSERVATION, NOT EXECUTION AUTHORITY. Runtime/process -> authoritative session observation -> recorder -> bounded history -> typed IPC -> Timeline. Trace never determines values, time, TON, scan results, physical integration or sensors. Neither plc-runtime nor process-simulation depends on trace-core.

trace-core depends only on plc-types and supplies generic TraceBuffer<T>, nonzero checked u64 TraceSequenceId, exact TracePoint and cursor pages. desktop-session retains typed Rust observations with core identities/values, visible TON state, diagnostics and public physical boundary observations. No JSON or app-contract DTO is stored in history. Live and historical DTO conversions share the same observation mapping.

Each successful manual mutation constructs its observation and records under the existing session mutex. A Ready student scan fault records its coherent rolled-back boundary once; an already Faulted call and internal execution failure do not fabricate a cycle. Failed/stale commands and reads do not append. Accepted LoadIntentId and RuntimeSessionId algorithms remain unchanged.

Use 4096 events per session, oldest-first eviction without renumbering and exact dropped count. Sequence exhaustion marks SATURATED, retains history and stops recording without failing/rolling back PLC/process execution. Query limits are 1..256; future cursors are rejected. Cursor gaps mean some requested events were evicted, not that values can be reconstructed.

IPC v6 carries exact u64 decimal strings. Query uses the existing blocking worker. React refreshes incrementally after manual observations, with session/source/effect-generation guards and no polling. Selection shows Historical Trace only; it cannot restore or change live state. Historical source navigation reuses the accepted Monaco span bridge and is disabled for stale source.

## Alternatives and consequences

Runtime tracing dependencies, racing recorder threads, browser-generated history, scan-number identity, unbounded arrays and JSON mutation-path storage violate the boundary or scope. Persistence/replay/streaming require future authorization. The bounded history is suitable for current manual diagnostics; it is not unlimited evidence for future grading. No third-party dependency or core semantic change is required.

See [TRACE_MODEL.md](../spec/TRACE_MODEL.md).

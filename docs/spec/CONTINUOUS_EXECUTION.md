# Deterministic continuous execution

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [PRODUCT_ARCHITECTURE.md](../PRODUCT_ARCHITECTURE.md). Kept for engineering history.

M4B adds application control above the accepted runtime. Host time paces. SimulationClock determines simulated time. Pause != Stop. Stop != Reset.

```text
Host pacer (wait configured period before every attempt)
  -> private run lease -> RuntimeSessionService mutex
  -> execute_one_cycle -> existing Runtime::single_scan
  -> Process step(configured period) -> stage next sensors
  -> coherent observation -> bounded trace
  -> identity-only wake -> typed snapshot/history queries -> React
```

## Modes and controls

| Core state | Mode    | Run/Resume  | Pause       | Stop        | Single Scan | Source/topology/reset                      |
| ---------- | ------- | ----------- | ----------- | ----------- | ----------- | ------------------------------------------ |
| READY      | STOPPED | Run         | unavailable | unavailable | available   | available under existing association rules |
| READY      | RUNNING | unavailable | available   | available   | unavailable | unavailable                                |
| READY      | PAUSED  | Resume      | unavailable | available   | unavailable | unavailable                                |
| FAULTED    | STOPPED | unavailable | unavailable | unavailable | unavailable | explicit Reset Runtime available           |

Run changes only mode/control metadata and appends ExecutionStarted. It performs no synchronous scan. Resume from PAUSED appends ExecutionResumed and allocates a fresh lease. Pause appends ExecutionPaused after invalidation; Stop appends ExecutionStopped after invalidation. Duplicate/no-op controls append nothing and allocate no worker. Stop preserves memory, staged inputs, TON, clock, count, process, sensors and trace. Only explicit Reset Runtime or Reset Process invokes the accepted reset operation. USER inputs remain stageable while RUNNING/PAUSED; PROCESS ownership rejects writes in every mode.

Load replacement, unload, manual scan, runtime reset, attach/detach and process reset require STOPPED in Rust and UI. Pure compile and read-only observations remain safe backend operations. Source/editor/Use Example/Compile/Load/topology/reset controls are disabled during RUNNING/PAUSED and explain: “Stop execution to edit or reload the program.” No editing action implicitly stops execution.

Source editing is also locked while a Run/Resume request awaits confirmation, preventing an in-flight start from losing its source association. The displayed mode still changes only from a backend response.

## Lease, ordering and quiescence

RunLease has private session ID and epoch fields. It can only originate in start_execution, never a browser request. Authorization requires current session, RUNNING and exact active lease under the session mutex. The epoch uses checked u64 addition. APP-EXECUTION-EPOCH-EXHAUSTED refuses a new Run/Resume without altering PLC/process/mode/history. Pause, Stop and faults clear the active lease under that same mutex. A scan already owning the mutex may finish before Pause acquires it; after Pause returns, every old tick is a no-op. Stop has the same guarantee. Resume/new session never reauthorizes an old lease.

Load reservation still allocates the accepted independent LoadIntentId; a newer failed intent still supersedes an older preparation. Both reservation and final commit check STOPPED. A candidate prepared before Run cannot replace the now-running session or change its trace, active lease, generation or runtime. Rejection is APP-EXECUTION-ACTIVE. Intent/session exhaustion retain their accepted errors.

## Pacing and failures

Only the native execution_driver module sleeps. Default 1× period: 100,000 µs. It waits before the first tick, attempts exactly one cycle, emits a wake, then waits again. No interval catch-up, elapsed-time integration, adaptive delta or speed multiplier exists. Delayed host work slows presentation/throughput but never changes simulated durations, TON, values or trace timestamps.

The shared cycle preserves the accepted transaction and next-scan feedback. Student failure records exactly one ScanFaulted after mode becomes STOPPED and authorization is invalidated. Count/time/writable memory/TON/process roll back as before. Stale ticks do nothing. Explicit Reset Runtime is required to recover. Internal runtime failures retain internalApplicationError; process/observation/worker failures stop closed and return APP errors without fabricated ScanCompleted/ScanFaulted/ExecutionStopped events or retries. Worker errors invalidate their exact lease before notifying; stale failure cleanup cannot stop a newer run.

## Observations, notifications and history

IPC v7 adds executionMode and nullable read-only executionRunId to RuntimeSnapshotDto/TraceEntryDto. SessionLoaded has STOPPED/null; the last issued run identity remains visible after Pause/Stop/fault. Control requests contain only contractVersion/sessionId. The host-only continuous_tick is absent from the command handler list.

execution-state-changed carries only contractVersion/sessionId/runId. It cannot carry snapshots, PLC values or execution authority. React matches the active source/session/run, then queries get_runtime_snapshot and incremental get_trace_page. Readers each allow one in-flight chain and one dirty bit; bursts coalesce into a follow-up, with no promise queue or browser execution timer. Mutation generations prevent old reads overwriting Pause/Stop/reload confirmations. Wrong-run/session wakes are ignored. History uses canonical Rust pages, never inserted event payloads.

After Run/Resume confirmation, one coalesced read also catches a terminal tick whose wake preceded delivery of that confirmation. A bounded 16-page history chain may request a coalesced follow-up if Run appended more events during the read. Known fail-closed APP errors allow explicit Unload, still checked by the backend, so an unavailable session can be replaced without guessing a STOPPED snapshot. Other source/topology operations remain locked until that authoritative unload completes.

Selecting history leaves live updates running. The selected sequence remains pinned while the live tank advances. Eviction removes the unavailable observation and shows an explicit no-longer-retained notice. Trace eviction/saturation cannot control execution. Test equality projects out controller mode/run identity and absolute trace IDs displaced by control rows; every PLC/process value, staged/committed/attempted image, TON state, fault, count and virtual timestamp remains exact.

## Evidence

Headless tests issue authorized ticks directly without sleeps. They cover equivalence, exact control trace, stale epochs, Stop preservation, fault rollback, USER/PROCESS authority, Load race, exhaustion, eviction and saturation. Host tests verify wait/tick/notify ordering and exit without exact wall-clock assertions. Native Windows tests use actual Run/Pause/Resume/Stop, observe quiescence across several periods, execute Tank scans 8/9 automatically, inspect pinned history during Run, and verify TON continuity plus manual scan after Stop. Both CI lanes retain all earlier semantic regressions; see M4B_REPORT.md for actual results.

# Desktop runtime session

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [PRODUCT_ARCHITECTURE.md](../PRODUCT_ARCHITECTURE.md). Kept for engineering history.

M2B separates editor source, pure compile metadata, loaded executable, backend runtime session, immutable snapshot and UI presentation. This is manual educational execution, without process/network/trace/grading services.

## Commands and ownership

Compile != Load != Execute. compile_st creates no runtime. load_st_program accepts contractVersion/source and independently compiles, validates, lowers and constructs Runtime with the Rust application defaults (100 ms, 10,000 operations). Configuration is read-only in this milestone. Failed preparation preserves the previous session; successful prepare-then-swap allocates the next canonical decimal u64 session ID.

desktop-session owns one Mutex-protected optional session and a checked generation counter. set_runtime_input, single_scan, reset_runtime, unload_program and get_runtime_snapshot require contractVersion/sessionId. Stale/unloaded generations return APP-RUNTIME-STALE-SESSION. No implicit current-session targeting exists. All mutation/observation DTOs are produced within one locked operation. Compilation/lowering occurs outside the lock, and async Tauri commands use the blocking pool.

## Load intent ordering (M2B-R1)

After checking the IPC version, every Load briefly locks the service to reserve the next nonzero checked u64 LoadIntentId and record it as latest_load_intent. This identity is backend-only and independent of RuntimeSessionId. Preparation has no service reference and performs compilation, validation, lowering and Runtime construction outside the mutex. Before swapping, commit locks again and requires its intent to match latest_load_intent. A superseded candidate returns APP-RUNTIME-STALE-LOAD without changing the session or allocating a session ID; it never returns another program's snapshot.

A newer reservation immediately invalidates older in-flight candidates, even if the newest preparation later fails. Failed preparation preserves the previously committed runtime; an older candidate cannot become a fallback. Only successful committed replacements advance RuntimeSessionId. Failed loads consume intent identities but do not consume session identities. Unload never resets either counter. Load ordering means mutex reservation order, not worker preparation completion or wall-clock timestamps.

Intent allocation uses checked addition, never wraps or reuses IDs. Exhaustion returns APP-RUNTIME-LOAD-INTENT-EXHAUSTED before preparation, retaining the committed session and counters. Session-ID exhaustion retains its existing APP-RUNTIME-GENERATION behavior. Frontend source revisions, request generations and expected-session checks remain unchanged: obsolete stale-load errors are discarded; an unexpected error on the current request invalidates its execution association and requires explicit reload. No automatic retry occurs.

## Values and observations

BOOL uses {type:"BOOL",value:boolean}; INT uses bounded i16 value; REAL uses the checked finite binary32 PlcReal; TIME uses {type:"TIME",microseconds:"unsigned decimal u64"}. TIME never crosses as a JSON number. Session/variable/instance IDs are separate opaque decimal-string identities, not memory addresses. Variables contain ID, original name, type, Input/Output/Local role and typed value. TON instances use their own IDs and contain name/kind/Q/ET; active_since is absent.

A complete snapshot contains sessionId/programName/state, completedScans, simulationTimeMicros, cyclePeriodMicros, operationBudget, variables, functionBlocks and optional fault. Counters/budget use exact decimal strings. Inputs show the staged editable value for the next attempted scan; output/local values and TON outputs are committed observations. Input IDs, permissions and exact types are independently checked by the existing Runtime::set_input. Invalid requests do not corrupt staging or committed values.

## Manual scan, fault and reset

Single Scan directly calls Runtime::single_scan without sleep or wall-clock pacing. Click speed has no semantic effect. Successful scans advance exactly one configured cycle; completed count and time are returned by Rust. For TON PT 300 ms and true input, four 100 ms scans start at 0/100/200/300 ms, observe ET 0/100/200/300 ms and Q false/false/false/true, then report completed times 100/200/300/400 ms.

Student faults are domain snapshot data: core diagnostic code/severity/message/category, optional scalar source span and resolvable related variable. They retain the authoritative Faulted state and rolled-back outputs/locals/TON/time/count. Internal diagnostics retain their APP category/code; transport/version/lock/worker failures remain application errors. Runtime and compiler markers have distinct owners and labels. Stale-source faults are not highlighted on edited text. Explicit reset calls Runtime::reset: all images including staged inputs return to defaults, TON Q/ET reset, fault clears and time/count become zero. Unload discards the executable but preserves editor/compile metadata and never aliases reset.

Design TODO before continuous/background execution: reconsider whether application-internal execution failures need a separate IPC/recovery envelope. For M2B-R1, ScanError::Internal remains snapshot data with category internalApplicationError, distinguishable from student runtimeFault; core semantics and recovery transport are unchanged.

## UI association and input commit

Load requires current Compile success. Single Scan requires that success, the exact loaded editor revision, a matching backend session, Ready state and no runtime request in flight. Editing immediately marks SOURCE CHANGED, disables execution/input mutation and clears source-bound runtime markers; old values remain explicitly labeled as the previous loaded source until reload or unload. Failed load disables the executable association without destroying the previous backend session. Local request generations and expected session IDs reject old replies, including replacement/unload/unmount races. No timestamps identify requests.

BOOL commits immediately; numeric inputs use local text drafts and an Apply action. INT drafts request bounded integers, REAL drafts request finite binary32-compatible decimal numbers and TIME drafts request integer microseconds only. No frontend ST literal parser exists. Last authoritative snapshots survive rejected input commits; error feedback stays visible. Only Input rows have editors; outputs/locals are read-only. Run/Pause/Stop remain disabled. React formats values and renders snapshots; it never predicts PLC results, count, time, TON or rollback.

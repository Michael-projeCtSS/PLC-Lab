# M4B — Deterministic Continuous Execution Controller

Date: 2026-10-14. M0–M4A gates and source reviews are accepted. M4A reviewed source: 89383b24e49ffb2782db481a635e6d75cf13f3a3. This work starts from its report commit 725719c76a45a141661b2e3ebabe2c82a76f321f, implements M4B only and stops for technical-lead source review.

## Completed

Implemented deterministic STOPPED/RUNNING/PAUSED control, private checked run leases, one shared manual/continuous cycle, native 1× pacing, canonical control traces, strict IPC v7, confirmed toolbar controls, locked active source/topology, and bounded authoritative live snapshot/history refresh. USER inputs remain stageable; PROCESS sensors remain protected. Historical selection stays pinned during Run and explicitly reports eviction. Run/Resume pending-source and early terminal-wake races are covered.

## Files

| Area              | Important files                                                                                                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Domain controller | crates/desktop-session/src/execution.rs, lib.rs, process.rs, trace.rs; Cargo.toml                                                                                                     |
| Native host       | apps/desktop/src-tauri/src/execution_driver.rs, commands.rs, lib.rs; Cargo.toml; main window capability                                                                               |
| Contract          | crates/app-contract/src/session.rs, trace.rs, lib.rs; execution_contract.rs and trace_contract.rs                                                                                     |
| IPC               | packages/ipc/src/execution.ts, runtime.ts, trace.ts, generated.ts and validator tests                                                                                                 |
| Presentation      | packages/ui/src/LabShell.tsx, useRuntimeSession.ts, useSessionTrace.ts, coalesced-refresh.ts, StructuredTextEditor.tsx, monaco-host.ts, ProcessPanel.tsx, TimelinePanel.tsx and tests |
| Validation        | tests/e2e/continuous-smoke.mjs and existing native regressions; tests/architecture/execution.test.mjs and boundary guards; scoped workflow screenshot                                 |
| Exhaustion seam   | Default-off trace-core test-support feature and near_exhaustion_for_test constructor only                                                                                             |
| Documentation     | ADR-0016, CONTINUOUS_EXECUTION.md, RUNTIME_SEMANTICS.md, trace/diagnostics/testing/features, architecture/boundary, development/roadmap/checklist/ADR index, this report              |

The main application page remains packages/ui/src/LabShell.tsx. Its accepted workspace layout and Tank renderer are preserved. Cargo.lock adds a direct host reference to the already-transitive Tokio package; no new package version or domain dependency is introduced.

## Architecture

ADR-0016 documents the explicitly authorized controller. Core types/IR/runtime/clock/TON/REAL/ST/physical integration and existing LoadIntent semantics remain unchanged. Core crates still have no React/Tauri dependency. The only added window capabilities are local event listen/unlisten; browser emission, remote origins and OS plugin privileges are absent.

## Execution architecture

```text
Host Pacer: wait configured period
  -> private session/epoch lease
  -> RuntimeSessionService mutex
  -> shared execute_one_cycle
  -> PLC Runtime -> Process step -> next-scan sensors
  -> coherent observation -> bounded Trace -> Snapshot
  -> identity-only Tauri wake
  -> typed snapshot + incremental trace queries
  -> React controls / Tank / Timeline / Diagnostics
```

## Authority model

Rust owns execution, time, values, process, mode, leases and trace. Host time only paces. SimulationClock supplies semantic time. React never evaluates PLC logic, integrates physics, derives sensors, inserts notification state into history, or issues automatic scan requests. One shared cycle preserves the accepted transaction and sensor feedback boundary. Trace remains observation, never authority.

## Execution modes

READY+STOPPED permits Run and Single Scan. READY+RUNNING permits Pause/Stop. READY+PAUSED permits Resume/Stop and forbids manual scans. FAULTED is always STOPPED; all four execution buttons are unavailable until explicit Reset Runtime. Pause/Stop preserve memory, staged inputs, TON, process/sensors, count, time and history. Stop enables edit/reload/manual scan; Pause retains the frozen run association. Neither invokes any reset.

Source, Use Example, Compile/Load, topology/reset/unload actions require Stop. The pending Run/Resume request also locks editing without predicting mode. USER staging is mutex-ordered during all three modes. Known fail-closed APP errors offer explicit backend-checked Unload so an unavailable session can be replaced; they never fabricate a STOPPED observation. A Load prepared before Run rechecks STOPPED at commit and cannot replace the active session or disturb its trace/lease/IDs.

## Run lease

RunLease has private RuntimeSessionId/checked epoch fields. Only the mode-changing start caller receives driver ownership; duplicate Run allocates no worker/event. Tick authorization checks session, RUNNING and exact active lease under the existing mutex. Pause/Stop/fault invalidate before returning; a previously locked cycle may complete before they acquire the lock, but no old worker can scan afterward. Resume gets a new checked epoch. Replacement/unload cannot alias an old lease. Exhaustion refuses Run/Resume while preserving state/history and returns APP-EXECUTION-EPOCH-EXHAUSTED.

executionRunId is read-only correlation metadata, never a lease or accepted control argument. Strict control requests contain only contractVersion/sessionId. continuous_tick is not a Tauri command.

## Pacing

Only apps/desktop/src-tauri/src/execution_driver.rs uses host duration/Tokio sleep. It waits the configured 100000 us before the first and every following attempt, issues at most one bounded cycle, then waits again. No elapsed-time delta, interval catch-up, burst or speed multiplier exists. A delayed 137 ms host wake still advances virtual time by 100 ms. Headless controller/domain tests use no sleeps. Host tests check ordering and termination, not exact real-time accuracy.

## Control trace proof

The executed real-ST test Load -> Run -> Tick -> Tick -> Pause -> Resume -> Tick -> Stop yields exactly:

| Sequence | Event            | Completed scans | Simulation time |
| -------- | ---------------- | --------------- | --------------- |
| 1        | SessionLoaded    | 0               | 0 ms            |
| 2        | ExecutionStarted | 0               | 0 ms            |
| 3        | ScanCompleted    | 1               | 100 ms          |
| 4        | ScanCompleted    | 2               | 200 ms          |
| 5        | ExecutionPaused  | 2               | 200 ms          |
| 6        | ExecutionResumed | 2               | 200 ms          |
| 7        | ScanCompleted    | 3               | 300 ms          |
| 8        | ExecutionStopped | 3               | 300 ms          |

Each control row records its post-transition mode without changing PLC count/time. Duplicate controls append nothing. The original eight event meanings, ordering, capacity 4096, page maximum 256, exact IDs, eviction and saturation behavior remain intact.

## Manual vs continuous equivalence

The same actual ST, initial state and input staging are run through 20 manual cycles and 20 authorized ticks for a Tank+TON+counter fixture and a staged-input TON fixture. Complete private core snapshots match exactly at each cycle, including staged/attempted/committed images, TON state, faults, scan count and virtual time. Tank/sensors and every ScanCompleted semantic observation match. Comparison excludes control rows, their absolute sequence displacement and controller mode/run identity only. No PLC/process value or timestamp is normalized.

## Tank automatic proof

Actual Use Example -> Compile -> Load -> Attach -> Run uses the real Tauri/WebView2 host driver. The harness never invokes continuous_tick or generates these scans manually. Actual canonical observations:

| Trace sequence | Completed scans | Simulation time | Level permille | High | Inlet applied | Mode    |
| -------------- | --------------- | --------------- | -------------- | ---- | ------------- | ------- |
| 13             | 8               | 800 ms          | 800            | TRUE | OPEN          | RUNNING |
| 14             | 9               | 900 ms          | 800            | TRUE | CLOSED        | RUNNING |

Scan 8's process interval creates High=true after the PLC read its preceding false input. Scan 9 reads that staged High and closes the inlet. Live labels/SVG/count/time and Timeline correlate with actual Rust snapshots/pages. The pre-existing nine-manual-scan proof and historical scan 8/9 viewport checks also pass.

## Pause proof

Native Tank Pause at scan 3, time 300 ms preserves the last completed cycle. Two complete Rust observations 550 ms apart and their trace pages are identical. Native TON Pause at scan 2 independently preserves nonempty real TON state. Old-lease rejection is also proven without sleeps on both platforms.

## Stop proof

Native Tank Stop at scan 9, time 900 ms preserves the complete last cycle and physical level 800. Observations/history stay identical over 550 ms. Single Scan then advances exactly to scan 10 and 1000 ms. Native TON Stop preserves its last cycle over the same interval. Resume's canonical row equals the preceding paused PLC/process/TON/count/time; later scans continue. No reset command is used by Stop/Resume.

## Fault proof

A genuine actual-ST divide-by-zero during an authorized tick rolls back count/time/memory/TON, changes mode to STOPPED and invalidates the lease before exactly one ScanFaulted observation. Stale ticks append nothing; Run remains rejected until explicit Reset Runtime. Existing actual Tank fault tests prove complete plant preservation through the same shared cycle. Host worker/observation/process failures remain APP errors, stop closed and fabricate no student fault or completion. Stale worker failure cannot stop a newer lease. No automatic reset/retry loop exists.

## UI notification model

The lightweight ExecutionWake contains only v7/sessionId/runId. React discards obsolete session/run/source notifications and old request responses, then rereads authoritative snapshot/history. Each reader has one running chain and one dirty bit; bursts coalesce rather than queue promises. History reads cap each chain to 16 pages and coalesce follow-up for newly appended rows. One read after Run/Resume confirmation catches terminal wakes that preceded its delivery. Confirmed Pause/Stop cannot be overwritten by older reads.

Native selection #4 remains pinned with unchanged historical text while live scans advance in RUNNING. Selection never pauses or mutates the session. Eviction shows a no-longer-retained notice and removes the unavailable inspector. The accepted renderer remains snapshot-driven.

## Tests

VALIDATED CODE SHA: **db5e357e49090d8adcb14c6f847a0e37afdf6887**

Repository: **Michael-projeCtSS/PLC-Lab — PUBLIC**, preserving the owner's explicit visibility direction. Credential-free HTTPS origin; normal branch pushes, no force, collaborator, secret, policy or visibility changes.

Workflow run: [38613113329](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38613113329). Both jobs genuinely executed; artifact result revisions equal the validated SHA. All selected tests have zero failures/ignored tests. Raw passing test-name multisets match Windows/Linux within each portable profile, including the same fixed-bit REAL, TON, rollback/determinism, ST, load-intent, Tank, trace and controller assertions.

| Independent job                | Job ID       | Result |
| ------------------------------ | ------------ | ------ |
| Windows canonical validation   | 116021239359 | PASS   |
| Linux headless core validation | 116021239907 | PASS   |

| Downloaded artifact  | Artifact ID | GitHub digest                                                           |
| -------------------- | ----------- | ----------------------------------------------------------------------- |
| linux-core-1         | 11918808230 | sha256:fdcc6d902650d037c0bacf745de343a99465d430e736c73fbda79ead4d4212a6 |
| windows-validation-1 | 11917714126 | sha256:60eacdfa1fd9941f6c1d2d0f605801dc77f8920ae8e5c4fade259721d2d036c4 |

| Command/evidence                | Actual result                                                                                                            |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| npm run format:check            | PASS locally and hosted canonical                                                                                        |
| npm run typecheck               | PASS locally and hosted canonical                                                                                        |
| npm run lint                    | PASS ESLint + Rust workspace clippy, all targets, -D warnings                                                            |
| npm run test:frontend           | PASS: 192 tests in 15 files                                                                                              |
| npm run test:architecture       | PASS Windows: 27, no skips; Linux: 25, two inapplicable Windows BAT guards                                               |
| npm run build                   | PASS production build in hosted canonical                                                                                |
| npm run check, hosted Windows   | PASS: 309 executed Rust tests + frontend/architecture/build                                                              |
| npm run ci:windows              | PASS canonical + 305 focused debug + 245 focused release + real native smoke                                             |
| npm run ci:core                 | PASS fmt/clippy + 305 debug + 245 release + portable architecture                                                        |
| Native Windows                  | PASS real Tauri/WebView2 Run/Pause/Resume/Stop, automatic Tank, TON, manual-after-Stop, pinned history; zero page errors |
| Cross-platform REAL             | PASS identical selected fixed-bit/subnormal/canonical-zero/nonfinite/nesting assertions in debug/release                 |
| Controller/session              | PASS: 67 Rust tests, including 10 new deterministic controller tests                                                     |
| app-contract                    | PASS: 27 Rust tests, including strict v7 control transport                                                               |
| trace-core / process-simulation | PASS: 6 / 13 tests, existing semantics retained                                                                          |
| Host pacing                     | PASS: 2 executed native-host Rust unit tests in canonical workspace                                                      |
| Publication/secret audit        | PASS current working tree and Git index; no findings; protected semantic paths unchanged                                 |
| Owner-host npm run check        | BLOCKED at a Rust executable launch, os error 4551; never counted as PASS                                                |
| Owner-host native attempt       | BLOCKED at native launch, spawn UNKNOWN; never counted as executed evidence                                              |

Earlier local native execution found an enabled Unload control during Run; its backend guard already rejected it. The UI guard was corrected before initial publication and the full actual native workflow passes. No assertions were weakened. Early local compilation errors were fixed before publication. No application-control/Defender setting was changed or bypassed.

Published M4B implementation commits: 5c279ea79478b8fea0499228458ea874463e3167 (controller), 10671edd7e28b819316510df23abaaefe83de675 (early terminal wake/live paging/error recovery), db5e357e49090d8adcb14c6f847a0e37afdf6887 (pending-start source guard). These are authorized implementation refinements, not CI infrastructure repairs. **CI-only repair commits: none.** Earlier source runs are supplementary; the gate uses only the exact final validated SHA above. This report is a later documentation-only commit, separate from tested code; its commit identity is available in Git history and the completion message.

Evidence artifacts contain per-stage logs/result JSON, native result JSON and scoped screenshots. The independent evidence audit verifies revision, executed counts, matching names, required regression assertions and native snapshot/trace proofs. No compilation-only, BLOCKED, NOT_RUN or skipped required job is represented as PASS.

## Regression audit

Compared with the accepted M4A report baseline, AGENTS, plc-types, plc-ir, plc-runtime, simulation-core, process-simulation, languages/st, network/grading placeholders, accepted type/IR/runtime/TON/ST/source-mapping specifications and historical validation reports are byte-identical. LoadIntent ordering remains intact, with an added STOPPED precondition at reservation/commit. Trace append/eviction/saturation algorithms are unchanged; only the explicitly permitted default-off exhaustion seam is added. CI selections, pinned actions, read-only permissions, required stages and failure propagation remain unchanged; only a scoped continuous Tank screenshot is added.

## Acceptance criteria

| #   | Requested criterion                                                 | Result/evidence                                                                                    |
| --- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 1   | Manual and continuous execution share one authoritative cycle path. | PASS: One execute_one_cycle; architecture guard counts exactly one runtime scan call.              |
| 2   | No second PLC runtime exists.                                       | PASS: Accepted plc-runtime remains the only engine; protected source diff is empty.                |
| 3   | ExecutionMode is distinct from RuntimeState.                        | PASS: Private application Mode and separate core RuntimeState; typed v7 observations.              |
| 4   | STOPPED/RUNNING/PAUSED semantics are explicit.                      | PASS: ADR-0016/specification and controller/button-matrix tests.                                   |
| 5   | Stop does not reset PLC.                                            | PASS: Exact core state comparison across Stop; native last-cycle vs stopped observation.           |
| 6   | Stop does not reset process.                                        | PASS: Native last-cycle/process equality and tank remains 800 permille after Stop.                 |
| 7   | Pause preserves complete state.                                     | PASS: Full native before/after equality plus stale-lease tests.                                    |
| 8   | Manual Single Scan is forbidden while RUNNING/PAUSED.               | PASS: Backend APP-EXECUTION-ACTIVE and UI/native disabled controls.                                |
| 9   | User Inputs remain stageable while active.                          | PASS: Real-ST ordered staging test; active-mode frontend input availability.                       |
| 10  | process-owned Inputs remain protected.                              | PASS: All three modes reject PROCESS writes; existing ownership/native regressions.                |
| 11  | topology/reset/load mutations require STOPPED.                      | PASS: STOPPED guards in service, final Load commit and UI/native controls.                         |
| 12  | load commit race against new Run is safe.                           | PASS: Deterministic prepared Load -> Run -> rejected commit; active lease still executes.          |
| 13  | continuous tick requires a matching run lease.                      | PASS: Private RunLease identity + exact active lease/mode/session checks under mutex.              |
| 14  | Pause invalidates old lease before returning.                       | PASS: Pause clears lease under mutex before its recorded response; old tick is Stale.              |
| 15  | Stop invalidates old lease before returning.                        | PASS: Stop clears lease under mutex before response; old tick is Stale.                            |
| 16  | Resume creates a new lease.                                         | PASS: Checked increment issues a distinct Resume lease; native resumed row preserves state.        |
| 17  | stale old lease cannot execute.                                     | PASS: Old leases reject after Pause/Stop/Resume/replacement/unload; stale UI tests.                |
| 18  | driver exists only in host layer.                                   | PASS: Only native execution_driver owns pacing and worker tasks.                                   |
| 19  | desktop-session contains no wall-clock pacing.                      | PASS: Source/dependency guard excludes clocks, durations, sleeps, Tokio and Tauri.                 |
| 20  | one host wake creates at most one PLC cycle.                        | PASS: Host paced_loop order test and single shared cycle per attempted tick.                       |
| 21  | host delay never determines PlcTime delta.                          | PASS: Configured PlcTime used unchanged; manual/leased exact state equality.                       |
| 22  | no catch-up burst exists.                                           | PASS: One sleep per iteration; no interval, elapsed-time integration or catch-up.                  |
| 23  | first automatic tick occurs after one cycle period.                 | PASS: Host test asserts wait precedes first and every subsequent tick.                             |
| 24  | genuine runtime fault auto-stops execution.                         | PASS: Actual ST divide fault halts/invalidate before coherent fault observation.                   |
| 25  | fault creates only one ScanFaulted attempt row.                     | PASS: One canonical ScanFaulted; repeated old tick is Stale; no auto-reset.                        |
| 26  | internal errors remain application errors.                          | PASS: APP worker/error test and distinct diagnostic category; no fabricated event.                 |
| 27  | control trace events are canonical/backend-generated.               | PASS: Typed Rust recorder creates four control kinds under the session mutex.                      |
| 28  | control events do not change scan/time.                             | PASS: Executed exact eight-row control test asserts each count/time pair.                          |
| 29  | live snapshots include ExecutionMode.                               | PASS: RuntimeSnapshotDto has strict mandatory executionMode.                                       |
| 30  | historical trace includes ExecutionMode.                            | PASS: Internal typed observation/TraceEntryDto preserve post-transition mode.                      |
| 31  | IPC v7 remains strict and exact.                                    | PASS: Rust/TS v7 request, enum, exact decimal identity and unknown-field tests.                    |
| 32  | Tauri event is notification only, not state authority.              | PASS: ExecutionWake contains only version/session/run identity; no state payload.                  |
| 33  | frontend re-queries authoritative state.                            | PASS: Actual native wake -> get_runtime_snapshot and get_trace_page integration.                   |
| 34  | continuous refresh is bounded/coalesced.                            | PASS: Serial coalescer with one dirty bit per reader; burst tests and bounded pages.               |
| 35  | stale wake signals cannot contaminate current session.              | PASS: Run/session/source/request guards; old signal/reply and unload tests.                        |
| 36  | Timeline updates during Run.                                        | PASS: Actual native automatic trace rows appear while Run advances the UI.                         |
| 37  | historical selection remains read-only during Run.                  | PASS: Pinned native historical text/sequence stays unchanged during automatic scans.               |
| 38  | Tank live SVG remains snapshot-driven.                              | PASS: Unchanged TankRenderer geometry from Rust process snapshots; native correlation.             |
| 39  | N continuous domain ticks equal N manual cycles semantically.       | PASS: 20 cycles per fixture; complete core snapshots and semantic observations equal.              |
| 40  | trace eviction does not affect execution.                           | PASS: Small-capacity direct-tick test continues 20 cycles with exact dropped count.                |
| 41  | trace saturation does not affect execution.                         | PASS: Near-MAX test saturates recorder while clock/process advance through 20 cycles.              |
| 42  | Run/Pause/Stop native smoke passes.                                 | PASS: Actual hosted Tauri/WebView2 controls, no mocks or test-issued ticks.                        |
| 43  | Pause native quiescence is proven.                                  | PASS: Native PAUSED full snapshot/history unchanged for 550 ms.                                    |
| 44  | Stop native quiescence is proven.                                   | PASS: Native STOPPED full snapshot/history unchanged for 550 ms.                                   |
| 45  | Resume continuation is proven.                                      | PASS: Native ExecutionResumed equals paused PLC/process/TON/count/time; later cycles advance.      |
| 46  | manual scan after Stop is proven.                                   | PASS: Native Single Scan after Stop advances count by one and time by 100000 us.                   |
| 47  | automatic Tank scan 8/9 causal history is proven.                   | PASS: Actual automatic canonical Tank scan 8/9 observations and snapshot/UI correlation.           |
| 48  | all previous M0→M4A regressions pass.                               | PASS: All accepted debug/release/REAL/TON/ST/load/process/trace and canonical regressions execute. |
| 49  | Windows CI passes.                                                  | PASS: Exact source SHA: all Windows stages, 309 Rust, frontend, production and native PASS.        |
| 50  | Linux CI passes.                                                    | PASS: Exact source SHA: Linux fmt/clippy, 305 debug, 245 release and 25 portable guards PASS.      |
| 51  | no forbidden scope expansion entered M4B.                           | PASS: Protected semantic paths unchanged; only requested controller/host/UI/trace integration.     |

## Assumptions

Use the accepted fixed 100 ms period and 1× pacing only. Read-only executionRunId correlates notifications while private RunLease remains the only tick capability. Keep the accepted PLC_RUNTIME_SEMANTICS.md unchanged and add the requested RUNTIME_SEMANTICS.md as an application-control companion. Compare semantic cycle observations independently from controller metadata/control-row sequence displacement. Keep the owner's existing PUBLIC repository visibility and accepted shell layout. Human checklist items are a review guide, not claimed owner sign-off.

## Known limitations

Owner-host application control blocked final local Rust/native launches; independent Windows execution is recorded separately and genuinely passes. Linux is headless; real native validation targets Windows/WebView2. Host 1× pacing is best effort and has no hard real-time guarantee. History remains bounded, in memory and cycle-boundary only. Technical-lead M4B source review is still required.

## Scope audit

No speed multiplier/turbo, instruction stepping/breakpoint, replay/rewind/restore, trace export/persistence, project persistence, exercise/grading/scoring engine, fault injection framework, network/protocol/physical-device connectivity, LAD/FBD/STL editor, new PLC type/instruction/timer/counter, 3D or process editor entered M4B. No PLC semantic assertion or security policy was weakened.

## Next dependency

STOP for technical-lead review of M4B controller/lease/pacer/IPC/live-refresh integration. Do not begin M5 or any later milestone.

## Gate

M4B GATE: PASS

Required implementation, independent Windows/Linux execution and real native acceptance pass for the exact validated code SHA above. This does not claim technical-lead source approval or authorize M5.

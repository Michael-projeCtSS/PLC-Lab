# M4A — Deterministic Trace Foundation and Timeline

Date: 2026-10-14. M1C/M2A/M2B and M3A validation/source review/final gates are accepted. M3A technical-lead review targeted c72a880d39ca442a49f93dce6351ada56f3a4c33; the inherited repository baseline is its documentation commit 7a31523e04d1709e421c64754f5228e0ff213f34. This batch implements M4A only and stops for technical-lead trace/Timeline source review.

## Completed

Implemented generic headless ordered TraceBuffer, typed session boundary observations, eight lifecycle event kinds, bounded read-only queries, strict IPC v6 and incremental request-driven Timeline presentation. Historical selection displays recorded PLC/TON/process/diagnostic state without changing live state. Added deterministic/negative/retention/load-order/contract/frontend/architecture/native tests and integrated them into both existing CI lanes.

## Files

| Area            | Important files                                                                                                                                                                                      |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Generic history | crates/trace-core/src/lib.rs                                                                                                                                                                         |
| Application     | crates/desktop-session/src/trace.rs, lib.rs, process.rs, Cargo.toml; tests/trace_session.rs                                                                                                          |
| Contract/host   | crates/app-contract/src/trace.rs, session.rs, lib.rs; tests/trace_contract.rs; apps/desktop/src-tauri/src/commands.rs and lib.rs                                                                     |
| IPC             | packages/ipc/src/trace.ts, trace.test.ts, runtime.ts, generated.ts, index.ts                                                                                                                         |
| Presentation    | packages/ui/src/useSessionTrace.ts, trace-history.ts, TimelinePanel.tsx, their tests; LabShell.tsx, styles.css, RuntimeShell.test.tsx; tests/frontend/trace-fixtures.ts                              |
| Native/quality  | tests/e2e/trace-smoke.mjs, process-smoke.mjs and desktop-smoke.mjs; tests/architecture/trace.test.mjs and existing boundary/CI guards; scripts/ci.mjs; scoped Timeline screenshots in validation.yml |
| Documentation   | ADR-0015, TRACE_MODEL.md, diagnostics/test/feature specifications, architecture/boundary docs, development/roadmap/human checklist/ADR index, this report                                            |

Cargo.lock adds one workspace path dependency from desktop-session to trace-core. No third-party library is added.

## Architecture

ADR-0015 documents the authorized boundary. trace-core depends only on plc-types; plc-runtime and process-simulation remain independent of trace-core. desktop-session records within the existing serialization boundary. Core execution, physical integration, accepted load intent ordering and committed session allocation remain unchanged.

## Trace architecture

```text
Authoritative Runtime/Process
  -> typed coherent Session Observation
  -> Trace Recorder under the same session mutex
  -> bounded ordered Trace History
  -> typed read-only IPC query
  -> Timeline / Historical Trace inspector
```

Trace is observation, never execution authority. Internal observations retain core SymbolId/FunctionBlockInstanceId, original names/type/role, PlcValue, input ownership, visible TON Q/ET, RuntimeState/Diagnostic including source provenance, and public TankSnapshot. No JSON or app-contract TraceEntryDto is retained internally. Shared observation-to-DTO conversion presents both live and historical values; no second process or PLC evaluator exists. Private TON active_since, process integration internals, IR and plans are absent.

## Sequence model

TraceSequenceId is a strongly typed nonzero checked u64, scoped to one RuntimeSessionId and beginning at 1 on successful replacement. RuntimeSessionId identifies committed runtimes; LoadIntentId independently orders load requests. Completed scan count counts successful PLC scans. SimulationClock supplies exact PlcTime. None of those three is trace identity.

Input/process actions retain current scan/time while receiving distinct sequence IDs. A student fault also receives a sequence while rolled-back scan/time remain unchanged. Reset can return PLC scan/time to zero without resetting trace sequence. No wall-clock timestamp, random identity or unordered Rust-map iteration contributes to canonical history.

## Retention

Default capacity: **4096 entries per RuntimeSession**. VecDeque evicts oldest first; sequence IDs remain unchanged and droppedCount is exact. The highest u64 sequence can be used once. Checked allocation then reports **SATURATED**, stops recording and retains history; it never wraps, reuses IDs, creates a student fault or rolls back/fails already accepted execution. Generic exhaustion tests exercise this otherwise theoretical boundary without sleeps.

Query limit: **1..256 entries**. null cursor begins at oldest retained; N returns sequence>N. gap=true exactly when N<oldest-1, meaning some entries requested after N were lost. N=oldest-1 loses none of its requested entries. null cursor has gap=false while droppedCount still exposes lost history. N=latest returns empty/gap=false. Future cursor is APP-TRACE-CURSOR; bad limit is APP-TRACE-LIMIT. Stale sessions are rejected by the existing generation guard. Query/snapshot reads append nothing.

History is bounded and in memory only. Frontend aggregation is also bounded to retained backend entries; old rows are removed when retention metadata advances. No file persistence/export exists.

## Event semantics

| Event           | Exact recording condition                                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| SessionLoaded   | Latest-intent successful commit creates new history; initial authoritative observation at sequence 1                            |
| InputStaged     | Accepted user input staging; post-stage value and semantic/runtime variable identity, unchanged scan/time                       |
| ProcessAttached | After validated attachment and initial sensor staging                                                                           |
| ScanCompleted   | After complete successful PLC scan, optional physical step and next-sensor staging; exactly one coherent full-cycle observation |
| ScanFaulted     | Ready attempt produces a genuine RuntimeFault; authoritative rolled-back PLC/process observation and source-backed diagnostic   |
| RuntimeReset    | After accepted Runtime::reset and preserved-plant sensor restage                                                                |
| ProcessReset    | After physical reset; PLC memory/count/time/fault preserved                                                                     |
| ProcessDetached | After plant/ownership removal; no-process observation, sensor Inputs now USER                                                   |

An already Faulted call performs no scan and appends no event. Failed compile/load, superseded load intent, failed attach, stale command, wrong input type, owned-sensor rejection and missing-scene reset/detach append nothing. Internal runtime failures are not relabeled ScanFaulted or PLC-RUNTIME faults. If an accepted later lifecycle observation contains an internal diagnostic, its original category/code is preserved. A failed closed process/session pipeline with no coherent boundary fabricates no entry. Normal eviction/saturation cannot fail a mutation. Capture and append complete under the same mutex before returning its live DTO.

Successful new Load replaces runtime/history and starts sequence 1; failed/stale Load preserves existing history; Unload discards it. Snapshot/query/compile are observational/pure operations.

## Tank trace proof

Actual shared Tank source is compiled and loaded, explicitly attached, then scanned nine times. Real native history contains **SessionLoaded, ProcessAttached, nine ScanCompleted**, sequences 1..11.

| Sequence | Event           | Completed scans | Simulation time | Level              | High  | Inlet applied |
| -------- | --------------- | --------------- | --------------- | ------------------ | ----- | ------------- |
| 1        | SessionLoaded   | 0               | 0 ms            | no scene           | —     | —             |
| 2        | ProcessAttached | 0               | 0 ms            | 0%                 | FALSE | FALSE         |
| 3..9     | ScanCompleted   | 1..7            | 100..700 ms     | 10..70%            | FALSE | TRUE          |
| 10       | ScanCompleted   | 8               | 800 ms          | 80% (800 permille) | TRUE  | TRUE          |
| 11       | ScanCompleted   | 9               | 900 ms          | 80% (800 permille) | TRUE  | FALSE         |

The trace records the accepted M3A cycle boundary. Scan 8 reads preceding High=false and opens the inlet; that interval creates High=true for the next scan. Scan 9 closes the inlet. No history is regenerated from present outputs. Selecting sequence 10 shows historical OPEN while the live process already shows CLOSED at scan 9. Selecting sequence 11 shows historical CLOSED. Native tests correlate both selected observations with an actual Rust get_trace_page response and verify complete live snapshot/history equality before and after selection/query.

## Fault trace proof

Real source `Result := 100 / Divisor;` with Divisor=0 produces ScanFaulted, PLC-RUNTIME-003, category runtimeFault, exact source expression span, completedScans=0, simulationTime=0 and committed Result=0. The event equals the returned core-backed live fault snapshot. A second call against Faulted records nothing. RuntimeReset subsequently records Ready/count 0/time 0/fault cleared.

A separate deterministic real-source sequence includes Tank, user inputs, visible TON Q/ET, nine scans, division fault, both resets and detach. Two independent sessions produce exactly equal histories without semantic normalization. The fault retains the preceding complete process snapshot at scan 9/time 900 ms. Existing M3A rollback tests remain unchanged and execute.

## Timeline

The compact ordered list displays sequence, event, PLC scan, exact simulation time and summary. Selection displays **Historical Trace**, recorded variables/TON/process/fault and a clear observation-only label. It cannot change or restore live state. Source-backed fault focus reuses the accepted Monaco span bridge; historical inspection and live markers remain separate.

Refresh is incremental after existing successful manual mutation observations, starting from the last retained sequence. Catch-up requests at most 16 pages of 256, bounded by the 4096-entry history. No continuous polling or streaming exists. Merge deduplicates by sequence, retains ordering and removes evicted rows. Session identity, observation token and local generation comparison discard obsolete pages on replacement/unload/source association changes. Large sequence/count/time strings use BigInt-safe formatting, never unsafe JS integer conversion.

New Load replaces displayed history; Unload clears it. Source edit preserves the last accepted previous history visibly STALE, cancels pending pages and disables source navigation. New page requests require a current loaded-source association; stale presentation remains a frozen previous observation. Dropped-history and internal SATURATED indicators are explicit. Query failures appear as trace query errors without fabricating observations or changing runtime authority.

## Authority audit

Neither trace-core nor Timeline executes PLC logic, advances SimulationClock, integrates physical state, derives sensors, calculates scan results/TON or writes runtime state. React only formats recorded numbers, booleans and exact strings. The only historical action is existing source-span focus. No rewind, restoration, replay or future-state inference is implemented.

## IPC

Contract advances v5 -> **v6**. New TraceSequenceId transport, TraceEventKind, TraceEntryDto, TracePageRequest/Dto and thin get_trace_page command. Entries carry exact sequence/count/time, runtimeState, variables, functionBlocks, optional process/fault and staged Input identity. Metadata carries capacity, oldest/latest sequence, droppedCount, saturated and gap. Every u64-like quantity is canonical decimal text; IDs are nonzero. Rust strict request decoding and TypeScript field/tag/value/ownership/state/ordering/query validators remain authoritative at the transport boundary. The live boundary validator is factored for reuse, retaining all previous checks. Tauri delegates through the existing blocking worker and does not synthesize/reorder/infer observations.

## Tests

| Command / evidence                                                                   | Result                                                                                                                                                                                                                               |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| node scripts/rust.mjs test --locked -p trace-core -p desktop-session -p app-contract | PASS in final Windows/Linux debug/release selections: 6 trace-core + 57 desktop-session + 25 app-contract tests; existing 13 process model tests also execute in each full core lane                                                 |
| npm run test:frontend                                                                | PASS locally: 179 tests, including 10 IPC trace, 8 Timeline and 9 hook/merge tests                                                                                                                                                   |
| npm run test:architecture                                                            | PASS: Windows 25 checks/0 skipped; Linux 23 portable checks plus two intentionally inapplicable Windows BAT guards                                                                                                                   |
| npm run format:check                                                                 | PASS: Prettier and Rust fmt                                                                                                                                                                                                          |
| npm run lint                                                                         | PASS: TypeScript ESLint and all-workspace Rust clippy with warnings denied                                                                                                                                                           |
| npm run build                                                                        | PASS: production frontend build and type checking                                                                                                                                                                                    |
| npm run validate                                                                     | PARTIAL locally: format/typecheck/lint/architecture/frontend passed; application control blocked a Rust desktop-session test executable (os error 4551), so the command failed and later stages did not execute through that command |
| npm run test:desktop                                                                 | PASS locally, including final styling: actual Tauri/WebView2 IPC v6, legacy regressions and native historical Tank proof; no page errors                                                                                             |
| Temporary ignored compact native harness (1028x750 actual WebView viewport)          | PASS: same real Tauri/IPC assertions, historical selection and legacy Diagnostics containment; no simulation/transport mocks                                                                                                         |
| npm run ci:windows                                                                   | PASS: final canonical format/typecheck/lint/clippy, 295 Rust + 179 frontend + 25 architecture tests, production build, 293 debug + 233 release tests and actual native smoke                                                         |
| npm run ci:core                                                                      | PASS: final core fmt/clippy, 293 debug + 233 release tests and 23 portable architecture checks; established two Windows-only guards inapplicable                                                                                     |
| Same deterministic REAL assertions/debug-release passing names on Windows/Linux x64  | PASS: identical 293 debug and 233 release passing test names on both platforms, including fixed binary32/subnormal/canonical-zero/nonfinite assertions                                                                               |
| Hosted native Timeline/Tank proof and screenshot review                              | PASS: actual hosted Tauri/WebView2 IPC v6, legacy regressions and 11-event historical Tank proof; no page errors; final 1028x750 scan 8/scan 9/default screenshots reviewed                                                          |

The early focused working-tree Rust run executed and passed; subsequent canonical rerun was blocked by host policy. That local block is not a passing test or substitute for final exact-SHA independent execution. Host security settings were not changed. Existing ts-rs serde-attribute notices and frontend bundle-size notice are informational; decoding/drift/validators/build checks execute. Linux's two unchanged Windows BAT guards are intentionally inapplicable there and remain required on Windows. No required semantic/trace/native test is ignored, weakened or replaced by compilation-only evidence.

## Independent evidence and publication

Repository: **Michael-projeCtSS/PLC-Lab — PUBLIC**. Before publication, GitHub CLI/API confirmed its public state. The owner explicitly directed: “Keep public and push M4A”, overriding the prior private-only publication constraint. No visibility change was performed by this implementation task. Current working-tree/index publication audits found no credential/build/profile findings; origin remains credential-free HTTPS. No collaborator, secret, permission/security-policy change or force push was made.

Branch: codex/m4a-trace-timeline. VALIDATED CODE SHA: **89383b24e49ffb2782db481a635e6d75cf13f3a3**. Workflow: [38606018519](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38606018519), push, attempt 1. Windows job **115996989544**: success, 2026-10-14 17:21:52–17:29:03 UTC. Linux job **115996988773**: success, 17:21:52–17:22:39 UTC. Every required stage has PASS, exit code 0 and evidence for the exact validated SHA. Downloaded and inspected **windows-validation-1**, ID **11916615658**, digest `sha256:3a808069b3fd2746e21b8b651e653285e6801f1fee583b8ec91d94b920e3203c`; **linux-core-1**, ID **11914679758**, digest `sha256:b6fedb0bdc5b61efe48bdf0e85cd39740e3ed0eb1218ab66dfd11c782a488318`. The ignored evidence audit verifies actual executed test names/counts, required trace/REAL/TON/load-order/process tests, exact revisions and successful stage exits. It verifies genuine native 11-entry Rust history, sequence/time/process observations, selected scan 8/9 entries, unchanged live state and no query/selection events. Final hosted historical/default screenshots were visually reviewed.

This report is a later documentation-only commit; identify it with `git log -1 -- docs/M4A_REPORT.md`. It does not replace the executed source SHA. Evidence downloads/audit remain under ignored artifacts/m4a-ci. Test artifacts are not trace persistence/export functionality. Initial implementation SHA **c3dd91a4f15f0d02d8bcb3d741a61ce36b95fb4b**, run [38602639342](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38602639342): Linux and Windows canonical/debug/release passed, but native startup failed because the debugging endpoint existed before a WebView page. This was not counted as native PASS. CI infrastructure repair **5d250fd4df1981a0378d6322eaa235e505500251** adds a bounded 20-second actual-page wait, preserving every native assertion. Run [38604347039](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38604347039) then executed the historical Tank proof, but the final legacy Diagnostics containment assertion failed: the empty Timeline minimum was 170 px. Presentation-only correction **89383b24e49ffb2782db481a635e6d75cf13f3a3** restores that minimum to the original 106 px; all assertions remain. A real compact 1028×750 local native check passed before publication. The final run above passes both lanes including native/default layout. There was one CI-infrastructure repair and one single-value presentation correction; neither alters PLC/process/IR/TON semantics or weakens tests.

## Regression audit

Protected-source diff against accepted 7a31523e04d1709e421c64754f5228e0ff213f34 is empty for AGENTS.md, plc-types, plc-ir, plc-runtime, simulation-core, process-simulation, languages/st, network/grading cores, approved type/symbol/IR/runtime/TON/ST/source-mapping specifications, historical validation docs and M2A/M2B/R1/M3A reports. Accepted RuntimeSessionId/LoadIntentId allocation/order algorithms remain unchanged. Existing core/runtime/TON/REAL/rollback/ST/Tank/ownership/native tests remain; only new trace recording and observation mapping are introduced above them. Live semantic observations match unchanged M3A expectations, including exact 0->10->...->80%/next-scan closure and distinct resets. The dedicated trace test additionally compares every recorded Tank event to its corresponding live snapshot.

CI retains pins, runners, read permissions, stage order, failure propagation and all existing test selections. It adds trace-core to both profiles and two scoped historical screenshots. No continue-on-error or assertion relaxation is introduced. One native-startup harness repair and a one-value empty Timeline height correction are disclosed below; no PLC/process or assertion semantics change.

## Acceptance criteria

| #   | Criterion                               | Result/evidence                                                                                                                      |
| --- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Reusable headless trace-core            | PASS: generic buffer/point/identity/page, plc-types dependency only                                                                  |
| 2   | Runtime independent of trace-core       | PASS: unchanged runtime dependency graph and guard                                                                                   |
| 3   | Observational authority                 | PASS: mutation -> capture -> append; no execution in trace/UI                                                                        |
| 4   | Sequence independent of scan/time       | PASS: distinct typed ID and repeated/reset-point tests                                                                               |
| 5   | Bounded history                         | PASS: fixed 4096 entries, VecDeque and small-capacity tests                                                                          |
| 6   | Explicit dropped history                | PASS: exact metadata/cursor gaps and visible warning                                                                                 |
| 7   | Deterministic ordering                  | PASS: checked sequence and complete real-source equality                                                                             |
| 8   | Load starts new trace                   | PASS: sequence 1 initial observation/replacement tests                                                                               |
| 9   | Failed Load preserves history           | PASS: actual failure and stale-intent correction test                                                                                |
| 10  | InputStaged post-stage observation      | PASS: identity/value/count/time test                                                                                                 |
| 11  | ProcessAttached sensor observation      | PASS: live/trace equality and native actual attach                                                                                   |
| 12  | One full-cycle ScanCompleted            | PASS: real-source nine-cycle equality; no intermediate observation                                                                   |
| 13  | Fault without physical advancement      | PASS: genuine fault/rollback/golden tests                                                                                            |
| 14  | RuntimeReset event                      | PASS: exact reset observation and count/time zero                                                                                    |
| 15  | ProcessReset event                      | PASS: distinct physical reset preserving PLC state                                                                                   |
| 16  | ProcessDetached event                   | PASS: no-process/USER ownership observation                                                                                          |
| 17  | Failed/stale commands append nothing    | PASS: input/type/owned/stale/missing/binding/load/internal rejection tests                                                           |
| 18  | PLC variables retained                  | PASS: typed identity/name/role/value observations                                                                                    |
| 19  | Visible TON state retained              | PASS: Q/ET golden and inspector tests; no epoch                                                                                      |
| 20  | Process boundary retained               | PASS: actual Tank observations, not regenerated outputs                                                                              |
| 21  | Fault source provenance retained        | PASS: core-backed exact division expression span and equal live fault                                                                |
| 22  | Bounded paginated query                 | PASS: 1..256 validation, ordered cursor tests                                                                                        |
| 23  | Stale session query rejected            | PASS: replacement/unload/backend generation tests                                                                                    |
| 24  | Incremental UI retrieval                | PASS: last-cursor refresh and bounded catch-up tests                                                                                 |
| 25  | Old replies cannot contaminate session  | PASS: explicit generation/identity/source/unload/replacement tests                                                                   |
| 26  | Timeline read-only                      | PASS: only selection/source focus actions                                                                                            |
| 27  | History selection preserves live state  | PASS: final actual hosted native history/selection, correlated Rust pages, unchanged live snapshot and reviewed scan 8/9 screenshots |
| 28  | Tank scan 8/9 causal difference visible | PASS: final actual hosted native history/selection, correlated Rust pages, unchanged live snapshot and reviewed scan 8/9 screenshots |
| 29  | Existing results unchanged              | PASS: protected-source/live-trace/M3A regression checks                                                                              |
| 30  | No wall-clock trace authority           | PASS: exact core PlcTime and architecture guards                                                                                     |
| 31  | Windows CI                              | PASS: final job 115996989544; all canonical/debug/release/native stages executed                                                     |
| 32  | Linux CI                                | PASS: final job 115996988773; core quality/debug/release/portable stages executed                                                    |
| 33  | Actual native Timeline proof            | PASS: final actual hosted native history/selection, correlated Rust pages, unchanged live snapshot and reviewed scan 8/9 screenshots |
| 34  | Run/Pause/Stop disabled                 | PASS: existing and new native/frontend checks                                                                                        |
| 35  | No scope expansion                      | PASS: scope/dependency audit                                                                                                         |

## Assumptions

Use the requested default of 4096 entries and page maximum of 256 entries. Reject future cursors; distinguish a truly lost requested entry from cursor=oldest-1. Mark saturation as soon as the maximum identifier is used. Preserve accepted internal runtime failure transport; record ScanFaulted only for genuine student RuntimeFault. Reuse the fixed Tank scene/source and existing worker/mutex/Monaco/CI boundaries. Publication follows the owner's explicit public-repository direction above.

## Known limitations

History is bounded/in-memory and contains session/cycle observations only. Native validation targets Windows/WebView2; Linux validation is headless. The owner host blocked one Rust test executable, though the separate actual native smoke passed; independent execution results are reported distinctly. The human checklist is a review guide, not owner sign-off. Technical-lead trace/Timeline source review remains required before continuous execution.

## Scope audit

No continuous Run, Pause/Stop execution, scheduler, wall-clock pacing, speed multiplier, background trace polling/thread/streaming, replay/rewind/restore/fork, instruction stepping, grading/scoring/exercise engine, production fault injection, network/protocol/Modbus/OPC UA, LAD/FBD/STL, new PLC instruction/timer/counter, process editor, 3D, project persistence or trace file persistence/export entered M4A. No protected PLC/physical semantics changed.

## Next dependency

Stop for technical-lead source review of M4A trace/session/IPC/Timeline implementation. Do not begin continuous execution or any later milestone. Validation alone does not authorize Run/Pause/Stop.

## Gate

M4A GATE: PASS

Required independent validation and actual native historical Timeline acceptance passed for the exact source SHA above. Owner-host application-control and prior failed runs are reported separately. Stop for technical-lead source review; continuous execution remains unauthorized.

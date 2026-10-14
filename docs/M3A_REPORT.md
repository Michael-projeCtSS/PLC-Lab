# M3A — Manual Tank Filling process vertical slice

Date: 2026-10-14. M1C/M2A/M2B final gates and the M2B-R1 Load intent correction are accepted by the technical lead. This batch implements M3A only and stops for process/application/UI source review. It does not authorize continuous execution or claim the eventual trace/grading portion of DEMO-001 is complete.

## Completed

Implemented a headless Rust integer Tank model, source-visible interface binding, explicit attachment/reset/detachment, backend sensor ownership, one complete PLC/process cycle per successful Single Scan, coherent IPC v5 snapshots, SVG presentation and a shared example source with explicit replacement confirmation. Added real-source tests, strict DTO/client tests, UI regressions, architecture guards and real native Tank acceptance to the existing independent CI lanes. Existing M0->M2B checks remain.

## Files

| Area           | Important files                                                                                                                                                                                             |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Physical model | crates/process-simulation/Cargo.toml, src/lib.rs, tests/tank.rs                                                                                                                                             |
| Application    | crates/desktop-session/src/lib.rs, src/process.rs, tests/process_session.rs                                                                                                                                 |
| Contract/host  | crates/app-contract/src/process.rs, session.rs, lib.rs; tests/process_contract.rs; apps/desktop/src-tauri/src/commands.rs and lib.rs                                                                        |
| IPC/renderer   | packages/ipc/src/generated.ts, runtime.ts, process.test.ts; packages/ui/src/ProcessPanel.tsx, tank-example.ts, RuntimeVariables.tsx, useRuntimeSession.ts, LabShell.tsx, styles.css and corresponding tests |
| Source data    | scenes/tank-filling/example.st, shared by frontend and real-source integration tests                                                                                                                        |
| Quality/native | scripts/ci.mjs, tests/architecture/process.test.mjs and updated boundary/CI guards; tests/e2e/process-smoke.mjs and desktop-smoke.mjs; one scoped Tank screenshot path in validation.yml                    |
| Documentation  | ADR-0014, PROCESS_SIMULATION.md, diagnostics/test/feature specifications, architecture/boundary docs, development/roadmap/human checklist/README, this report                                               |

## Architecture

ADR-0014 documents the authorized process crate and application orchestration boundary. Accepted PLC, IR, ST, clock and load-order semantics remain unchanged.

## Process architecture

```text
PLC Runtime -> committed Outputs -> desktop-session bindings
  -> headless process-simulation -> private physical state + sensors
  -> desktop-session input staging -> next PLC scan

combined authoritative snapshot -> typed IPC -> React/SVG
```

process-simulation depends only on plc-types; it knows no SymbolIds, source language, runtime, host or renderer. desktop-session orchestrates both the existing runtime and the new model under the existing mutex. simulation-core remains the accepted clock leaf; the runtime never imports the physical model. ADR-0014 records the explicitly authorized new crate placement and preserves ADR-0005's model/renderer separation. No runtime transaction redesign was necessary.

## Tank model

Private integrated level: u16 integer permille, initial 0, clamped to 0..1000. Low is level<=200 (20%); High is level>=800 (80%). Fixed inlet rate: 1000 permille/second. Fixed pump rate: 800 permille/second. Rates/thresholds are Rust-owned and exposed read-only.

Each enabled contribution is checked i128(rate)*i128(cycleMicros)/1,000,000. Truncate each nonnegative contribution independently; discard fractional permille per step without a remainder. Add inlet and subtract drain before clamping; both actuators at a full tank therefore retain 1000 rather than draining an incorrectly pre-clamped intermediate. At 100 ms the four effects are +100, -80, +20 and 0. Even u64::MAX microseconds fits the intermediate range and clamps deterministically. No wall time or separate process clock exists.

LevelPercent is level/10.0 constructed through finite PlcReal. It never becomes integrated state. Headless tests verify canonical zero, 3.3 binary32 bits, exact thresholds/effects, maximum-duration behavior, reset and repeated snapshot equality. The physical model owns no PLC memory or hidden __tank_* symbols.

## Binding model

| Source name (ASCII case-insensitive) | Role       | Type |
| ------------------------------------ | ---------- | ---- |
| LevelLow                             | VAR_INPUT  | BOOL |
| LevelHigh                            | VAR_INPUT  | BOOL |
| LevelPercent                         | VAR_INPUT  | REAL |
| InletValve                           | VAR_OUTPUT | BOOL |
| DrainPump                            | VAR_OUTPUT | BOOL |

Find immutable descriptors produced from compiler declarations and lowered runtime memory roles, then retain actual SymbolIds privately in the application. Preserve original spelling. Extra variables and shifted IDs are supported; no numeric ID/address convention or same-type inference exists. Validate all five names/types/roles and all staging preconditions before replacing/staging an attachment. Missing/wrong-type/wrong-role declarations produce PLC-PROCESS-001/002/003 with precise expected/found messages. Frontend metadata hints explain the disabled Attach control; Rust independently validates every request.

## Input authority

Inputs begin USER. Explicit attachment assigns PROCESS to the three bound sensors; Outputs/Locals carry null inputAuthority. Sensor editors disappear and PROCESS is visible, but direct backend writes are independently rejected with PLC-PROCESS-004. Unrelated Inputs remain USER. Detach restores USER without clearing staged sensor values, resetting PLC memory or advancing time. Ownership is attached to the current RuntimeSessionId, not editor text or magic IDs.

## Cycle ordering

Sensors(t) already staged -> Runtime::single_scan at t -> commit Outputs(t) and advance core count/time -> physical step for cyclePeriod -> Process(t+dt) -> derive/stage Sensors(t+dt) -> return one combined boundary observation. The mutex covers the entire operation. No observer can see a successful core scan without its corresponding plant/sensor stage, and no zero-delay same-scan feedback loop exists.

## Tank vertical slice

Actual native/source-to-runtime sequence at 100 ms:

| Boundary | Level permille / percentage | Low   | High staged | Inlet applied | Completed time |
| -------- | --------------------------- | ----- | ----------- | ------------- | -------------- |
| Attach   | 0 / 0%                      | true  | false       | false         | 0 ms           |
| Scan 1   | 100 / 10%                   | true  | false       | true          | 100 ms         |
| Scan 2   | 200 / 20%                   | true  | false       | true          | 200 ms         |
| Scan 3   | 300 / 30%                   | false | false       | true          | 300 ms         |
| Scan 4   | 400 / 40%                   | false | false       | true          | 400 ms         |
| Scan 5   | 500 / 50%                   | false | false       | true          | 500 ms         |
| Scan 6   | 600 / 60%                   | false | false       | true          | 600 ms         |
| Scan 7   | 700 / 70%                   | false | false       | true          | 700 ms         |
| Scan 8   | 800 / 80%                   | false | true        | true          | 800 ms         |
| Scan 9   | 800 / 80%                   | false | true        | false         | 900 ms         |

Scan 8 reads the preceding High=false, so the valve remains open while its physical interval creates High=true. Scan 9 reads that new sensor and closes the valve. Native testing uses UI example confirmation, Compile, Load and Attach separately; each mutation is correlated with a real targeted Rust observation. It checks visible percentage, fill geometry, actuator/sensor labels, ownership rejection, resets, detach/pure execution, stale-source blocking, reload scene removal and disabled Run/Pause/Stop. No transport/model mock exists in native acceptance. A separate actual ST source drives the drain from full to 920 after one cycle, then to zero with Low active.

## Reset semantics

| Action         | PLC memory/fault/TON             | PLC count/time | Physical level        | Sensors                             | Last applied actuators                   |
| -------------- | -------------------------------- | -------------- | --------------------- | ----------------------------------- | ---------------------------------------- |
| Reset Runtime  | Existing Runtime::reset defaults | Reset to zero  | Preserved             | Restaged from preserved plant       | Preserved until next successful interval |
| Reset Process  | Preserved                        | Preserved      | Reinitialized to zero | Initial sensors restaged            | Preserved                                |
| Detach Process | Preserved                        | Preserved      | Scene discarded       | Last staged values retained as USER | Scene discarded                          |

The model's headless reset returns its exact default state; application plant reset retains previously applied commands while reinitializing tank state. This documents the actuator-reset ambiguity without applying new PLC outputs or altering PLC semantics. Attachment initializes flags from real current committed outputs, including attachment after prior pure scans. Native acceptance proves Runtime reset at 80% retains High feedback, then Process reset empties the plant while retaining PLC count/time/output state. A following scan resumes filling through real PLC logic.

## Fault semantics

A student PLC fault never advances the model. The integration test first fills a nonempty plant, then actual compatible ST provisionally closes the valve/starts the pump before division by zero. The existing runtime rolls back committed state/count/time; the complete physical/sensor/applied-actuator snapshot stays equal. Faulted rescans do not move it. Failed attachment to this Faulted runtime preserves the old scene. Runtime reset clears the actual fault and preserves/restages the plant. Process reset never clears a PLC fault.

No normal student process arithmetic fault is invented. Fixed configuration and typed duration make valid steps effectively infallible. An impossible process/binding/staging invariant returns APP-PROCESS-INTERNAL and latches that application session closed until explicit replacement/unload; a private corruption test proves failure occurs before normal attachment mutation and no false observation is returned. Accepted internal runtime diagnostics retain category internalApplicationError in snapshots and their deferred envelope review note; no broad recovery framework or core behavior change is added.

## Renderer authority audit

React does not integrate level, calculate flow, derive sensors/actuator physics or advance time. It maps supplied levelPermille to SVG height/percentage and supplied booleans to labels/classes. Sensor positions use read-only supplied thresholds. No animation/timer/frame pacing exists. Owned Inputs have no editors. Source changes mark the old plant STALE and disable scan/process mutations; old replies cannot transfer it to a new source/session. Example insertion requires explicit confirmation and only edits source/invalidation. All process values originate in Rust snapshots.

## IPC

Version 5 adds ProcessSceneKind, ProcessSnapshotDto/TankProcessSnapshotDto, InputAuthority, AttachProcessSceneRequest and attach_process_scene/detach_process_scene/reset_process_scene. The new commands use explicit sessionId/version; attach also names TANK_FILLING. All existing load/input/scan/reset/observation responses carry optional process state and nullable authority metadata. Fields include bounded integer level, sensor/actuator booleans and read-only rates/thresholds. No private integrated implementation state, IR or executable plan crosses IPC. Strict validators reject unknown kinds/fields, malformed values/ranges and incoherent ownership metadata without calculating physical behavior. Existing exact TIME/IDs/counters/budget and finite REAL behavior remains.

## Tests

| Command / evidence                                                                                                          | Result                                                                                                                                                                                                           |
| --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| node scripts/rust.mjs test --locked -p process-simulation -p desktop-session -p app-contract                                | PASS: 13 model + 47 session + 23 contract tests                                                                                                                                                                  |
| node scripts/rust.mjs clippy --locked -p desktop-session -p app-contract -p process-simulation --all-targets -- -D warnings | PASS                                                                                                                                                                                                             |
| npm run test:frontend                                                                                                       | PASS: 152 tests, including fixed DTO renderer/ownership/confirmation/mismatch and process request protections                                                                                                    |
| npm run test:architecture                                                                                                   | PASS: 22 Windows checks; process dependency and renderer guards included                                                                                                                                         |
| npm run validate                                                                                                            | PASS on the owner host: full format/typecheck/lint/clippy, 277 workspace Rust tests, frontend/architecture checks, production build and real native smoke                                                        |
| npm run ci:windows                                                                                                          | PASS: format/typecheck/lint/clippy, 277 Rust + 152 frontend + 22 architecture tests, production build, 275 debug + 215 release tests and actual native smoke                                                     |
| npm run ci:core                                                                                                             | PASS: core fmt/clippy, 275 debug + 215 release tests; 20 portable architecture checks (two unchanged Windows-only BAT guards execute on Windows)                                                                 |
| Actual hosted native Tank/legacy IPC/Monaco/runtime smoke                                                                   | PASS: genuine Tauri/WebView2 IPC v5, legacy checks and complete manual Tank loop; no page errors; hosted 1028x750 screenshot reviewed with full graphic containment, no row overlap and untruncated sensor names |
| Same fixed REAL assertions and all passing debug/release names across Windows/Linux                                         | PASS: identical 275 debug and 215 release passing test names on Windows x64/Linux x64, including fixed binary32/subnormal/zero/nonfinite assertions                                                              |

No required test is weakened, ignored, blocked or substituted with compilation. Linux's two unchanged Windows BAT guards are inapplicable there and remain required/executed on Windows. Existing ts-rs attribute notices remain informational; Rust serde and generated/client drift/validation tests execute. Native evidence and metadata are downloaded under ignored artifacts/m3a-ci; test artifacts are not a production trace recorder.

## Independent evidence

Repository: Michael-projeCtSS/PLC-Lab, PRIVATE; credential-free HTTPS origin. Branch: codex/m3a-tank-process. Implementation SHA: bb279da0c2a4279ce36f39f3716b6e98443769be. First run [38588648414](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38588648414) passed both lanes/native. A UI-only follow-up adds precise interface mismatch hints and three tests. Its run 38590689093 also passed both lanes, but screenshot review identified a compact-window layout defect: the Process grid children extended beyond their row and overlapped the panels below. The final presentation-only commit c72a880d39ca442a49f93dce6351ada56f3a4c33 reserves the full active-scene row, shows complete sensor names and strengthens native checks for row overlap, full graphic containment/viewport visibility and name truncation. No Rust/core/model/IPC/CI change occurred; earlier passing runs are not used as final visual acceptance.

VALIDATED CODE SHA: **c72a880d39ca442a49f93dce6351ada56f3a4c33**. Final run: [38594692123](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38594692123), push, attempt 1. Windows job **115958260333**: success, 2026-10-14 16:02:55–16:14:19 UTC. Linux job **115958260199**: success, 16:02:55–16:04:32 UTC. All required stages have exit code 0 and PASS evidence for the exact validated SHA. Downloaded and inspected artifacts: **windows-validation-1**, ID **11910136005**, digest `sha256:29f122faa0d1d7ae776233ae74f2c14f9f8fc3238c21d4f6bb8da3d72ab0e394`; **linux-core-1**, ID **11908179452**, digest `sha256:e97711e2fb84eb90054d429930ab9d364da1ac3a66a481b4891b9aa2bd8ceef4`. The ignored evidence audit verifies executed test names/counts, exact revisions, stage exits and native sensor/actuator/reset/ownership/staleness observations. The final hosted Tank screenshot was visually reviewed; strengthened assertions additionally prove full graphic containment within its panel and usable viewport, no next-row overlap and complete sensor names.

This report is recorded in a later documentation-only commit. It does not replace the executed code SHA; identify its commit with git log -1 -- docs/M3A_REPORT.md. No CI infrastructure repair or semantic assertion change was needed. The owner-host validation passed for this batch; earlier host-policy failures remain historical records and are not reclassified.

## Regression audit

Protected-source diff against accepted acccab45029570519e10229d35d1454994e8a90c is empty for AGENTS.md, plc-types, plc-ir, plc-runtime, simulation-core, languages/st, trace/network/grading crates and approved type/IR/runtime/TON specifications. Historical validation and M2A/M2B/R1 reports stay unchanged. RuntimeSessionId and LoadIntentId allocation/order algorithms remain unchanged, including newer-failed-load supersession; existing deterministic tests plus an attached-scene test execute. No ST/type/IR/runtime/TON/clock/budget semantics changed. Cargo.lock adds only the new workspace path dependency; no third-party library was introduced. Tauri capability/permission policies and CI pins/runners/read permissions/failure propagation remain unchanged; only required package selections and a scoped screenshot were added.

## Acceptance criteria

| #   | Criterion                                    | Result/evidence                                                                                                 |
| --- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| 1   | Headless Rust physical model                 | PASS: process-simulation depends only on plc-types                                                              |
| 2   | Physical state separate from PLC memory      | PASS: private tank state; IDs live only in app bindings                                                         |
| 3   | Integer authoritative level                  | PASS: bounded u16 with checked i128 integration                                                                 |
| 4   | No wall-clock physics                        | PASS: typed cycle duration only; architecture guard                                                             |
| 5   | No React simulation                          | PASS: fixed snapshot renderer/geometry tests and guard                                                          |
| 6   | Explicit required interface validation       | PASS: all five missing/type/role cases tested                                                                   |
| 7   | Semantic/runtime IDs, no magic addresses     | PASS: metadata lookup and shifted-ID/mixed-case real source                                                     |
| 8   | Process-owned sensors                        | PASS: authority metadata for three Inputs only                                                                  |
| 9   | Backend rejects owned writes                 | PASS: direct Rust and actual native rejection                                                                   |
| 10  | Explicit attach                              | PASS: compile/load/name match never attaches implicitly                                                         |
| 11  | Failed attach preserves state                | PASS: binding failures plus existing valid/faulted scene preservation                                           |
| 12  | Successful Load removes scene                | PASS: headless and actual native new-session observation                                                        |
| 13  | Failed Load preserves scene                  | PASS: non-default scene equality and stale intent protection                                                    |
| 14  | Process advances only after successful scan  | PASS: orchestration success branch and cycle tests                                                              |
| 15  | Failed scan does not move plant              | PASS: complete process snapshot equality after provisional writes/fault                                         |
| 16  | Committed outputs drive step                 | PASS: real ST fill/drain/output tests                                                                           |
| 17  | Sensors staged for next scan                 | PASS: coherent observations and scan 8/9 feedback                                                               |
| 18  | One-scan delay proved                        | PASS: actual source + native nine-scan sequence                                                                 |
| 19  | Runtime reset preserves/restages plant       | PASS: non-default headless and native 80% reset                                                                 |
| 20  | Process reset preserves PLC state/count/time | PASS: output/local/TON/user/fault tests and native distinction                                                  |
| 21  | Detach restores USER                         | PASS: direct and native manual inputs/pure scan                                                                 |
| 22  | Coherent combined snapshots                  | PASS: one locked cycle/mutation/observation and targeted native correlation                                     |
| 23  | Snapshot-only Tank renderer                  | PASS: fixed DTO labels/geometry, no future-value calculation                                                    |
| 24  | Native 0->10->...->80%                       | PASS: actual hosted Tauri/WebView2 UI and targeted Rust observations; nine scans and reviewed native screenshot |
| 25  | High asserts at 80%                          | PASS: exact threshold and source/native scan 8                                                                  |
| 26  | Valve closes next scan                       | PASS: source/native scan 9 retains 800                                                                          |
| 27  | Deterministic drain                          | PASS: actual source and fixed model -80/clamp tests                                                             |
| 28  | 0/100% clamps                                | PASS: both boundary/extreme-duration/headless source tests                                                      |
| 29  | M1/M2 semantics unchanged                    | PASS: protected-source and regression audit                                                                     |
| 30  | Windows CI                                   | PASS: final job 115958260333, canonical/build/debug/release/native stages executed                              |
| 31  | Linux CI                                     | PASS: final job 115958260199, core quality/debug/release/portable checks executed                               |
| 32  | Actual hosted native vertical slice          | PASS: actual hosted Tauri/WebView2 UI and targeted Rust observations; nine scans and reviewed native screenshot |
| 33  | Run/Pause/Stop disabled                      | PASS: frontend/native controls and command surface                                                              |
| 34  | No excluded feature expansion                | PASS: scope/dependency audit                                                                                    |

## Assumptions

Use the explicitly preferred separate process-simulation crate to keep the clock leaf unchanged. Truncate each contribution per cycle without retaining fractions. Preserve last applied actuator flags on application Process reset/Runtime reset; only a later successful interval applies new committed commands. Attachment is suitable only in Ready; explicit Process reset/detach may operate while Faulted without clearing the PLC fault. Reuse the accepted private CI workflow and feature-branch publication. Source data is shared from scenes/tank-filling/example.st; no exercise framework is introduced.

## Known limitations

Only the fixed manual Tank scene/configuration is supported. Native acceptance targets Windows/WebView2; Linux validation is headless. Pane scrolling remains available at compact desktop sizes, with authoritative values/labels and SVG readability checked in native acceptance. The human checklist remains a manual review guide, not an owner sign-off. Technical-lead source review is still required before any continuous execution authorization.

## Scope audit

No continuous Run, Pause/Stop execution semantics, wall-clock pacing, scheduler, speed control, trace/timeline recording, grading/exercise engine, networking/protocol, LAD/FBD/STL, other timer/counter/edge block, user FB, persistence, arbitrary process editor/graph, 3D, CFD/pressure/PID, analog output or hardware I/O entered M3A. No repo visibility/collaborator/secret/security-policy change was made, and pushes were normal without force. Publication audits scan current tree/index and preserve accepted project work.

## Next dependency

Stop for technical-lead review of M3A process-model/application/UI source. Do not begin M3B, M2C or any continuous execution work. Independent validation alone grants no later milestone authorization.

## Gate

M3A GATE: PASS

Implementation, local regression, independent Windows/Linux execution and genuine native manual Tank acceptance passed. Stop for technical-lead source review; no later milestone is started.

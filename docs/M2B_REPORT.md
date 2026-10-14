# M2B implementation report

Date: 2026-10-14. Branch: codex/m2b-runtime-session. M1C and M2A validation/source review are accepted. Independent Windows/Linux validation and actual native execution passed. Technical-lead application/UI source review is the next dependency before M2C/M3 authorization.

PRIVATE repository: [Michael-projeCtSS/PLC-Lab](https://github.com/Michael-projeCtSS/PLC-Lab). VALIDATED CODE SHA: `fc3d49743615c82468e42e1a59b8941139d38052`. Implementation commit: `6811245dd25eab94a55d6a91a0936fde2bf03c83`. The final report is recorded in a later documentation-only commit and does not alter the validated source. Main and accepted history remain unchanged.

## Completed

Added one Rust-owned manual runtime session, explicit load/replacement/unload, typed staged input editing, Single Scan, coherent live scalar/TON snapshots, core runtime faults with source markers, deterministic reset and stale-source/response protection. Compile remains pure. Run/Pause/Stop remain disabled.

## Files

- crates/desktop-session: headless application service and real ST/runtime session tests.
- crates/app-contract/src/session.rs, contract generator/version and runtime_contract tests: strict v4 presentation DTOs and checked IDs/values.
- apps/desktop/src-tauri: one managed service and thin blocking-worker commands.
- packages/ipc/src: generated contract, strict runtime client/validators, shared transport/compilation response code and tests.
- packages/ui/src: runtime hook, live Variables/input drafts, display formatting, separate Monaco markers, shell/status/controls and tests.
- scripts/ci.mjs, architecture/conformance/native tests: all previous gates plus portable application/contract tests and real interactive smoke.
- ADR-0013, desktop session specification and requested architecture/specification/development/checklist/roadmap documentation.
- validation.yml adds only two explicitly scoped native screenshots; permissions, jobs, runners, action pins and gates are preserved.

## Architecture

No M1 core or M2A ST semantics changed. desktop-session is an application crate above the accepted core and ST frontend, with no Tauri/React dependency. The desktop owns one Arc<RuntimeSessionService>. One std::sync::Mutex serializes mutation and observation; preparation occurs outside the lock. ADR-0013 documents the authorized increment.

## Session architecture

Editor Source -> Compile (pure diagnostics/declarations) is separate from Source -> explicit Load -> ST compiler -> existing IR validation -> owned ExecutionPlan -> RuntimeSession -> existing Runtime::single_scan -> coherent RuntimeSnapshotDto -> presentation.

The candidate is compiled/validated/lowered and Runtime constructed before replacement. Only successful snapshot preparation and locked swap replace the prior session. No browser IR, plan or compile result is trusted as executable authority. Core and ST crates never import the session service.

## Compile vs Load

Compile has no access to service ownership and creates no runtime. Load intentionally recompiles exact source at the Rust boundary, with application defaults defined once in Rust: 100,000 microseconds per cycle and 10,000 operations. The UI displays returned configuration and cannot choose it in M2B. Failed load returns compiler diagnostics or application errors and preserves the previous executable/state.

## Session identity

Successful loads allocate checked monotonic nonzero u64 generations, starting at 1. Unload/reset do not reuse generations. Every input/scan/reset/unload/observation request names the intended session; stale targets fail before mutation with APP-RUNTIME-STALE-SESSION. A mutex protects the loaded option and generation. Poisoning and exhaustion fail closed without unsafe code.

React associates load with the exact local editor revision and filters responses by monotonic request generation and expected session ID. Source edits immediately invalidate the executable association and source-bound runtime markers. Last observed old-session values remain explicitly labeled STALE until reload/unload; they are never represented as results of edited source. Pending controls and a synchronous busy guard prevent duplicate mutations. Old replies cannot restore a replaced/unloaded/edited association.

## Runtime snapshot

```text
RuntimeSnapshotDto {
  contractVersion, sessionId, programName, state: READY | FAULTED,
  completedScans: decimal u64 string,
  simulationTimeMicros: decimal u64 string,
  cyclePeriodMicros: decimal u64 string,
  operationBudget: decimal u64 string,
  variables: [{ variableId, name, typeName, role: Input | Output | Local, value }],
  functionBlocks: [{ kind: TON, instanceId, name, q, etMicros }],
  fault: null | { code, severity, message, span, category, relatedVariable }
}
```

Each DTO comes from one locked Runtime observation. Inputs use staged_inputs, output/local values use committed state, and TON comes from function_blocks. No raw storage map, private activation epoch or executable representation is exposed. get_runtime_snapshot is explicitly targeted and used by native tests; the UI adds no polling.

## Typed values

BOOL uses a boolean value, INT an i16 number, REAL the accepted checked finite PlcReal and TIME exact canonical decimal-string microseconds. Requests independently reject malformed tags/fields/IDs, out-of-range integers, non-finite REAL and numeric/overflow/noncanonical TIME. Rust retains binary32 quantization/canonical-zero semantics. Frontend binary32 representability checks validate wire/input representation only and return the received display number; they do not evaluate PLC arithmetic. Maximum finite binary32's shortest JSON decimal is accepted. IDs and count/budget also use exact strings, avoiding unsafe JavaScript integers.

## Input semantics

Only Input rows offer editors. BOOL commits immediately; numeric drafts require Apply/Enter. TIME uses integer microseconds, never a frontend ST literal parser. Backend uses Runtime::set_input for identity/permission/exact-type checking. Staging does not alter committed outputs or time/count; the next attempted scan snapshots it. Rejected input retains last authoritative state and understandable feedback. Reset visibly clears staged inputs along with all other state.

## Single Scan

Single Scan calls the unchanged Runtime::single_scan. It never sleeps or waits a cycle in real time. Successful completion advances one exact configured cycle in Rust; failed scans preserve count/time and committed outputs/locals/FB state. User click speed has no semantic effect. React never increments or predicts time/count/values. Ready/current-source/no-pending requirements gate the button; Faulted disables it until explicit reset.

## TON proof

The final hosted native UI/Monaco/load/input/scan execution passed for the validated SHA. An earlier local interactive working-tree run also passed. Initial state: scan/time/ET 0, Motor/Q false. With Start true, cycle 100 ms and PT 300 ms:

| Scan start | Completed scan | Completed time | Motor / Q | ET     |
| ---------- | -------------- | -------------- | --------- | ------ |
| 0 ms       | 1              | 100 ms         | FALSE     | 0 ms   |
| 100 ms     | 2              | 200 ms         | FALSE     | 100 ms |
| 200 ms     | 3              | 300 ms         | FALSE     | 200 ms |
| 300 ms     | 4              | 400 ms         | TRUE      | 300 ms |

Native UI mutations are correlated against complete, real Rust get_runtime_snapshot replies targeting the exact displayed generation. These values originate from existing TON state, without mocks or frontend calculation. The test also proves same-scan Internal := Start; Motor := Internal from actual source.

## Fault proof

Actual native source Result := 100 / Divisor with default Divisor 0 yields PLC-RUNTIME-003 / runtimeFault / Error, FAULTED, Result 0, completed count 0 and time 0. Backend span is line 4, scalar columns 11–24, exactly `100 / Divisor`. Native geometry assertions verify the runtime Monaco underline covers that expression. Single Scan is disabled, and committed variables match the loaded defaults. Headless session tests additionally prove rollback after a prior non-default committed Result 50.

## Reset

Explicit reset invokes unchanged Runtime::reset and returns the same plan/configuration/session ID with READY, no fault, default staging/input image/output/local/TON state, scan/time 0. Native reset equals the initially loaded snapshot and clears the runtime marker; service tests prove input/TON reset too. Unload instead discards the executable, preserves source/compile metadata and rejects old targeted observation.

## Frontend authority audit

React owns only drafts, association/request identities, formatting and last supplied snapshot. No PLC expression evaluation, assignment prediction, count/time increment, TON transition or rollback calculation exists. Compiler and runtime marker owners are distinct and both use the accepted scalar-to-UTF-16 converter. Transport errors have a compatible IpcTransportError alias; domain compiler/runtime diagnostics stay separate data.

## Tests

| Executed command/check                                 | Result                | Evidence/count                                                                                                                                     |
| ------------------------------------------------------ | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Local npm run check                                    | PASS                  | Format, typecheck, ESLint/workspace clippy, 236 Rust, 20 architecture, 108 frontend and production build before final frontend regression          |
| Final local npm run test:frontend / typecheck / ESLint | PASS                  | 109 frontend tests; strict types/lint                                                                                                              |
| Local focused contract/session debug tests             | PASS                  | 18 contract + 24 session tests, including final full workspace execution                                                                           |
| Latest local npm run validate                          | FAIL at native launch | Canonical checks/build passed; spawn UNKNOWN prevented final native execution                                                                      |
| Local optimized Rust command                           | BLOCKED               | serde_json build script never executed, application control OS error 4551                                                                          |
| npm run ci:windows                                     | PASS                  | Full canonical checks, 236 workspace Rust, 234 focused debug, 174 release, 109 frontend, 20 architecture, production build and actual native smoke |
| npm run ci:core                                        | PASS                  | fmt/clippy, 234 debug, 174 release, all 18 portable architecture checks                                                                            |
| Actual hosted Tauri/WebView2 interactive smoke         | PASS                  | Real compiler/session IPC, staged inputs, committed values, read-after-write, TON, fault/marker/rollback/reset, typed maximum TIME and unload      |

Focused CI debug command selects plc-types, plc-ir, plc-runtime, simulation-core, st-frontend, app-contract and desktop-session. Release selects plc-runtime, simulation-core, st-frontend, app-contract and desktop-session. All required Rust tests actually execute, with zero failed/ignored/filtered cases. No blocked or unrun local requirement is relabeled PASS; independent hosted evidence fulfills the platform requirements.

Superseded harness attempts exposed ambiguous Run/Reset Runtime selection, reply-capture correlation and a leftover capture reference; corrected selectors/targeted observations preserve all runtime assertions. A source-pattern guard also needed whitespace tolerance after Rust formatting. No PLC assertion, semantics, required job or security setting was weakened.

## Independent CI and artifacts

[Final run 38565604434](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38565604434), attempt 1, push of the validated SHA: both jobs SUCCESS. Windows job 115857363232 ran 2026-10-14 12:40:27–12:52:26 UTC; Linux job 115857362913 ran 12:40:26–12:41:53 UTC. Both used Node v24.21.0 and the pinned Rust 1.99.0 on the existing Windows x64/MSVC and Linux x64/GNU runner families.

Downloaded and reviewed stage result records, actual per-test execution logs, full workflow metadata/log and native result.json. Every required final stage has PASS, exitCode 0, null launchError/signal and the exact validated SHA. Sorted passing test names match across platforms: 234 debug and 174 release. These include 24 application-session and 18 contract tests in both profiles, all 77 ST tests, original TON/rollback/clock tests and unchanged fixed binary32/subnormal/canonical-zero, non-reassociation and non-finite REAL assertions. Windows canonical checks execute all 236 workspace Rust tests, 109 frontend tests and 20 architecture checks, including both BAT checks. Linux reports exactly two inapplicable Windows-only BAT skips, which are not counted among its 18 portable passes. No required test was skipped.

Native result.json confirms actual application startup, IPC v4 manual-single-scan capability, unsupported-version rejection, seven panels and empty pageErrors. runtimeSession records full actual initial/default/read-after-write/fault/reset/typed snapshots and the four-scan TON sequence. Visible UI values/time/count/Q/ET are correlated with coherent Rust observations for the exact displayed session. The marker geometry resolves the actual division expression. Run/Pause/Stop remain disabled, source editing disables Single Scan, and unload removes live values. desktop.png, runtime-ton.png and runtime-fault.png were visually inspected. The compact foundation panes scroll and long variable names have tooltips.

- windows-validation-1: artifact 11896556467, digest `sha256:ff054be645825ce58fa6a9066b7f1281c6f8281771d321d384c9a5e36e5dbdd1`.
- linux-core-1: artifact 11894327125, digest `sha256:65a29755ba4e8290a1a9287c476086e644a37693c47d8d0e54cf3a33dbc97dff`.

Workflow retention remains 14 days. Local downloaded evidence and evidence-audit.json are under ignored artifacts/m2b-ci/38565604434. Generated binaries, browser profiles and screenshots were not committed. A fresh publication scan covered all 187 working-tree publication candidates without credential/key/binary findings. Authenticated account and repository privacy were verified; origin remains credential-free HTTPS. No collaborator, secret, permission, security-policy or force-push change occurred.

## CI infrastructure repair

First run [38563765546](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38563765546), SHA 6811245dd25eab94a55d6a91a0936fde2bf03c83: Linux PASS, Windows canonical/debug/release PASS, native FAIL. Playwright checkbox.check asserted an immediate checked state while the controlled input waited for Rust acknowledgement.

One infrastructure-only repair, SHA fc3d49743615c82468e42e1a59b8941139d38052, changes the harness to a normal click, then checks the control after the authoritative snapshot acknowledgement. The actual backend staged-TRUE/value checks remain, and an explicit post-ack isChecked assertion was added. Only tests/e2e/runtime-smoke.mjs changed; application/core semantics and expected PLC results did not. The failed run remains failed; final success comes from the complete new run above.

## Regression audit

Frozen plc-types/plc-ir/plc-runtime/simulation-core and languages/st source/tests remain byte-identical to the accepted M2A state. Historical M1/M2A validation documents are preserved. Core-info capability/version expectations migrate explicitly to v4 manual-single-scan while pre-load/compile-only controls remain disabled. Original runtime/REAL/TON/rollback/clock/ST tests remain mandatory in debug/release on both platforms.

## Acceptance criteria

| #   | Criterion                               | Result | Evidence                                                                   |
| --- | --------------------------------------- | ------ | -------------------------------------------------------------------------- |
| 1   | Compile remains pure                    | PASS   | Unchanged compiler service has no session ownership; architecture guard    |
| 2   | Explicit Load                           | PASS   | Separate Load Program action and command; compile-only UI tests            |
| 3   | Backend recompiles/validates/lowers     | PASS   | Real source preparation uses accepted compiler/validator/lower             |
| 4   | Failed load preserves prior session     | PASS   | Complete prior snapshot equality and generation test                       |
| 5   | One backend-owned runtime               | PASS   | One managed service with optional LoadedSession                            |
| 6   | Stale generations rejected              | PASS   | Input/scan/reset/unload/observation negative tests                         |
| 7   | Serialized mutations                    | PASS   | Mutex boundary, concurrent scans/replacement, poison/exhaustion tests      |
| 8   | Exact typed IPC values                  | PASS   | Checked core types, canonical strings, maximum TIME round trips            |
| 9   | Only inputs editable                    | PASS   | Backend role/type checks and readonly Output/Local UI tests                |
| 10  | Existing set_input API                  | PASS   | Service invokes Runtime::set_input; staging tests                          |
| 11  | Existing single_scan API                | PASS   | Service invokes Runtime::single_scan; actual headless execution            |
| 12  | No JS PLC state/time calculation        | PASS   | Snapshot-only hook and authority architecture audit                        |
| 13  | Coherent full snapshots                 | PASS   | One locked observation; concurrency/snapshot equality tests                |
| 14  | Committed outputs/locals visible        | PASS   | Backend observation and live-value presentation tests                      |
| 15  | Same-scan read-after-write              | PASS   | Actual ST through existing runtime; supplied snapshot rendering            |
| 16  | Existing TON Q/ET state                 | PASS   | Four-scan headless source proof and committed FB observation               |
| 17  | Private epoch absent                    | PASS   | Exact TON DTO field tests reject activeSince                               |
| 18  | Runtime faults are domain data          | PASS   | Faulted snapshot with original code/category; transport separated          |
| 19  | Fault maps to ST                        | PASS   | Core provenance resolves line 4, scalar columns 11–24                      |
| 20  | Fault shows committed rollback          | PASS   | Non-default Result 50 retained after later failure                         |
| 21  | Failed scan keeps time/count            | PASS   | Headless rollback and frontend fault snapshot tests                        |
| 22  | Faulted disables Single Scan            | PASS   | READY/current/pending gating; fault UI tests                               |
| 23  | Explicit existing reset                 | PASS   | Runtime::reset; whole default snapshot equality                            |
| 24  | Edited source makes association stale   | PASS   | Revision invalidation disables scan/inputs and source markers              |
| 25  | Old responses discarded                 | PASS   | Late load/scan, wrong-generation, unmount and error tests                  |
| 26  | Exact native TON sequence               | PASS   | Final hosted native artifact proves the exact four-scan TON sequence       |
| 27  | Native division fault proof             | PASS   | Final hosted actual PLC-RUNTIME-003, expression marker, rollback and reset |
| 28  | Run/Pause/Stop disabled                 | PASS   | UI and native assertions retained throughout manual workflow               |
| 29  | Accepted regression semantics preserved | PASS   | Frozen core/ST byte comparison; original tests retained                    |
| 30  | Windows independent CI                  | PASS   | Run 38565604434 Windows terminal SUCCESS; all stage artifacts inspected    |
| 31  | Linux independent CI                    | PASS   | Run 38565604434 Linux terminal SUCCESS; actual debug/release execution     |
| 32  | Real native Windows smoke               | PASS   | Final actual native Tauri/WebView2 smoke PASS; no page errors              |
| 33  | No later-stage implementation           | PASS   | Source/dependency/scope audit                                              |

All 33 criteria are supported by the final independent execution and regression/scope audit. Historical failures/blocks above remain separate evidence.

## Scope audit

No continuous Run/Pause/Stop semantics, background scheduler, wall-clock pacing, speed multiplier, process simulation, networking/protocol, trace/timeline recording, grading/exercises, forcing/watch tables, LAD/FBD/STL, other standard blocks, user Functions/FBs, persistence, multi-task or hardware I/O entered M2B.

## Assumptions

Keep configuration fixed in Rust rather than accepting arbitrary browser cycle/budget choices. Use one headless application crate to enable portable service tests. Inputs show staged values and old-session values remain labeled stale until explicit reload/unload. Numeric drafts refresh from returned snapshots after successful mutation; they never become authoritative PLC values.

## Known limitations

Only the assigned manual Alpha session is supported. The owner-host final native launch failed and optimized build was application-control blocked; host policy was not changed. The compact accepted foundation layout has scrolling panes and tooltip-backed long names. Monaco retains the existing non-failing bundle-size warning. ts-rs ignores serde deny_unknown_fields when deriving tagged-enum TypeScript; Rust serde and explicit frontend validators independently enforce unknown-field rejection, covered by executed negative tests. No lint waiver or assertion change hides this boundary.

## Next dependency

Technical-lead source review of runtime-session ownership/serialization, DTOs/IPC and the UI association/input/fault workflow. Validation PASS does not replace that review. Stop before M2C/M3; no authorization is inferred.

## Gate

M2B GATE: PASS

All 33 implementation/validation criteria passed for the exact recorded code SHA. Source review is pending as required before the next milestone. M2C/M3 have not started.

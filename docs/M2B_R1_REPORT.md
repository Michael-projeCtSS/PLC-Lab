# M2B-R1 — Load intent ordering

Date: 2026-10-14. Scope: the required M2B source-review concurrency patch only. Starting status supplied by the technical lead: M2B validation PASS, source review PASS WITH REQUIRED PATCH, final gate HOLD. This report does not approve a later milestone or replace historical M2B validation evidence.

## Root cause

The accepted service prepared each candidate outside its mutex, then unconditionally swapped it after acquiring the lock. The lock prevented simultaneous mutation but did not identify request order. If B completed before A, B could be loaded and acknowledged before A later replaced it. React could discard A's response while the backend nevertheless contained A, causing subsequent targeted B operations to fail as stale.

## Completed

Implemented private backend Load intent ordering and stale commit protection, four backend tests, two frontend integration tests, the requested specification/design notes and one native-harness synchronization repair.

## Fix

After validating the IPC version, Load briefly locks to reserve a checked nonzero backend LoadIntentId and record it as latest. Compilation, validation, lowering and Runtime construction remain outside the mutex. Final commit reacquires the mutex and rejects any intent other than latest with APP-RUNTIME-STALE-LOAD, before allocating a committed session generation or changing the loaded session. A newer intent supersedes older requests even when its preparation fails; the prior valid session survives.

Intent exhaustion returns APP-RUNTIME-LOAD-INTENT-EXHAUSTED without wrapping, reusing identities or mutating the committed session. Existing session-generation exhaustion remains APP-RUNTIME-GENERATION. Frontend production code is unchanged; new tests prove obsolete stale-load errors are discarded and current-request errors fail closed until explicit reload. No automatic retry was added.

## Architecture

The accepted prepare-outside-lock, commit-under-lock service architecture remains intact. ADR-0013 records the authorized ordering correction; core semantics, UI authority, session ownership and IPC contract are unchanged.

## Identity model

| Identity         | Ownership and lifetime                                                                 | Allocation                                                                                    |
| ---------------- | -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| LoadIntentId     | Private backend request authority; never transported over IPC                          | Every version-valid Load reservation, including requests whose preparation subsequently fails |
| RuntimeSessionId | Existing committed runtime identity; required by targeted commands and present in DTOs | Only a successfully committed replacement; stale/failed loads do not consume one              |

Both counters are service-local checked monotonic u64 values. Zero means no intent has yet been reserved; issued identities start at one. Unload does not reset either counter. Request order means reservation under the backend mutex, independent of preparation completion and wall time. Preparation is a private associated helper with no service reference; tests exercise those same private production operations. No public test API, delay, new IPC field or contract-version change exists.

The application specification includes the requested future design note: before continuous/background execution, reconsider a separate IPC/recovery envelope for application-internal execution failures. For this patch ScanError::Internal retains category internalApplicationError in snapshots, distinct from student runtimeFault.

## Deterministic race tests

- **A requested → B requested → B commits → A rejected:** reserve intent 1 then 2; prepare/commit B first; finish preparing A afterward; reject A with APP-RUNTIME-STALE-LOAD. The complete backend B snapshot stays identical and its session remains 1. C uses intent 3 and session 2, proving a stale commit did not consume a session ID.
- **X loaded → A requested → B requested → B fails → A rejected → X preserved:** load and scan actual X to non-default memory/count/time; reserve/prepare A; public Load B reserves the newer intent and fails actual ST compilation. A cannot commit and the entire X observation remains identical. Public valid C succeeds with the next committed session ID.
- Checked intent exhaustion preserves the complete prior session and both counters across repeated rejected requests; no identity is reused.
- Wrong contract versions are rejected before reservation and cannot supersede a valid pending intent.

All four new backend tests use real ST compilation/lowering/runtime and explicit operation order. No sleep, source-size trick, scheduler luck or production test delay provides the proof. Existing concurrent scan/input tests remain unchanged.

## Files

- crates/desktop-session/src/lib.rs: private intent/preparation/commit factoring and four deterministic/boundary tests.
- packages/ui/src/useRuntimeSession.test.ts: two stale-load response integration regressions; production hook unchanged.
- docs/adr/0013-desktop-runtime-session.md: authorized load-order correction to the existing decision.
- docs/spec/DESKTOP_RUNTIME_SESSION.md: identity, failure and ordering policy; future internal-error envelope note.
- docs/spec/DIAGNOSTICS.md: the two stable application error codes.
- docs/spec/TEST_STRATEGY.md: deterministic load-order evidence.
- tests/e2e/runtime-smoke.mjs: CI-only synchronization of rendered fault marker/expression geometry; original assertions retained.
- docs/M2B_R1_REPORT.md: this report, committed after independent evidence review.

## Regression

Application patch SHA: **961d49d4245304b5f97ac69abe2ddfbdff852d54**. Validated code SHA: **6f5743213af797ba60fef7bb017383b9b72f472c**. Repository: **Michael-projeCtSS/PLC-Lab, PRIVATE**. Branch: codex/m2b-runtime-session. Existing workflow: [run 38573633606](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38573633606), attempt 1, push.

The first [run 38571420355](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38571420355), SHA 961d49d, remains **FAIL**: Linux passed; Windows canonical checks (240 Rust/111 frontend/20 architecture), focused debug (238) and release (178) passed, but native smoke failed with “Runtime fault expression and marker must be rendered.” The helper had observed a marker, then made an immediate one-time expression geometry read that returned null. Remaining native checks after that assertion were not executed and are not counted as passes. Failed-run job IDs: Windows 115877311550; Linux 115877312257.

One CI-only repair, **6f5743213af797ba60fef7bb017383b9b72f472c**, waits for both nonzero rendered geometries and captures them in one DOM observation, then applies every original position/width/line assertion. It uses the existing bounded Playwright wait, without a fixed sleep, skip, mock, enlarged coordinate tolerance or production change. Application/core/contract/React sources are byte-identical to 961d49d. The revised actual local native harness passed before publication.

## Tests

| Command / evidence                                                     | Result                                                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| node scripts/rust.mjs test --locked -p desktop-session -p app-contract | Local PASS: 28 session + 18 contract tests; no failures/ignored tests                                                                                                                                                                                                                     |
| npm run test:frontend                                                  | Local PASS: 111 tests                                                                                                                                                                                                                                                                     |
| npm run test:architecture                                              | Local PASS: 20 tests, zero skips                                                                                                                                                                                                                                                          |
| npm run validate                                                       | Local BLOCKED/nonzero 101: formatting, typecheck, ESLint/clippy and frontend/architecture passed; Windows application control blocked workspace desktop_session test binary before execution, OS error 4551. The canonical command is not reported as local PASS                          |
| npm run test:desktop                                                   | Local PASS: production frontend/native build and actual Tauri/WebView2/Rust IPC smoke, including manual runtime session behavior                                                                                                                                                          |
| npm run ci:core                                                        | Linux PASS: fmt/clippy, 238 focused debug + 178 release tests, 18 portable architecture tests; two unchanged Windows BAT platform guards                                                                                                                                                  |
| npm run ci:windows                                                     | Hosted PASS: full canonical check (240 workspace Rust tests, 111 frontend tests, 20 architecture tests, formatting/typecheck/lint/clippy and production build), 238 focused debug tests, 178 release tests and actual native Tauri/WebView2 smoke. Zero required tests skipped or ignored |

The two CI Rust commands select the same seven debug packages (plc-types, plc-ir, plc-runtime, simulation-core, st-frontend, app-contract, desktop-session) and five release packages (plc-runtime, simulation-core, st-frontend, app-contract, desktop-session). Both execute the four new load tests and all existing required tests, including unchanged fixed REAL bit assertions; compilation alone supplies no acceptance evidence.

Windows job **115884957650**: success, UTC 13:39:38–13:52:18. Linux job **115884957239**: success, UTC 13:39:38–13:41:09. Workflow conclusion: success. Hosted nodes/toolchain: Node v24.21.0, pinned Rust 1.99.0, Windows x64/MSVC and Linux x64/GNU. Downloaded logs/results/native screenshots are retained locally under ignored artifacts/m2b-r1-ci/38573633606; failed-run artifacts are retained under 38571420355. The focused artifact audit records exact revision, stage exits and matching passing test names. Final artifacts: windows-validation-1 ID **11899787367**, digest `sha256:d13116c4f0f2b222e5de8c42fc3b0c284c5e44f21042eb16a928394073ca7bc2`; linux-core-1 ID **11899415655**, digest `sha256:5eb957b6ee33ea86c83623a73f76c4523369b69c6c9fc872fb9c4b8d52ac9239`. Both scoped archives were downloaded and inspected; retention remains 14 days.

Native Windows smoke is **PASS**, with actual application startup, IPC v4/manual-single-scan, seven panels and no page errors. Real UI mutations are correlated with targeted Rust snapshots. TON scans 1–4 report times 100/200/300/400 ms, ET 0/100/200/300 ms and Q false/false/false/true. Actual division-by-zero fault is PLC-RUNTIME-003 at line 4, columns 11–24, Faulted with count/time zero and unchanged outputs; original expression/marker geometry assertions execute. Reset returns Ready and clears the actual fault/state. INT minimum, REAL 1.5 and TIME 18446744073709551615 microseconds pass real IPC. Source-edit invalidation, unload and disabled Run/Pause/Stop pass. Hosted screenshots were inspected; marker geometry is checked before the existing screenshot step scrolls the Variables pane. The same unmodified fixed REAL bit assertions pass on Windows/Linux in both profiles, and the audit compares all passing Rust test names across platforms.

Existing behavior remains covered: first session 1; monotonic successful replacement/unload; failed replacement preservation; stale input/scan/reset/unload/observation rejection; serialized scans; old input cannot mutate replacement; pure Compile; unchanged single scans, TON, source-backed faults/rollback/reset and exact maximum TIME IPC. No existing test/assertion was weakened. Existing ts-rs notices about deny_unknown_fields remain unchanged; actual serde/contract tests execute.

## Acceptance criteria

| Criterion                                                   | Result and evidence                                                                                                                                                  |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1-001 intent before preparation and stale commit rejection | PASS: private checked reservation + locked latest-intent comparison                                                                                                  |
| R1-002 failed newest load invalidates older work            | PASS: actual failed B compile leaves A stale; no fallback                                                                                                            |
| R1-003 failed load preserves committed session              | PASS: full non-default X snapshot equality and existing replacement regression                                                                                       |
| R1-004 exhaustion fails closed                              | PASS: repeated checked exhaustion preserves snapshot/counters; existing session-ID exhaustion retained                                                               |
| R1-005 expensive preparation outside mutex                  | PASS: prepare_load has no service reference; only reservation/commit lock                                                                                            |
| R1-006 inverted completion test                             | PASS: B survives rejected late A, executed in both CI profiles/platforms                                                                                             |
| R1-007 failed newest test and valid C recovery              | PASS: X preserved, A rejected, C gets next session ID                                                                                                                |
| R1-008 frontend protection retained                         | PASS: production source/request/expected-session checks unchanged; two new integration tests; no retry                                                               |
| R1-009 required regressions                                 | PASS: all existing required tests execute in current Windows/Linux debug/release lanes and real native Windows smoke; local canonical host block recorded separately |
| R1-010 internal error note only                             | PASS: future design note added; existing snapshot category/transport retained                                                                                        |
| Strict scope and no weakened tests                          | PASS: protected source/contract/workflow/harness audit; no future work                                                                                               |

## Scope audit

Relative to pre-patch fae49418dd4bc3b9fc11f6b1981de1dc475d0d49, AGENTS.md, plc-types, plc-ir, plc-runtime, simulation-core, languages/st, app-contract, packages/ipc, desktop/Tauri adapter, production React code, lockfiles, CI workflow/scripts and historical validation reports are unchanged. Only application Load ordering, added tests, corresponding documentation and the isolated native harness observation synchronization changed. Existing assertions remain unchanged; new tests supplement the existing suites.

The current working-tree/index publication audit scanned 188 project files with no detected tokens, private keys, credential URLs or executable publication paths. Origin remains credential-free HTTPS to the verified private repository; no remote/security configuration, permissions, collaborators or secrets changed. Push was normal, without force.

No ST/IR/Runtime/TON/SimulationClock/budget/value semantics changed. No continuous Run/Pause/Stop, scheduler, process, trace/timeline, network, grading, other language or additional PLC feature was implemented.

## Assumptions

Retain the existing private feature branch and push-triggered validation workflow authorized for M2B independent validation. Backend request order is mutex reservation order at the start of version-valid RuntimeSessionService::load, as requested. No frontend intent ID or worker cancellation scheme is necessary.

## Known limitations

The full owner-host canonical command remains blocked at one generated workspace test binary by Windows application control; its unexecuted tests are not counted as local passes. Focused local tests and the separately executed real native smoke passed; independent full Windows/Linux execution supplies the required acceptance evidence. No host security policy was changed. The temporary internalApplicationError snapshot envelope remains the explicitly accepted limitation until separately reviewed before background execution.

## Next dependency

Stop for technical-lead review of this required patch and final M2B approval. M2B FINAL GATE remains HOLD pending that review. Do not begin M2C or M3.

The report is recorded in a later documentation-only commit; the validated source identity remains 6f5743213af797ba60fef7bb017383b9b72f472c. Resolve the report commit with `git log -1 -- docs/M2B_R1_REPORT.md`. No later documentation-only revision is claimed as the executed source SHA.

## Gate

M2B-R1 PATCH: PASS

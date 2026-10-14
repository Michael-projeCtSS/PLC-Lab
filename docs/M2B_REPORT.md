# M2B implementation report

Date: 2026-10-14. Branch: codex/m2b-runtime-session. M1C and M2A validation/source review are accepted. Implementation and focused tests are present; final canonical and independent CI evidence is pending. This report will be finalized only after actual execution.

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

Real native UI/Monaco/load/input/scan execution passed locally. Initial state: scan/time/ET 0, Motor/Q false. With Start true, cycle 100 ms and PT 300 ms:

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

Local npm run check: PASS, including formatting, TypeScript, ESLint/workspace clippy, 20 architecture tests, 108 frontend tests (before the final transport-failure regression), 236 workspace Rust tests and the production frontend build. Application/contract coverage is 24 desktop-session + 18 app-contract (11 retained + 7 new) = 42 executed tests. An earlier interactive native working-tree run passed. The latest npm run validate failed at native launch with spawn UNKNOWN after canonical checks/build passed; the final local native launch is not reported as PASS. No host-security change or workaround was applied. Local release tests are BLOCKED before execution: Windows application control rejected the serde_json build script with OS error 4551. Independent Windows/Linux CI remains pending; compilation-only evidence will not count as execution.

Superseded harness attempts exposed ambiguous Run/Reset Runtime selection, reply-capture correlation and a leftover capture reference; corrected selectors/targeted observations preserve all runtime assertions. A source-pattern guard also needed whitespace tolerance after Rust formatting. No PLC assertion, semantics, required job or security setting was weakened.

## Regression audit

Frozen plc-types/plc-ir/plc-runtime/simulation-core and languages/st source/tests remain byte-identical to the accepted M2A state. Historical M1/M2A validation documents are preserved. Core-info capability/version expectations migrate explicitly to v4 manual-single-scan while pre-load/compile-only controls remain disabled. Original runtime/REAL/TON/rollback/clock/ST tests remain mandatory in debug/release on both platforms.

## Acceptance criteria

Final per-criterion results will be recorded after complete independent execution; independent Windows/Linux gates are currently NOT_RUN. No pending requirement is counted as PASS.

## Scope audit

No continuous Run/Pause/Stop semantics, background scheduler, wall-clock pacing, speed multiplier, process simulation, networking/protocol, trace/timeline recording, grading/exercises, forcing/watch tables, LAD/FBD/STL, other standard blocks, user Functions/FBs, persistence, multi-task or hardware I/O entered M2B.

## Assumptions

Keep configuration fixed in Rust rather than accepting arbitrary browser cycle/budget choices. Use one headless application crate to enable portable service tests. Inputs show staged values and old-session values remain labeled stale until explicit reload/unload. Numeric drafts refresh from returned snapshots after successful mutation; they never become authoritative PLC values.

## Known limitations

Only the assigned manual Alpha session is supported. Monaco retains the existing non-failing bundle-size warning. ts-rs ignores serde deny_unknown_fields when deriving tagged-enum TypeScript; Rust serde and explicit frontend validators independently enforce unknown-field rejection, covered by executed negative tests. No lint waiver or assertion change hides this boundary.

## Next dependency

Complete independent validation, then technical-lead runtime-session/application/UI source review. Stop before M2C/M3; no authorization is inferred.

## Gate

M2B GATE: FAIL

Independent validation remains pending. M2C/M3 have not started.

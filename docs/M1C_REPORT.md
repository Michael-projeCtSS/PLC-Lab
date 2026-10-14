# M1C completion report

Date: 2026-10-14. Scope: M1C-001 through M1C-022, headless standard Function Block foundation and TON only. Implementation and substantive tests are present. The acceptance gate cannot pass while host execution policy blocks required validation. M2 has not begun.

## Completed

Added typed standard definitions/signatures, declared instance ownership, single instance-to-definition resolution, checked parameter bindings, static validation, normalized owned TON instructions, private transactional timer state, immutable Q/ET observations, reset and exact budget integration. Added tests covering every requested matrix case and updated architectural specifications. Accepted M1B behavior is preserved.

## Files

Paths below are relative to C:/Users/mjgom/Documents/PLC_Student. Existing staged/untracked work was preserved; no commit, remote or branch change was made.

| Files added or changed                                                                                           | Purpose                                                                                                      |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| crates/plc-types/src/function_blocks.rs (new)                                                                    | Typed definitions, authoritative TON signature, catalog/instance declarations and declaration tests          |
| crates/plc-types/src/lib.rs, symbols.rs, diagnostics.rs                                                          | Module export, shared checked identifier helper, stable FB compiler codes and related identities             |
| crates/plc-ir/src/model.rs, validation.rs                                                                        | Program catalog, instance-only typed calls, binding/catalog validation and defensive duplicate unit test     |
| crates/plc-ir/tests/function_blocks.rs (new), validation.rs                                                      | Positive/negative call tests and migration from the provisional API                                          |
| crates/plc-ir/tests/support/mod.rs, support/ton.rs (new)                                                         | Small shared test-only direct-IR fixture; no runtime mocks                                                   |
| crates/plc-runtime/src/function_blocks.rs (new), lib.rs                                                          | Private TON state and immutable public observation type                                                      |
| crates/plc-runtime/src/plan.rs, executor.rs, runtime.rs                                                          | Owned normalized instructions, exact charges and atomic instance/memory publication/reset                    |
| crates/plc-runtime/src/ton_transaction_tests.rs (new)                                                            | Private epoch rollback and internal invariant tests                                                          |
| crates/plc-runtime/tests/ton.rs (new)                                                                            | Public runtime timing, associations, skip/repeat, reset, normalization and determinism tests                 |
| docs/adr/0011-function-block-instance-model.md (new), 0010-transactional-alpha-runtime.md, README.md             | Authorized provisional-call migration and accepted M1B status                                                |
| docs/spec/STANDARD_FUNCTION_BLOCKS.md, TON_SEMANTICS.md (new)                                                    | Model, signature, algorithm, exact timing, transaction/budget/observation policy                             |
| docs/spec/CANONICAL_PLC_IR.md, PLC_RUNTIME_SEMANTICS.md, DIAGNOSTICS.md, TEST_STRATEGY.md, PLC_FEATURE_MATRIX.md | Updated executable subset, binding errors, state rules, required test coverage and pending acceptance status |
| docs/architecture/ARCHITECTURE.md, docs/ROADMAP.md, docs/M1C_REPORT.md (new)                                     | Current boundaries, review dependency and evidence                                                           |

No Cargo manifest, third-party dependency, lockfile, React, Tauri command, IPC contract, shell/style, launcher or human checklist change was required.

## Architecture

ADR-0011 records the explicitly authorized replacement of the provisional M1A call shape. Semantic declarations stay in plc-types; canonical calls and validation stay in plc-ir; owned execution and authoritative state stay in plc-runtime. simulation-core supplies only SimulationClock and depends on plc-types. No reverse, React, Tauri, trace or language dependency was added. BTreeMap cloning remains the accepted correctness-first policy.

### Function Block model

```text
Definition (FunctionBlockId, semantic kind, ordered typed signature)
  -> declared Instance (FunctionBlockInstanceId, exactly one definition ID)
  -> Call (instance ID + typed bindings + statement SourceRef)
  -> private Runtime state (instance-ID map, independent of SymbolId memory)
```

Program owns the catalog. Calls resolve instance -> definition; they cannot supply a competing definition. Numeric IDs/names never select TON behavior. StandardFunctionBlock::Ton does. Forward declarations may be assembled but unresolved references fail validation. Names are case-insensitive ASCII metadata per POU. Non-entry instances cannot be called by the entry.

The old definition argument and separate CallInput/CallOutput vectors are removed. CallBinding has typed Input, Output and reserved InOut cases; duplicate parameters are rejected across all directions. No persisted IR/frontend schema exists to migrate. User-defined signatures are metadata only, with no custom body/executor/plugin.

### TON signature

| ParameterId | Name | Direction | PlcType | Association rule           |
| ----------- | ---- | --------- | ------- | -------------------------- |
| 1           | IN   | Input     | BOOL    | Required on every call     |
| 2           | PT   | Input     | TIME    | Required on every call     |
| 3           | Q    | Output    | BOOL    | Optional writable SymbolId |
| 4           | ET   | Output    | TIME    | Optional writable SymbolId |

Parameter IDs are local to the resolved definition. The standard factory owns the exact signature; it cannot be replaced with caller-supplied parameters. Omitted inputs are never retained/defaulted. Lowering normalizes IN then PT and Q then ET regardless of producer binding order. Entry Input targets are read-only.

### TON runtime algorithm

At each executed call, now is the fixed scan-start SimulationClock value. IN false clears the epoch and sets Q false/ET zero. IN true on an inactive instance sets active_since = now. Otherwise elapsed = checked(now - active_since). Then ET = min(elapsed, current PT) and Q = elapsed >= current PT. PT zero immediately asserts Q with ET zero.

Changing PT while true preserves the epoch: reducing PT can assert Q and increasing it can deassert Q. Skipped calls change neither state nor associations; a later call catches elapsed virtual time. Repeated calls to one instance execute sequentially at the same now and see earlier working state. An impossible backward clock is APP-CORE-001, not a student RuntimeFault.

### Timer timeline evidence

Actual optimized TON test output, with 100 ms cycles and PT 300 ms:

| Scan | Scan start | ET     | Q     | Returned time |
| ---- | ---------- | ------ | ----- | ------------- |
| 1    | 0 ms       | 0 ms   | false | 100 ms        |
| 2    | 100 ms     | 100 ms | false | 200 ms        |
| 3    | 200 ms     | 200 ms | false | 300 ms        |
| 4    | 300 ms     | 300 ms | true  | 400 ms        |
| 5    | 400 ms     | 300 ms | true  | 500 ms        |
| 6    | 500 ms     | 300 ms | true  | 600 ms        |

These are executed test results, not a frontend animation or fabricated trace. The debug TON executable is host-blocked, so this timeline is currently verified in optimized mode only.

### Transaction model

begin_scan retains the accepted last-attempted input image and clones writable PLC memory plus committed FB states into working copies. All calls and association writes touch only those copies. After execution, checked next clock/count and output-image preparation must succeed before publication. Then memory, outputs, instance state, count and clock publish together under one exclusive Runtime borrow. No fallible semantic operation follows publication.

Arithmetic, budget, internal, clock and count completion faults discard working copies. Private tests compare full instance maps, including active_since, and previously non-default committed memory/output/time/count. Last-attempted inputs intentionally remain visible. Faulted refuses scans until full explicit reset; reset restores every instance inactive/Q false/ET zero.

### Execution budget

Normal expression operations charge for IN, then PT. One operation is charged before the state update, one before a present Q write and one before a present ET write. Constant/read inputs therefore cost 3 with no associations, 4 with one and 5 with both. Charge is checked before the effect. Private tests force failure after mutation but before Q and ET writes, and after a completed TON call before a later statement; all committed state rolls back.

### Runtime snapshot

RuntimeSnapshot::function_blocks() returns an immutable map keyed by FunctionBlockInstanceId, containing FunctionBlockStateSnapshot::Ton { q: bool, et: PlcTime }. The snapshot owns its observation, survives later scans/reset independently and provides no mutable runtime handle. active_since is private and never represented by a fake PLC SymbolId or frontend IPC field.

### Diagnostics

| New stable code | Meaning                                  |
| --------------- | ---------------------------------------- |
| PLC-COMP-016    | Unknown FB instance                      |
| PLC-COMP-017    | Unknown FB definition                    |
| PLC-COMP-018    | Missing required input                   |
| PLC-COMP-019    | Unknown parameter in resolved definition |
| PLC-COMP-020    | Wrong binding direction                  |
| PLC-COMP-021    | Parameter/target type mismatch           |
| PLC-COMP-022    | Duplicate binding                        |
| PLC-COMP-023    | Unsupported executable FB kind           |
| PLC-COMP-024    | Invalid/foreign instance owner           |

Existing codes 001/003/014 exactly cover unknown/invisible/read-only output targets. Catalog duplicate identity/name/invalid name reuse 005/002/006. Compiler diagnostics retain severity, message, mapped optional call/declaration source and relevant typed instance/definition/symbol identity. Existing RuntimeFault codes remain unchanged. Internal timer invariants use APP-CORE-001 and call source.

## Tests

| Command/check                                                                                | Actual result                                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| node scripts/rust.mjs fmt --all                                                              | PASS                                                                                                                                                                                                                          |
| npm.cmd run format:check (inside canonical validation)                                       | PASS: Prettier and Rust fmt                                                                                                                                                                                                   |
| npm.cmd run lint                                                                             | PASS: ESLint and workspace/all-target clippy with warnings denied                                                                                                                                                             |
| npm.cmd run typecheck (canonical validation and desktop build)                               | PASS                                                                                                                                                                                                                          |
| node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core | BLOCKED: 34 IR and 38 runtime tests passed; debug TON process rejected before execution (4551; later attempt also error 5)                                                                                                    |
| node scripts/rust.mjs test --locked -p plc-types -p simulation-core                          | BLOCKED at plc-types launch, error 4551; no tests in that invocation executed                                                                                                                                                 |
| node scripts/rust.mjs test --locked -p simulation-core                                       | PASS: 3 clock tests and doc tests                                                                                                                                                                                             |
| node scripts/rust.mjs test --release --locked -p plc-runtime --test ton -- --nocapture       | PASS: all 14 TON behavioral tests, actual timeline printed                                                                                                                                                                    |
| node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core              | PARTIAL: all 52 runtime tests passed (10 unit + 28 M1B + 14 TON); optimized simulation-core launch blocked, error 4551                                                                                                        |
| npm.cmd run validate                                                                         | FAIL/incomplete: format/typecheck/lint passed, architecture 10/10 and frontend 18/18 passed; workspace Rust passed contract 5, conformance 2, IR 34 then runtime unit launch was blocked by host policy                       |
| npm.cmd run test:desktop                                                                     | BLOCKED at native launch: production Vite frontend build and Tauri/Rust desktop compilation passed; smoke harness spawn UNKNOWN. Code Integrity records node.exe loading the desktop executable as a signing-policy rejection |
| Dependency/IPC/scope audit and SHA256 comparisons                                            | PASS: contract, LabShell, styles, IPC client/generated contract and human checklist match pre-task hashes; core boundary guards pass                                                                                          |

Windows Code Integrity event 3077 reports policy ID {0283ac0f-fff1-49ae-ada1-8a933130cad6}: generated executables did not meet enterprise signing requirements or violated policy. No security policy, signing requirement or test assertions were weakened. Ordinary retries had mixed outcomes; none of the blocked tests is counted as passed. The native smoke never reached WebView2/IPC assertions in this batch.

### Required 36-case coverage

The complete requested semantic matrix has passing evidence from the executed IR tests and optimized runtime tests. This does not make the blocked debug/full validation passing. Test names below refer to function_blocks.rs (IR), ton.rs (public runtime) and ton_transaction_tests.rs (private runtime); parameterized cases assert each listed variant.

| Case                         | Evidence/test                                                                                                                   | Executed result                     |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| 1 IN false                   | false_input_is_inactive_and_zero_preset_is_immediate                                                                            | PASS release                        |
| 2 PT zero                    | false_input_is_inactive_and_zero_preset_is_immediate                                                                            | PASS release                        |
| 3 Exact timeline             | exact_100ms_cycle_300ms_preset_uses_scan_start_and_caps_elapsed                                                                 | PASS release                        |
| 4 ET cap                     | Same six-scan timeline test                                                                                                     | PASS release                        |
| 5 False after expiration     | false_after_expiration_resets_and_true_starts_a_new_epoch                                                                       | PASS release                        |
| 6 New epoch                  | Same false/true sequence test                                                                                                   | PASS release                        |
| 7 PT decrease                | preset_changes_recalculate_against_the_retained_epoch                                                                           | PASS release                        |
| 8 PT increase                | Same dynamic PT test                                                                                                            | PASS release                        |
| 9 Q BOOL association         | q_and_et_associations_are_visible_to_later_same_scan_instructions                                                               | PASS release                        |
| 10 ET TIME association       | Same output association test                                                                                                    | PASS release                        |
| 11 Same-scan reads           | Same association/copy test                                                                                                      | PASS release                        |
| 12 Omitted Q                 | omitted_associations_update_instance_outputs_without_touching_other_symbols                                                     | PASS release                        |
| 13 Omitted ET                | Same four-association-mode test                                                                                                 | PASS release                        |
| 14 Same instance twice       | exactly_two_calls_to_one_instance_are_allowed_and_later_reset_wins; repeated_calls_see_sequential_working_state_at_the_same_now | PASS release                        |
| 15 Unselected branch         | skipped_calls_preserve_state_and_associations_then_catch_elapsed_virtual_time                                                   | PASS release                        |
| 16 Recall after skip         | Same skipped/catch-up test                                                                                                      | PASS release                        |
| 17 Missing IN                | required_inputs_are_each_mandatory_and_not_retained                                                                             | PASS debug                          |
| 18 Missing PT                | Same per-input rejection test                                                                                                   | PASS debug                          |
| 19 Wrong IN type             | input_types_require_bool_and_time_without_coercion                                                                              | PASS debug                          |
| 20 Wrong PT type             | Same per-input type test                                                                                                        | PASS debug                          |
| 21 Wrong Q type              | output_types_require_bool_and_time_without_coercion                                                                             | PASS debug                          |
| 22 Wrong ET type             | Same per-output type test                                                                                                       | PASS debug                          |
| 23 Read-only targets         | both_outputs_reject_entry_input_targets                                                                                         | PASS debug                          |
| 24 Unknown instance          | unknown_instance_is_source_backed                                                                                               | PASS debug                          |
| 25 Unknown definition        | missing_definition_is_checked_on_declaration_and_call                                                                           | PASS debug                          |
| 26 Competing definition      | catalog_cannot_rebind_an_instance_to_a_competing_definition                                                                     | PASS debug                          |
| 27 Unknown parameter         | unknown_parameter_does_not_satisfy_a_required_input                                                                             | PASS debug                          |
| 28 Wrong direction           | wrong_directions_and_reserved_inout_are_rejected                                                                                | PASS debug                          |
| 29 Duplicate binding         | duplicate_bindings_are_rejected_across_all_directions; private model validator duplicate guard                                  | PASS debug                          |
| 30 Unsupported kind          | user_kind_is_unsupported_even_with_ton_signature_and_numeric_id_one; private lowering guard                                     | PASS debug/release                  |
| 31 Later arithmetic rollback | later_arithmetic_and_budget_faults_preserve_committed_timer_epoch_and_memory                                                    | PASS debug/release                  |
| 32 Budget rollback           | Same later-fault test; budget_failure_between_state_and_each_output_write_rolls_back_the_whole_scan                             | PASS debug/release                  |
| 33 Clock rollback            | clock_and_counter_completion_failures_rollback_timer_mutation                                                                   | PASS debug/release                  |
| 34 Full reset                | reset_restores_every_instance_and_all_m1b_state_defaults; private fault/reset assertions                                        | PASS release; private debug/release |
| 35 Independent instances     | two_instances_of_one_definition_have_independent_state                                                                          | PASS release                        |
| 36 Identical sequence        | identical_input_sequences_produce_identical_results_and_snapshots                                                               | PASS release                        |

Additional executed evidence: unknown/foreign targets and owner/source validation; signature normalization through budget fault source/target order; normal expression fault charging; source-backed impossible epoch reversal; rollback includes last-attempted input image policy. New plc-types unit tests exist and compile under clippy, but their executable is blocked, so those tests are not claimed passing.

## Acceptance criteria

| Requested criterion                               | Result | Evidence/limitation                                                                         |
| ------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------- |
| 1 Distinct definition/instance/call/binding/state | PASS   | Typed model and separate private runtime state                                              |
| 2 Single authoritative relationship               | PASS   | Immutable instance declaration; duplicate rebinding rejected                                |
| 3 Semantic TON kind                               | PASS   | Definition 77 executes TON; user definition 1 is rejected                                   |
| 4 Authoritative typed signature                   | PASS   | Immutable standard factory; binding type/direction tests                                    |
| 5 Required IN/PT                                  | PASS   | Each omission rejected; no retained-input mechanism                                         |
| 6 Optional typed Q/ET                             | PASS   | Four association combinations and type rejection                                            |
| 7 Invalid binding validation                      | PASS   | Executed IR matrix, constructor and defensive guards                                        |
| 8 Entry inputs protected                          | PASS   | Both association targets rejected                                                           |
| 9 Normalized owned plan                           | PASS   | Owned typed instruction; no runtime signature lookup                                        |
| 10 Deterministic input order                      | PASS   | Shuffled producer order retains PT fault source after IN charge                             |
| 11 Scan-start time                                | PASS   | Fixed now and private nonzero activation epoch assertions                                   |
| 12 Exact timeline                                 | PASS   | Actual optimized six-scan evidence above                                                    |
| 13 PT zero                                        | PASS   | Immediate Q/ET assertion                                                                    |
| 14 ET cap                                         | PASS   | Post-expiration scans and dynamic preset assertions                                         |
| 15 False resets                                   | PASS   | Expired timer clears outputs and restarts                                                   |
| 16 Dynamic PT retained epoch                      | PASS   | Decrease asserts; increase deasserts without epoch restart                                  |
| 17 Independent instances                          | PASS   | Two instances sharing definition 77 tested                                                  |
| 18 Repeated calls                                 | PASS   | Exact two-call and three-call sequences at the same now                                     |
| 19 Skipped calls                                  | PASS   | No mutation/association writes; later elapsed jump                                          |
| 20 Transactional FB state                         | PASS   | Private map clones and full-map rollback assertions                                         |
| 21 Runtime fault rollback                         | PASS   | Later division fault preserves private epoch and PLC state                                  |
| 22 Budget rollback                                | PASS   | Later fault and failure before each association                                             |
| 23 Completion rollback                            | PASS   | Clock and count boundaries discard mutations                                                |
| 24 Reset defaults                                 | PASS   | Epoch/Q/ET, memory, fault, time/count reset                                                 |
| 25 Unassociated outputs update                    | PASS   | All association combinations observed                                                       |
| 26 Same-scan visibility                           | PASS   | Subsequent copies see new BOOL/TIME values                                                  |
| 27 Separate instance identity/state               | PASS   | Instance map; no generated symbols, immutable snapshot                                      |
| 28 Accepted M1B semantics preserved               | PASS   | Runtime regression suite passes in release; memory/operator/clock implementations unchanged |
| 29 All M0/M1A/M1B regression gates                | FAIL   | Required debug/full/native checks blocked by host execution policy                          |
| 30 No frontend/UI execution                       | PASS   | Architecture guards, scope search and unchanged hashes                                      |

## Assumptions

Use one Program-owned catalog, typed nonzero identities and per-POU case-insensitive ASCII instance metadata, matching existing Alpha declaration conventions. Standard factories retain exact signatures. All executable entry TON declarations receive defaults, even if never called; non-entry declarations remain metadata. No persisted schema or public API compatibility promise exists for the explicitly provisional M1A FB call.

## Known limitations

The host signing policy blocks some newly generated executables, leaving focused debug, all-workspace, optimized clock and native smoke gates incomplete. Passing optimized runtime/IR tests cannot substitute for those gates. Cross-architecture REAL validation remains the accepted M1B limitation. The feature matrix therefore does not mark TON accepted/complete yet.

## Scope audit

No ST lexer/parser/AST, Monaco integration, LAD/FBD/STL frontend/editor, TOF/TP/counter/edge block, user FB or Function execution, RETAIN, physical address, process/device/tank/pump/valve model, network/protocol/Modbus/OPC UA, trace recording, timeline, grading, forcing, UI execution, background loop, OS timer, thread scheduler or multiple task was implemented. Core has no React/Tauri dependence. packages/ipc remains a read-only typed information contract. LabShell.tsx remains accepted presentation code with Run/Pause/Stop/Single Scan disabled. The engineering contract and human checklist remain byte-identical.

## Next dependency

Resolve the host execution-policy block through the user's normal trusted build/development environment, rerun all required gates, then technical-lead review of ADR-0011, model/validation/lowering, TON state/transaction/budget and evidence. Stop before any first ST frontend authorization; do not begin M2.

## Gate

M1C GATE: FAIL

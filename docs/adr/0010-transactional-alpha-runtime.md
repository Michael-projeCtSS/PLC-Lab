# ADR-0010: Owned execution plans and transactional Alpha scans

- Status: Accepted by technical-lead M1B code review on 2026-10-14; M1C extension recorded in ADR-0011
- Date: 2026-10-14
- Scope: M1B-001 through M1B-016

## Context and authorization

M1A review accepted the type, identity, source, and structured IR foundations. The M1B assignment explicitly authorizes headless execution, transactional scans, clock behavior, and a new dependency direction. It keeps FB calls provisional and prohibits timers, languages, process/network simulation, traces, grading, and UI execution.

The foundation architecture had a reserved `simulation-core -> plc-runtime` dependency for future orchestration. M1B explicitly prohibits that reverse dependency and permits the runtime to use the clock from simulation-core. This record supersedes that reserved dependency direction, not the canonical dataflow or process/renderer separation.

ADR-0006 originally required trace integration from the first runtime. The M1B assignment explicitly excludes trace recording. Preserve SourceRef, resolvable source metadata, and deterministic snapshots/faults now; event recording remains a separate reviewed dependency before educational execution is exposed. Do not claim trace infrastructure is implemented.

## Decision

Keep Canonical PLC IR unchanged except the assigned entry-input write validation rule. Lower only `ValidatedProgram` into a separate owned, private-field ExecutionPlan in plc-runtime. Use an owned typed execution tree for the current bounded expression/assignment/IF subset; no bytecode, storage addresses, or borrowed IR lifetime leaks into Runtime.

Compile an explicit Alpha memory layout from the selected entry Program. Its Input/Output roles map to the external boundary; globals and entry locals are internal writable state. Non-entry POU declarations are excluded from this layout. Reject entry InOut rather than infer parameter semantics. Category is an interface/declaration tag, not universal physical I/O.

Use one synchronous `Runtime::single_scan` orchestrator in plc-runtime. It owns staged inputs, the committed state, a clock, configuration, and scan count. Rust exclusive borrowing and an owned input image prevent mutation during execution. Working state is discarded on faults; no output auto-reset policy is invented.

simulation-core owns only deterministic checked SimulationClock behavior in this batch and depends only on plc-types. plc-runtime imports plc-types, plc-ir, and simulation-core. Remove unused trace dependencies from both crates. Future application orchestration can wire independent runtime/process/network services without reversing this dependency; that orchestration is not implemented here.

Runtime arithmetic is checked i16 and explicit finite binary32 operations. Evaluation is left-to-right, AND/OR eager, IF branch selection exclusive. No constant folding, fast-math, reassociation, or fused-operation transformation is introduced. Every arithmetic REAL result is reconstructed through PlcReal. TIME supports comparisons only.

## Alternatives

| Alternative                                                | Decision                                                                                     |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Runtime owns borrowed ValidatedProgram                     | Rejected: couples runtime lifetime and state to a frontend candidate.                        |
| Runtime clones canonical Program and evaluates it directly | Rejected: fails to establish a separate executable subset/layout boundary.                   |
| Lower to bytecode now                                      | Deferred: a typed owned tree is enough for the approved subset and preserves source anchors. |
| Incremental writes directly to committed memory            | Rejected: cannot meet transactional rollback.                                                |
| Treat every Output/Input as physical I/O                   | Rejected: confuses POU interfaces with the entry boundary.                                   |
| Clock in runtime or trace-core                             | Rejected: simulation-core already owns virtual time; trace does not drive execution.         |
| Generic multi-task scheduler / real-time loop              | Rejected: unapproved scope and unnecessary for deterministic single scans.                   |

## Consequences and migration

No project/IR persistence or frontend contract changes are needed. Plan internals are not serialized or exposed to React. M1A read-only input assignment candidates now receive PLC-COMP-014; unsupported executable declarations receive PLC-COMP-015 during lowering. Reserved FB calls remain rejected and provisional.

Cargo.lock must record the authorized path dependency changes; no third-party dependency is added. Architecture guards enforce the new core direction and unchanged frontend/IPC boundary. Full M0/M1A validation and substantive direct-IR execution tests are required.

See [PLC_RUNTIME_SEMANTICS.md](../spec/PLC_RUNTIME_SEMANTICS.md) for exact memory, arithmetic, budget, fault, reset, and completion rules. The later authorized [ADR-0011](0011-function-block-instance-model.md) replaces the provisional FB call and extends transactional state for TON; accepted M1B behavior is preserved.

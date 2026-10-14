# Alpha deterministic PLC runtime semantics

Status: M1B accepted by technical-lead review. M1C extends state and execution only for standard TON under [ADR-0011](../adr/0011-function-block-instance-model.md); review is required before M2. This specification preserves [ADR-0010](../adr/0010-transactional-alpha-runtime.md). It defines PLC Lab's generic educational Alpha policy, not a vendor/full IEC profile.

## Boundary and ownership

```text
Canonical semantic Program
  -> validate -> borrowed immutable ValidatedProgram
  -> lower -> owned ExecutionPlan
  -> Runtime -> single_scan -> committed snapshot or fault
```

ExecutionPlan can be created only by checked lowering from ValidatedProgram. It owns an executable tree, SourceMap, and compiled memory layout. Its fields/instructions are private to plc-runtime. Runtime takes the plan by value, so the candidate and validation borrow can be dropped before any scan. There is no IR deserializer, parser, native-code generation, JavaScript evaluator, or executable IPC.

Lowering retains only source documents/elements referenced by executable nodes, not unrelated candidate metadata. Layout size, execution nodes, and retained anchors are therefore bounded by M1A's validated declaration/node limits; execution nesting stays within its depth limit. Missing/inconsistent validated metadata returns an internal lowering error. Candidate construction/import allocation remains a separate future boundary.

Canonical IR represents common semantics; the plan represents the current executable subset and entry memory roles. They remain distinct types. The plan retains SymbolId and SourceRef, but no source-language syntax, renderer objects, byte addresses, or string FB signatures. M1C adds a normalized typed TON instruction with instance ID, IN/PT expressions, optional Q/ET targets and source; the entry instance layout selects explicit semantic standard kinds. The executable statement subset is assignment/IF/TON; expressions are constants, reads, accepted unary/binary operations/comparisons. No constant folding occurs.

## Defaults and memory layout

`PlcType::default_value()` is authoritative: BOOL false, INT 0, REAL canonical +0.0, TIME zero microseconds. All initial staged inputs, input-image values, internal values, and outputs use it.

Memory is keyed by SymbolId in BTreeMaps, independent of names. Lowering compiles explicit `MemoryRole` values:

| Declaration ownership/category | Alpha role                                                                    |
| ------------------------------ | ----------------------------------------------------------------------------- |
| Entry Program Input            | ExternalInput: staged externally, read-only scan image                        |
| Entry Program Output           | ExternalOutput: writable working state and committed external outputs         |
| Entry Program Local            | Internal: writable persistent state                                           |
| Global Local                   | Internal: writable persistent global state                                    |
| Entry Program InOut            | Unsupported executable declaration, PLC-COMP-015                              |
| Non-entry POU symbol           | Excluded from the execution layout; never implicitly an external input/output |

Input/Output categories continue to describe POU interfaces. This mapping applies only to the selected Alpha entry Program. User-defined functions/FB execution remains unimplemented. Standard TON has its own typed signature and private instance state, distinct from symbol memory. Non-entry declarations can be present as metadata; the validator forbids accessing their locals/instances from the entry. Unsupported entry declarations fail lowering even if unused.

The memory implementation explicitly separates staged external inputs, last attempted scan input image, committed writable values, temporary working values, and committed external outputs. Last attempted input image is observational data, never writable state.

`set_input(SymbolId, PlcValue)` checks membership, compiled role, and exact type before changing staging. Unknown/non-input/wrong-type requests return API errors without mutation or a runtime fault. Staging is allowed between scans, including while Faulted; reset clears staging. No write/force API exists for committed internals or outputs. There is no physical I/O or Siemens addressing.

## One cyclic scan

Runtime owns exactly one cyclic task. There is no thread, timer, real-time pacing, background loop, priority, interrupt, or multi-task scheduler. Each explicit `single_scan()` call in Ready performs:

1. Clone staged external inputs into an owned read-only input image; remember it as the last attempted input image.
2. Clone last committed writable values and private FB instance state into temporary working state; capture fixed scan-start time.
3. Execute the plan once against that input image/working state and time under a fresh operation budget.
4. Prepare checked next time/count and the committed output image before publishing any writable changes.
5. Atomically publish successful working values, outputs and FB instance state, increment completed-scan count, and assign the prepared advanced clock.
6. Return the completed snapshot and number of charged operations.

Commit precedes the visible clock advancement; no observer can access an intermediate state because the method exclusively borrows Runtime. Clock/count overflow is checked before publication, so a completion failure also rolls back all writes. No fallible semantic operation follows publication.

Snapshots are owned immutable data containing staged inputs, last scan input image, committed writable values/outputs, virtual time, completed count, state, optional fault diagnostic, and an instance-ID map of FunctionBlockStateSnapshot::Ton { q, et }. RuntimeSnapshot::function_blocks returns only a shared reference to that owned map. Private active_since is not exposed or stored as a PLC symbol. Snapshots are not trace recording and have no transport/UI schema.

### Input and write visibility

The input image cannot change during a scan. External updates require the same exclusive runtime borrow as single_scan; no callbacks or shared mutable input handles exist. Staging a new input between scans does not retroactively change the previous image or committed outputs.

Writes to entry inputs are rejected in semantic validation with PLC-COMP-014. Entry Output/Local and supported globals can be written. Within a scan, a later read of writable memory sees earlier writes immediately. For `A := TRUE; B := A;`, B reads true. Internal and output reads in the next successful scan see their last committed values. Categories impose no future Function/FB parameter policy.

### Transactional failure policy

Any RuntimeFault aborts the scan, discards all working writes, preserves every previously committed internal/output value, leaves time and completed count unchanged, records the fault, and enters Faulted. Outputs are not forced false. The input image remains read-only; its observation from the failed attempt may be inspected. Staging is external state, not an executed write.

Impossible plan/memory/type invariants return internal errors with InternalApplicationError diagnostics, also faulting the runtime without publishing writes. They are not student RuntimeFaults. All ordinary invalid program errors belong to validation/lowering.

## Evaluation and arithmetic

Expressions evaluate their operands left-to-right. AND/OR evaluate both operands eagerly, including any arithmetic faults within the right operand. Expressions currently contain no side-effecting calls. Later side-effecting expressions cannot silently change this policy. IF evaluates its condition once and executes only its selected body. Statements execute in stored order.

| Type/operation                | Execution policy                                                                                       |
| ----------------------------- | ------------------------------------------------------------------------------------------------------ |
| BOOL NOT, AND, OR             | Boolean operations on BOOL only                                                                        |
| BOOL equality/inequality      | Exact boolean identity                                                                                 |
| INT +, -, *, unary negate     | Checked i16; overflow faults, never wraps or saturates                                                 |
| INT divide                    | Zero divisor faults; checked division, MIN/-1 faults; other quotients truncate toward zero             |
| INT comparisons               | Signed ordering/equality                                                                               |
| REAL +, -, *, /, unary negate | Explicit binary32 operation sequence, each result passed through PlcReal finite/canonical construction |
| REAL comparisons              | Exact finite numeric equality/order, no epsilon tolerance                                              |
| TIME comparisons              | Exact nonnegative u64 duration equality/order                                                          |

REAL arithmetic uses normal Rust f32 operations without fast-math or algebraic reassociation. Each basic operation rounds to binary32 under the supported target's normal round-to-nearest, ties-to-even environment; there are no fused multiply-add operations. Finite subnormals are preserved, underflow can produce canonical zero, and infinity/NaN results fault before entering working or committed memory. REAL division by zero (including 0/0) is reported as non-finite REAL result, not the INT zero-divisor code.

Bit-level cross-platform determinism is a required target property. Tests contain fixed binary32 bit-pattern expectations and must run unchanged as supported target CI expands. Current Windows acceptance does not prove untested architectures or modified host floating-point environments; no host control-register changes are made by this runtime.

Mixed types and TIME arithmetic remain unsupported. A well-typed constant division by zero is not folded away: its selected runtime evaluation faults. An unselected IF branch can contain well-typed faulting arithmetic and must not execute it.

## Per-scan execution budget

RuntimeConfig contains a positive u64 operation budget and positive microsecond cycle period. Zero values are rejected. No hidden default period/budget is selected by Runtime.

Charge one operation immediately before each constant, read, unary result, binary result, assignment write, or IF branch selection. Operand evaluation happens before charging its parent result; assignment value before charging its write; condition before charging IF selection. Selected body operations count normally; unselected bodies consume nothing. Snapshot/copy/commit, layout checks, time/count bookkeeping, and API input staging are not PLC execution operations.

TON evaluates IN then PT with normal expression charges, charges one operation before instance update, then one before each present Q and ET association write (in that order). Constant/read inputs cost 3 without associations, 4 with one, 5 with both. See [TON_SEMANTICS.md](TON_SEMANTICS.md) for scan-start, dynamic PT, skipped/repeated call and rollback policy. No generic call dispatcher or other standard block executes.

Exactly N charged operations may complete with budget N. Attempt N+1 faults before that operation's effect. The counter resets each scan. Failure after earlier PLC writes or TON mutations still rolls them all back, including failure between state update and output association. Empty programs complete with zero charged operations but require a positive configured budget. Validation's 64-depth/10,000-node limits bound representation; they do not replace this independent execution budget.

## Fault codes

All runtime faults use RuntimeFault category and Error severity. Operation faults resolve the retained source anchor and identify the assignment target where meaningful. Completion overflow has no invented source location. Messages do not expose stack traces or host details.

| Code            | Meaning                                                                 |
| --------------- | ----------------------------------------------------------------------- |
| PLC-RUNTIME-001 | Execution budget exceeded                                               |
| PLC-RUNTIME-002 | INT arithmetic overflow, including negate MIN and MIN/-1                |
| PLC-RUNTIME-003 | INT division by zero                                                    |
| PLC-RUNTIME-004 | Arithmetic REAL result is non-finite                                    |
| PLC-RUNTIME-005 | Virtual clock overflow during scan completion                           |
| PLC-RUNTIME-006 | Completed-scan counter overflow                                         |
| APP-CORE-001    | Internal execution/lowering invariant failure; InternalApplicationError |

Lowering distinguishes unsupported future constructs/declarations (compiler diagnostics) from impossible validated invariants (internal errors). No student error uses panic. Diagnostic construction uses only tested fixed code/message invariants.

## Clock, numbering, and reset

SimulationClock lives in simulation-core and uses PlcTime. `new()` starts at zero. Explicit `advance(delta)` checks u64 addition and leaves the clock unchanged on overflow; a zero delta is a valid primitive no-op, while RuntimeConfig rejects a zero cycle period. No OS time is consulted. An explicit `at(time)` constructor supports deterministic clock setup/tests without a runtime clock-mutation API.

Completed-scan count initially equals 0; the first successful result is scan 1 at one period. Failed attempts do not increment it. A successful scan advances exactly once by the configured period. Runtime time and count advance only with successful state publication.

State is Ready or Faulted. A Faulted runtime refuses single_scan until explicit `reset()`. Reset preserves the owned plan/configuration, restores all memory images/staging to type defaults and every TON to inactive/Q false/ET zero, clears the fault, sets count/time to zero, and returns Ready. There is no silent recovery, RETAIN policy, reset that retains outputs, RUN/PAUSE/STOP mode, or live configuration replacement.

## Tests and limits

Tests must construct IR directly, validate, lower, drop the candidate if desired, create a runtime, stage inputs, call single_scan, and inspect committed data/faults. Cover the four approved golden programs, repeated identical runs, stable inputs, intra-scan visibility, persistence/commit/rollback, unchanged outputs/time/count on fault, input permission validation, all INT faults, finite/canonical/subnormal REAL and bit patterns, TIME comparisons, selected IF only, eager booleans, exact budget boundaries, clock/count overflow, zero configuration, input API validation, foreign declaration roles, and deterministic reset. No test uses sleeps for semantic behavior.

M1C adds the full required TON matrix, including private epoch rollback after arithmetic/budget/clock/count failures, exact 100/300 ms timing, dynamic presets, repeated/skipped calls, independent instances, omissions, same-scan visibility, immutable observation, signature normalization, compiler errors and reset. Debug and optimized runtime tests retain the existing REAL bit-pattern evidence.

This is a headless execution foundation, not a completed educational experience. No other standard block, user FB/function execution, language frontend, process/network model, trace recorder, grading, physical I/O, background loop, or UI execution is included. Source anchors and snapshots prepare future evidence but cannot substitute for approved trace recording. Existing shell execution buttons remain disabled.

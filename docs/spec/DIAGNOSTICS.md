# Diagnostics model

## M2B runtime session diagnostics

Runtime faults arrive as coherent snapshot data with their original PLC-RUNTIME code, severity, category, scalar span and optional resolved related variable. APP-CORE internal diagnostics retain internalApplicationError and are never presented as student runtime faults. Compiler/load diagnostics and runtime faults have separate Monaco marker owners. Editing prevents old runtime spans from marking new source. Reset clears the actual backend fault.

Application errors: APP-RUNTIME-STALE-SESSION (replaced/unloaded target), APP-RUNTIME-INPUT (identity/permission/type), APP-RUNTIME-LOAD (preparation/configuration), APP-RUNTIME-GENERATION (exhausted generation), APP-RUNTIME-LOCK (poisoned mutation boundary), APP-RUNTIME-INTERNAL (inconsistent metadata), APP-RUNTIME-WORKER (failed blocking request). These remain transport/application errors, separate from compiler and student fault results. Existing IPC codes retain their meaning; v4 rejects unsupported versions.

M2B-R1 adds APP-RUNTIME-STALE-LOAD for an obsolete load candidate superseded by a newer backend intent, and APP-RUNTIME-LOAD-INTENT-EXHAUSTED for checked intent allocation exhaustion. Neither mutates the committed runtime nor returns another load's snapshot. These use the existing IpcError envelope; contract version, PLC values and student diagnostics are unchanged.

## M2A Structured Text diagnostics

Rust ST errors use the common CompilerDiagnostic/Error record and one-based Unicode scalar spans. The compilation DTO preserves code/severity/message and optional span; packages/ui/src/source-spans.ts converts to Monaco UTF-16 without changing the core source model. Shared PLC-COMP diagnostics from final IR validation retain their meanings. APP-IPC-005 reports a failed compilation worker separately from student errors.

| Code       | Meaning                                                                           |
| ---------- | --------------------------------------------------------------------------------- |
| PLC-ST-001 | Invalid character or unsupported nested/unterminated comment                      |
| PLC-ST-002 | Expected grammar construct, identifier or separator                               |
| PLC-ST-003 | Malformed, out-of-range, non-finite or invalid TIME literal                       |
| PLC-ST-004 | Unsupported Alpha construct/type or multiple programs                             |
| PLC-ST-005 | Duplicate variable/FB declaration or name collision                               |
| PLC-ST-006 | Unknown variable identifier                                                       |
| PLC-ST-007 | Incompatible assignment/operator or non-BOOL IF condition                         |
| PLC-ST-008 | Invalid assignment target, including read-only VAR_INPUT                          |
| PLC-ST-009 | Invalid TON declaration, instance, parameter, direction, required input or target |
| PLC-ST-010 | Compiler configuration/source/token/nesting/diagnostic limit                      |

For example: "Expected ';' after assignment." Lexical/syntax errors and semantic faults are returned as data. Diagnostics never start PLC execution. See [STRUCTURED_TEXT_ALPHA.md](STRUCTURED_TEXT_ALPHA.md).

Diagnostics must explain problems in student-facing language and carry stable machine-readable data. Accepted M1A/M1B data includes checked codes/messages, severities, categories, source anchors, related identities, entry-write/lowering errors and mapped runtime faults. M1C adds standard-FB catalog/binding errors and typed definition/instance related identities. There is no parser, trace recorder, or diagnostic UI stream; application IPC DTOs remain unchanged.

## Categories

| Category                 | Producer and meaning                                                    | Planned code prefix |
| ------------------------ | ----------------------------------------------------------------------- | ------------------- |
| compilerDiagnostic       | Language frontend: syntax, names, types, unsupported constructs         | PLC-COMP            |
| runtimeFault             | PLC runtime: budget exhaustion, arithmetic/memory/call-depth failure    | PLC-RUNTIME         |
| simulationFault          | Process model/orchestrator: invalid configuration, model update failure | SIM                 |
| communicationFault       | Virtual network/adapter: timeout, disconnect, malformed message         | NET                 |
| internalApplicationError | Desktop/transport: unexpected invariant or IPC incompatibility          | APP                 |

Codes are stable identifiers, not text fragments or exception names. PLC-COMP-001 through 013 remain unchanged; M1B adds 014/015 and PLC-RUNTIME-001 through 006. APP-CORE-001 identifies impossible validated execution/lowering invariants as InternalApplicationError. APP-IPC-001 through 004 and the renderer's APP-INTERNAL-001 remain unchanged. Simulation/communication faults remain vocabulary only.

| Code         | Meaning                                                        |
| ------------ | -------------------------------------------------------------- |
| PLC-COMP-001 | Unknown symbol/name                                            |
| PLC-COMP-002 | Duplicate declaration name                                     |
| PLC-COMP-003 | Invalid scope/ownership/visibility                             |
| PLC-COMP-004 | Incompatible redeclaration                                     |
| PLC-COMP-005 | Duplicate semantic identity                                    |
| PLC-COMP-006 | Invalid Alpha identifier                                       |
| PLC-COMP-007 | Unsupported assignment types                                   |
| PLC-COMP-008 | Unsupported operand types or incorrect expression result claim |
| PLC-COMP-009 | Non-BOOL condition                                             |
| PLC-COMP-010 | Unregistered source reference                                  |
| PLC-COMP-011 | Reserved function-block call not supported by M1A              |
| PLC-COMP-012 | Semantic validation size/nesting limit exceeded                |
| PLC-COMP-013 | Missing entry or entry not a Program                           |
| PLC-COMP-014 | Write to read-only Alpha entry-program Input                   |
| PLC-COMP-015 | Unsupported executable Alpha declaration (entry InOut)         |
| PLC-COMP-016 | Unknown function-block instance                                |
| PLC-COMP-017 | Instance references an unknown function-block definition       |
| PLC-COMP-018 | Required function-block input missing                          |
| PLC-COMP-019 | Parameter does not belong to the resolved definition           |
| PLC-COMP-020 | Function-block binding has wrong direction                     |
| PLC-COMP-021 | Function-block parameter/association type mismatch             |
| PLC-COMP-022 | Duplicate function-block parameter binding                     |
| PLC-COMP-023 | Unsupported executable function-block kind                     |
| PLC-COMP-024 | Invalid/foreign function-block instance owner                  |

M1C keeps all previous codes stable; PLC-COMP-011 retains its historical reserved-call meaning and is no longer emitted for TON. Constructor duplicate errors expose PLC-COMP-022; compiler validation also guards the same invariant and adds the call's source. Catalog/instance relationships are checked before execution. Invalid association targets reuse PLC-COMP-001 (unknown symbol), 003 (visibility) and 014 (read-only entry Input), because these meanings apply exactly. Catalog duplicate identities/names reuse 005/002; malformed names reuse 006.

FB compiler errors carry mapped call or optional declaration locations and FunctionBlockInstance, FunctionBlockDefinition or Symbol related identity where meaningful. Numeric definition/parameter IDs are identities, never runtime behavior selectors. TON adds no normal RuntimeFault code. An impossible activation timestamp later than scan-start time is APP-CORE-001/InternalApplicationError with the call source; rollback includes private epoch and Q/ET.

| Runtime code    | Meaning                                                         |
| --------------- | --------------------------------------------------------------- |
| PLC-RUNTIME-001 | Execution operation budget exhausted                            |
| PLC-RUNTIME-002 | Checked INT overflow, including negation MIN and MIN/-1         |
| PLC-RUNTIME-003 | INT division by zero                                            |
| PLC-RUNTIME-004 | Non-finite REAL arithmetic result, including REAL zero division |
| PLC-RUNTIME-005 | Virtual clock overflow at completion                            |
| PLC-RUNTIME-006 | Completed-scan counter overflow                                 |

All runtime faults have Error severity. Operation faults resolve ExecutionPlan's retained source and relate the assignment target when meaningful. Completion overflow has no invented source. Faults abort the transaction and enter Faulted; outputs, internal state, time, and completed count remain unchanged. A faulted runtime refuses another scan until explicit reset. Internal errors follow the same publication safety while retaining their separate category. See [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md).

## Shared record

| Field    | Type                                                                                | Requirement                                                                                |
| -------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| code     | string                                                                              | Stable nonempty machine identifier                                                         |
| severity | info, warning, error                                                                | error represents failed compilation/execution/operation; info/warning do not imply success |
| message  | string                                                                              | Plain English explanation; future localization can resolve by code                         |
| category | one of the five categories above                                                    | Identifies responsible domain                                                              |
| source   | SourceLocation or null                                                              | Optional location in the original language document                                        |
| related  | Symbol/Pou/Component/FunctionBlockDefinition/FunctionBlockInstance identity or null | Optional typed identity for the affected declaration/component                             |

SourceLocation now contains a typed document/element reference, ST/LAD/FBD/STL language, and optional TextSpan. IDs are nonzero. Text positions are one-based line/Unicode scalar column; spans are start-inclusive/end-exclusive, allow points, and reject reversed ranges. A graphical source can have no text span. See [SOURCE_MAPPING.md](SOURCE_MAPPING.md). Mapping to Monaco's UTF-16 columns belongs in a future adapter; no byte-offset or filesystem-path assumption exists.

Validated constructors enforce source invariants, code syntax (uppercase ASCII letters/digits/internal hyphens), and nonblank messages. Fields are private; getters and explicit `with_source`/`with_related` construction preserve immutable access. Locations and diagnostics are outbound Serialize types only; untrusted deserialization remains absent. SourceMap registers and resolves anchors with checked document/language/element consistency. No diagnostic IPC stream, UI notification system, runtime collector, or localization engine exists.

Program errors are CompilerDiagnostic/Error records. InternalApplicationError identifies defects or transport incompatibility, never ordinary student arithmetic faults. Production `expect` sites cover only fixed diagnostic-code/message invariants or private synchronized indexes; they are not student-input validation paths. Runtime tests deliberately corrupt private test state to prove impossible result types return APP-CORE-001 rather than a student RuntimeFault. No panic handler or host stack trace is added.

## Presentation and behavior

Show severity, code, and message together; provide source navigation only for a valid mapped location. Runtime faults must stop the affected simulated task according to the later documented fault policy and appear in authoritative trace evidence. Communication faults must identify the virtual operation; they must not be converted into invented successful replies. Do not expose raw stack traces, host paths, or sensitive payloads as ordinary student messages.

The foundation Diagnostics panel shows application IPC state only. IpcError has code/message because it is a transport rejection, not a full compiler/runtime diagnostic. The typed client maps it into an explicit unavailable/error UI state. A future diagnostic event DTO must preserve the shared categories and locations and include checked generation/drift validation.

## Validation

Rust tests cover checked identities/coordinates/spans/registries, all source-language tags, stable codes/category/severity wire names, nonblank messages, and diagnostics with text/graphical/no source and related identities. IR tests check mapped compiler errors and deterministic diagnostic ordering. Existing frontend/native tests still cover unavailable IPC and rejection visibility. Future editor/trace tests must cover actual Unicode encoding conversion and language lowering; M1A implements neither.

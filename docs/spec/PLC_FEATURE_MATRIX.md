# PLC feature matrix

## M2A implemented source frontend

M1C independent validation is accepted. M2A adds real Alpha ST source lexing, a distinct AST, semantic compilation and mandatory existing IR validation; headless tests lower and execute that source through the frozen runtime, including TON. Desktop support is Monaco editing and pure typed compile IPC v3 with diagnostics/declaration metadata. Run/Pause/Stop/Single Scan and runtime values remain unavailable. See [STRUCTURED_TEXT_ALPHA.md](STRUCTURED_TEXT_ALPHA.md) for IMPLEMENTED/PLANNED/UNSUPPORTED features. Earlier milestone descriptions below remain historical; no additional PLC semantics or other language/process/network/trace/grading feature is added.

This is a support plan, not a claim of implemented semantics. Sprint 0 provides a shell and contracts only. Alpha 0.1 is the first minimal ST vertical-slice target; MVP follows DEMO-001 and separately approved increments. Future entries are uncommitted and require scoped tasks.

M1A/M1B are accepted. M1C adds a declared standard-FB catalog, typed normalized calls, transactional private instance state and headless TON code. TON acceptance remains pending until all validation/runtime tests pass; see [M1C report](../M1C_REPORT.md) for actual evidence. See [TYPE_SYSTEM.md](TYPE_SYSTEM.md), [CANONICAL_PLC_IR.md](CANONICAL_PLC_IR.md), [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md), and [TON_SEMANTICS.md](TON_SEMANTICS.md). This does not implement ST execution, a language frontend, process/network simulation, trace recording, or a completed educational vertical slice.

| Current implementation                                                           | Status                                                |
| -------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Alpha values / conservative typing / symbols / source anchors                    | M1A accepted                                          |
| Direct IR assignment, expressions, IF execution                                  | M1B accepted by technical-lead review                 |
| Entry memory images and transactional commit/rollback                            | M1B accepted by technical-lead review                 |
| Checked INT / finite canonical REAL / TIME comparisons                           | M1B accepted by technical-lead review                 |
| One explicit cyclic task, positive period, operation budget, deterministic clock | M1B accepted by technical-lead review                 |
| ST frontend or user-facing execution                                             | Not implemented                                       |
| Typed standard-FB definitions/instances/bindings and validation                  | M1C code and validation tests present; review pending |
| Headless standard TON, transactional state and Q/ET snapshots                    | M1C code present; runtime validation pending          |
| User FB/function execution, TOF/TP/counters/edge blocks                          | Not implemented                                       |
| Process/network simulation, protocol, trace recorder, grading                    | Not implemented                                       |

| Area           | Alpha 0.1 target                                                                                       | MVP target                                                   | Future support                                                 |
| -------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ | -------------------------------------------------------------- |
| Languages      | ST only                                                                                                | ST; LAD after DEMO-001 and M6 approval                       | FBD (M7), educational STL/IL (M8)                              |
| Scalar types   | BOOL, INT, REAL, TIME                                                                                  | Add integer widths and LREAL when required by exercises      | BYTE, WORD, DWORD, SINT, DINT, USINT, UINT, UDINT and profiles |
| Compound types | None                                                                                                   | Selected ARRAY/STRUCT support after explicit semantic design | Nested composites and additional type facilities               |
| POU            | One entry Program                                                                                      | Functions, Function Blocks, explicit instances               | Multiple programs/tasks and richer organization                |
| Variables      | Explicit declarations; named input, output and internal variables for the single program               | Globals, POU locals, FB instance state                       | Retained state and explicit address/profile adapters           |
| Assignment     | Typed scalar assignment                                                                                | Composite assignment after semantics are specified           | Profile-specific extensions through adapters                   |
| Expressions    | Literals, variable reads, parentheses; BOOL AND/OR/NOT; numeric +, -, *, /; =, <>, <, <=, >, >=        | Broader numeric/bitwise operators and explicit conversions   | Additional validated standard functions                        |
| Type rules     | No implicit BOOL/numeric conversion; numeric bounds and REAL precision specified before implementation | Documented conversion/overflow rules                         | Explicit vendor profiles; no changes to generic semantics      |
| Control flow   | IF / THEN / ELSE / END_IF                                                                              | ELSIF, CASE and bounded FOR/WHILE/REPEAT only when approved  | Additional flow facilities under VM budgets                    |
| Timers         | TON only, one explicit instance model designed with virtual time                                       | TOF and TP when exercises need them                          | More standard blocks/profiles                                  |
| Counters       | None                                                                                                   | CTU, CTD, CTUD when requested                                | Profile extensions                                             |
| Edge blocks    | None                                                                                                   | R_TRIG, F_TRIG when requested                                | Additional validated blocks                                    |
| Diagnostics    | Typed compiler/runtime fault vocabulary with source mapping                                            | Rich explanations and related locations                      | Localization and profile guidance                              |
| Runtime        | One deterministic bounded VM for ST-lowered IR                                                         | Same VM for every approved language                          | Additional scheduling profiles via explicit adapters           |

## Scope interpretation

Alpha declarations, one Program, and named variables are the minimum support needed to exercise the requested assignments and TON. This does not authorize implementation during Sprint 0. TIME literal syntax, integer width/range, REAL representation, division errors, output commit on fault, IF evaluation, and TON boundaries must be specified and tested before the respective M1/M2 work ships. Do not infer vendor behavior.

No ST parser, VM, timers/counters, language editors, process/network simulation, or protocols exist in Sprint 0. Do not implement LAD/FBD/STL production editors before DEMO-001 proves the architecture. New features must preserve source mappings, deterministic trace behavior, resource limits, and cross-language equivalence.

# ADR-0009: Typed semantic foundation before execution

- Status: Accepted by technical-lead M1A code review on 2026-10-14; FB call shape remains provisional
- Date: 2026-10-14
- Scope: M1A-001 through M1A-012

## Context

[ADR-0002](0002-canonical-plc-ir.md) intentionally deferred IR instructions and numeric semantics to M1. The approved M1A assignment now requests those representations and their validator, explicitly excluding execution. This record fills that open decision; it does not supersede the single-runtime, source-mapping, sandbox, clock, or renderer decisions.

## Decision

Keep values, compatibility rules, identities, declarations, diagnostics, and source anchors in `plc-types`. Keep typed semantic expressions, ordered statements, structured conditionals, reserved FB call bindings, and validation in `plc-ir`. No additional crate or dependency is necessary.

Use the minimal generic Alpha profile: BOOL as `bool`, INT as `i16`, REAL as validated finite `f32` with canonical positive zero, TIME as a nonnegative `u64` microsecond duration. No cross-type assignment or numeric promotion is supported. TIME arithmetic is unsupported. Exact same-type equality and numeric/TIME ordering are supported by the type rules. See [TYPE_SYSTEM.md](../spec/TYPE_SYSTEM.md); arithmetic evaluation remains unimplemented and requires M1B review.

Use opaque nonzero typed IDs allocated by a project assembler, separate from display names and storage. Alpha names use deliberate ASCII case-insensitive resolution. Source references identify a document and element; optional text spans accommodate text and future graphical sources through the same registry.

Choose typed structured IR with immutable candidate programs. A validator returns diagnostics or an immutable borrowed `ValidatedProgram`. A call shape can describe future TON inputs/outputs and persistent instance identity, but all calls are rejected until signatures and execution are separately approved. This does not declare or implement TON.

## Alternatives

| Approach                                 | Assessment                                                                                                                                                      |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Structured semantic IR                   | Simple typed data for the approved expression/assignment/IF subset; ordered bodies; source mapping without syntax or storage details. Chosen.                   |
| Basic blocks and branches                | Appropriate for later lowering/control-flow analysis, but introduces target bookkeeping without a current loop or execution requirement. Deferred.              |
| Stack bytecode                           | Couples semantic representation to operand stack, addresses, and instruction scheduling before a VM exists. May become a separate lowering target after review. |
| ST AST or graphical document as IR       | Couples the common core to syntax or editor geometry and prevents one language-neutral semantic boundary. Rejected.                                             |
| Generic JSON/string opcode               | Loses exhaustiveness, typed operands, and constructor invariants. Rejected.                                                                                     |
| Separate source/diagnostic/symbol crates | No dependency-direction benefit at this size; adds unnecessary fragmentation. Rejected.                                                                         |

## Consequences and migration

`plc-types` remains independent of `plc-ir`; `plc-ir` depends only on `plc-types`. Existing runtime, trace, simulation, network, grading, and language crates remain reservations. Frontends will eventually assemble candidates and submit them for validation; a future runtime will require a validated representation plus its own execution/sandbox checks.

The M0 text-only `SourceLocation` Rust constructor and public diagnostic fields are replaced by document/element anchors, optional `TextSpan`, validated diagnostic construction, and immutable getters. Category and severity wire names stay the same. There are no existing consumers outside `plc-types` tests, no persisted projects, and no diagnostic IPC DTO to migrate. Core-information IPC v2 and the accepted shell are unchanged.

Typed values have checked serde round trips for future fixtures/boundaries; TIME serializes as decimal microseconds per ADR-0003/0008. IR, symbol tables, and source registries have no inbound deserialization or persistence schema. Newer persisted IR compatibility remains a later specification.

Validation limits protect representation traversal only. They provide no execution-budget or VM sandbox guarantee. The M1A foundations are accepted; the call shape is provisional and must not freeze FB/TON semantics. M1B's assigned execution decisions are recorded separately in [ADR-0010](0010-transactional-alpha-runtime.md).

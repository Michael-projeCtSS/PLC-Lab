# Canonical PLC semantic IR

> **Prototype-era specification (milestones M1-M4B).** The semantic rules below remain implemented and tested in the current product; status lines and statements about missing features, milestones or the user interface are historical. Current overview: [PRODUCT_ARCHITECTURE.md](../PRODUCT_ARCHITECTURE.md) and [PLC_FEATURE_MATRIX.md](PLC_FEATURE_MATRIX.md).

Status: M1A and M1B accepted by technical-lead review. M1C replaces the provisional FB call under [ADR-0011](../adr/0011-function-block-instance-model.md). plc-ir remains execution-independent; validated candidates lower to a separate owned plan in plc-runtime. No parser, bytecode, or trace producer exists. See [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md), [ADR-0002](../adr/0002-canonical-plc-ir.md), and [ADR-0009](../adr/0009-semantic-foundation.md).

## Representation choice

| Alternative                      | Benefit                                                                                               | Cost / decision                                                                                                                   |
| -------------------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Structured semantic IR           | Direct typed operands; ordered bodies; conditional branches structurally owned; simple source mapping | Chosen for the approved straight-line and IF subset. Later lowering may produce basic blocks or bytecode.                         |
| Basic blocks with branch targets | Useful for control-flow analysis, loops, and execution lowering                                       | Requires target identity/termination/reachability rules before those features exist. Deferred.                                    |
| Stack bytecode                   | Compact and convenient for one possible VM                                                            | Exposes stack effects/storage/scheduling too early. Keep any future bytecode distinct from the canonical semantic representation. |

The IR has no ST tokens, parentheses, declaration syntax, THEN/END_IF nodes, LAD grid/contact classes, FBD coordinates/wires, or STL mnemonic text. Source language exists only in source metadata. Enums identify semantic operations; payloads have Rust types rather than arbitrary JSON/string opcodes. Structured semantic expressions can be assembled directly from any frontend's analyzed document.

## Abstract schema

The following notation describes data, not executable source syntax:

```text
Program = (SymbolTable, SourceMap, FunctionBlockCatalog, entry: PouId, ordered Body)
Body = [Statement]
Expression = (source: SourceRef, result: PlcType, ExpressionKind)
ExpressionKind = Constant(PlcValue)
               | Read(SymbolId)
               | Unary(UnaryOperator, Expression)
               | Binary(BinaryOperator, Expression, Expression)
Statement = (source: SourceRef, StatementKind)
StatementKind = Assign(target: SymbolId, value: Expression)
              | If(condition: Expression, then: Body, else: Body)
              | FunctionBlockCall(instance: FunctionBlockInstanceId, typed bindings)
```

Every expression has an explicit claimed result type. Constants derive it from their value constructor. Reads/operators accept a claim that validation checks against declarations and compatibility rules. This permits candidate IR to report meaningful frontend/program errors rather than assume a producer is correct. No public struct fields are exposed for convenience; constructors and immutable accessors form the API. Enum cases are the public semantic vocabulary.

One registered Program is the entry. Other POU kinds provide declaration ownership only. The candidate owns immutable symbol/source registries. Read and assignment targets use stable IDs and must be global or local to the entry; no addresses, variable storage, task definition, or process image exists.

## Alpha operations and static semantics

Let `Γ(id)` be the visible symbol's declared type. Rules in [TYPE_SYSTEM.md](TYPE_SYSTEM.md) are implemented once in `plc-types::compatibility`.

- Constant(v): result must be the type of v, guaranteed by construction.
- Read(id): id must be visible; result must equal Γ(id).
- Unary(op, e): e must validate; `unary_result(op, type(e))` must exist and equal the claimed result.
- Binary(op, l, r): both operands must validate; `binary_result(op, type(l), type(r))` must exist and equal the claimed result.
- Assign(id, e): target must be visible, e valid, and assignment compatibility must be identity. M1B's assigned Alpha rule additionally rejects writing an entry-program Input; it does not impose Function/FB parameter policy.
- If(c, a, b): c must have BOOL type; both bodies validate in stored order. Empty bodies are legal, including absent ELSE represented by an empty body.

Unary vocabulary: NOT and numeric negation. Binary vocabulary: AND, OR, add/subtract/multiply/divide, equal/not-equal, less/less-equal/greater/greater-equal. The IR records operator nesting explicitly, without syntax precedence rules. Arithmetic and boolean evaluation are not implemented. No operand is read from live state, no result value is calculated, and no branch is selected during validation.

Bodies have explicit vector order. Structured IF owns both branches, so dangling targets, fallthrough terminators, jumps into another scope, and branch cycles cannot be expressed. No loop, jump, return, recursion, or arbitrary control-flow node is approved. Both branches are validated even for a constant condition. No condition or arithmetic constant folding occurs.

M1B's separate runtime specification defines evaluation/fault/scan semantics. Validity here does not establish that division is safe, that overflow cannot happen, or that a scan stays within its independently enforced execution budget.

## Declared function-block calls (M1C)

`FunctionBlockCall` carries only FunctionBlockInstanceId and a vector of typed CallBinding cases: Input(ParameterId, Expression), Output(ParameterId, SymbolId), or reserved InOut(ParameterId, SymbolId). Its constructor rejects duplicate parameter IDs across all directions with PLC-COMP-022. Statement SourceRef identifies the call. The Program-owned catalog supplies the single authoritative instance -> definition relationship; no call supplies a competing definition ID.

Definitions contain an explicit semantic kind and ordered typed signature. Standard TON's immutable signature is IN/Input/BOOL/required (ID 1), PT/Input/TIME/required (ID 2), Q/Output/BOOL/optional (ID 3), ET/Output/TIME/optional (ID 4). Numeric definition IDs never identify TON. Parameter IDs belong to the resolved definition. The instance owns its identity, definition reference, POU owner, name and optional source; private runtime state lives in plc-runtime, outside semantic IR and user symbol memory.

Validation checks catalog sources/owners/definition references, call instance and entry ownership, supported kind, required inputs, membership, direction, types and writable visible targets. Entry Input output associations are forbidden by PLC-COMP-014. Only valid TON calls can execute; user-defined signature metadata is not executable. Inputs are checked in signature order; lowering normalizes IN then PT and Q then ET. See [STANDARD_FUNCTION_BLOCKS.md](STANDARD_FUNCTION_BLOCKS.md) and [TON_SEMANTICS.md](TON_SEMANTICS.md). The old CallInput/CallOutput API and definition argument are removed; no persisted IR or frontend schema needs migration.

## Source mapping and trace readiness

Each expression/statement requires a registered document/element reference. Declarations may have optional references, which must also resolve if present. [SOURCE_MAPPING.md](SOURCE_MAPPING.md) describes language-neutral metadata. Shared anchors allow one source element to lower to multiple semantic operations. Registered language must match document provenance; coordinates are optional.

Future execution and trace events can retain these anchors for highlighting, monitoring, diagnostics, and replay. A source anchor describes provenance, not evidence of execution. No UI computes logic or fabricates a trace, and there is no trace schema/recording in M1A.

## Validation boundary

`validate(&Program)` returns `Result<ValidatedProgram<'_>, Vec<Diagnostic>>`. The wrapper has a private constructor and immutably borrows its candidate, so its checked representation cannot be mutated. It contains no runtime state or evaluated values.

Validation checks entry kind/existence, symbol identity and visibility, declared/inferred expression types, assignment compatibility, boolean/comparison/arithmetic typing, BOOL conditions, source registration, structured children, and declared FB catalog/binding semantics. Nonzero identities, unique declarations, registry consistency, valid spans, finite REAL, bounded INT, and nonnegative TIME are enforced by the respective constructors.

Diagnostics are compiler errors with stable codes, plain English messages, resolved optional source metadata, and related symbol/POU IDs where appropriate. Unknown source references report document/element IDs in the message and have no invented source location. Invalid operands suppress dependent type errors, while independent operands and both branches are checked. Traversal order is reproducible: declaration ID order, then statement order with left-before-right expressions and then-before-else branches.

There are deterministic validation limits: maximum depth 64 (top-level statement depth 0; children increment depth), and 10,000 visited symbol/FB declarations, parameters, binding, expression and statement nodes. Exceeding either emits one PLC-COMP-012 and halts further traversal. These limits protect the validator; they are not VM budgets. Construction/allocation and destruction of candidate IR are not an untrusted project sandbox. There is no inbound IR deserializer; future import/lowering must bound allocation/nesting before construction too.

Student/program errors return diagnostics. The only `expect` calls in production refer to core-owned diagnostic code/message constants, and the symbol table's internal index accesses assume its atomic private-index invariant. A violation there is an implementation defect, never an invalid student program. APP-CORE-001 reserves the internal-failure code; no student error is relabeled as it.

## Review example

For the conceptual logic `Motor := Start AND NOT Stop;`, declarations give Start, Stop, and Motor BOOL types and distinct symbol IDs. The semantic shape is:

```text
Assign Motor#3 @source(assign)
  Binary AND : BOOL @source(and)
    Read Start#1 : BOOL @source(start)
    Unary NOT : BOOL @source(not)
      Read Stop#2 : BOOL @source(stop)
```

Each source label stands for a registered `SourceRef`, not an operation or string opcode. This example contains no executed output and no assumed scan behavior.

## Evidence and compatibility

Integration golden tests construct and validate the four requested conceptual programs without a parser/VM, then inspect their semantic shape and typing. Negative tests cover unknown/foreign symbols, unsupported assignments/operands, false result claims, non-BOOL conditions, invalid entry, dangling sources, both branches, FB relationships/bindings, duplicate ports, deterministic diagnostics, and traversal limits. M1C adds positive TON validation, source-backed catalog/binding failures and a private defensive duplicate check.

Test assemblers live only in `crates/plc-ir/tests/support`; production APIs have no test-only public fields. `plc-ir` adds no dependencies and has no serde/persistence schema. Any later persisted IR requires an explicit version and compatibility/migration policy per ADR-0008. Changing these semantics after review requires the ADR/approval process in AGENTS.md.

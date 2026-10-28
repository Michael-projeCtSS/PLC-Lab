# Language-independent symbol model

> **Prototype-era specification (milestones M1-M4B).** The semantic rules below remain implemented and tested in the current product; status lines and statements about missing features, milestones or the user interface are historical. Current overview: [PRODUCT_ARCHITECTURE.md](../PRODUCT_ARCHITECTURE.md) and [PLC_FEATURE_MATRIX.md](PLC_FEATURE_MATRIX.md).

Status: M1A declarations and resolution implemented in `plc-types::symbols`. There is no parser, storage allocation, memory/process image, or POU execution.

## Identity and names

`SymbolId` and `PouId` are different opaque nonzero u32 types. Constructors reject zero. A frontend/project assembler allocates unique IDs in each namespace and reuses them throughout one compiled representation. Allocation must be explicit and reproducible; it is not derived from a name hash, memory address, wall-clock time, or renderer state. The current API deliberately does not create a hidden global ID generator.

A symbol has private fields with immutable accessors: ID, original source-visible name, `PlcType`, `Scope`, `VariableCategory`, and optional `SourceRef`. IR reads/writes refer to `SymbolId`, never names. A rename can retain an ID; tests show the same identity with two different display names. Cross-recompilation/persistence stability is not promised here; the future project assembler must define that policy.

Alpha identifiers deliberately use ASCII `[A-Za-z_][A-Za-z0-9_]*`. This is an identifier invariant, not an ST lexer. Resolution uses ASCII uppercase normalization, preserving original spelling for display. `Motor`, `motor`, and `MOTOR` collide within a namespace. Unicode identifiers and qualified-name syntax require a later approved frontend/profile policy; host Rust/JavaScript name rules do not define PLC resolution.

## Scope and category

| Concept                   | Implemented representation                                      |
| ------------------------- | --------------------------------------------------------------- |
| Global declaration        | `Scope::Global`, category Local                                 |
| Program-local declaration | `Scope::Pou(PouId)` owned by a registered Program               |
| Input parameter/variable  | `VariableCategory::Input` in a POU                              |
| Output parameter/variable | `VariableCategory::Output` in a POU                             |
| In-out parameter/variable | `VariableCategory::InOut` in a POU                              |
| Local/internal variable   | `VariableCategory::Local` in a POU or global scope              |
| POU ownership             | `Pou` with ID/name and Program, Function, or FunctionBlock kind |

Function and FunctionBlock kinds are ownership tags only. They do not implement function bodies, invocation, instance storage, retained state, or call semantics. A POU must be registered before its variables can be declared. Input/Output/InOut categories are rejected in the global scope. Future extensions can add storage and parameter policies without replacing symbol identity.

Categories describe declaration/interface roles, not universal physical I/O. M1B compiles a separate Alpha entry memory layout: selected entry Inputs are read-only external inputs, Outputs are writable external outputs, and entry Locals/globals are internal state. Non-entry POU symbols are excluded. The validator rejects entry Input assignments with PLC-COMP-014; lowering rejects executable entry InOut with PLC-COMP-015. No Function/FB parameter semantics, physical addresses, or device behavior is inferred. See [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md).

## Resolution

`resolve(Scope::Pou(owner), name)` checks a valid registered owner, then the owner's namespace, then globals. A local deliberately shadows a global. Lookup in global scope sees globals only. Other POUs' locals are invisible. POU names live in their own case-insensitive namespace and cannot duplicate another POU name; variables live in per-scope namespaces.

`get(SymbolId)` provides declaration identity lookup. `visible(scope, id)` additionally checks scope visibility for IR validation. ID references cannot bypass ownership, even when a foreign symbol exists. Global IDs remain visible to any valid POU context. An explicit global identity can refer to a shadowed global; future name lowering must use the resolution policy above.

`declare_pou` and `declare` are the explicit mutation points. They check all invariants before changing the two indexes. Duplicate rejection leaves existing declarations intact. Candidate `Program` takes ownership of the completed table and exposes it immutably.

## Errors and diagnostics

| Error                                                  | Stable diagnostic |
| ------------------------------------------------------ | ----------------- |
| Unknown name or SymbolId                               | PLC-COMP-001      |
| Duplicate name, same type/category                     | PLC-COMP-002      |
| Invalid owner/lookup/category scope, foreign ID access | PLC-COMP-003      |
| Same normalized name with different type or category   | PLC-COMP-004      |
| Duplicate SymbolId or PouId                            | PLC-COMP-005      |
| Invalid source-visible identifier                      | PLC-COMP-006      |

Redeclaration checks apply only within the same scope. They distinguish incompatible redeclaration from an ordinary duplicate. POU name duplicates are ordinary duplicates; no function overloading is implemented. `SymbolError` is a typed error with a reusable `diagnostic()` adapter. Declaration producers can attach their source location; IR validation attaches the source of the failed access and related symbol ID.

Tests cover global fallback, local shadowing, case collisions, category/owner representation, duplicate IDs/names, incompatible type/category redeclarations, unknown names/IDs, invalid owners, foreign visibility, invalid identifiers, atomic failure, and stable diagnostic codes.

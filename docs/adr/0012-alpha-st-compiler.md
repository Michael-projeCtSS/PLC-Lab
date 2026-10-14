# ADR-0012: Bounded Alpha ST compilation and presentation-only editor

- Status: Authorized by M2A; technical-lead source review required before M2B
- Date: 2026-10-14
- Scope: M2A-001 through M2A-040

## Context

M1C independent validation passed for a4a22f19c7bcdbd1c1d78a38804c29e15da32828. The M2A assignment authorizes the first ST source frontend and compilation UI while freezing accepted runtime/TON/IR/type semantics. An owned compile result must coexist with the existing borrowed ValidatedProgram without self-referential storage.

## Decision

Use handwritten, bounded Rust lexing/parsing in languages/st, a distinct source-spanned ST AST, and semantic compilation through the existing type compatibility, SymbolTable, FB catalog and Canonical IR constructors. Always run the unchanged IR validator before constructing the opaque immutable CompiledProgram. It owns the candidate and provides explicit borrowed revalidation at existing ExecutionPlan lowering; only headless integration tests execute it in this milestone.

Allocate identities in deterministic declaration/syntax order. Bound source bytes before allocation, token count, both recursive and flat-tree depth and diagnostic collection. Interpret numeric literal signs at the source boundary so INT minimum is representable; use existing PlcValue/PlcReal constructors and checked integer TIME microseconds. No general expression evaluation or new runtime rule is introduced. Preserve the common scalar-column SourceMap/SourceRef model and convert spans to Monaco UTF-16 only in the presentation adapter.

Add pure compile IPC through app-contract and a Tauri blocking worker. IPC v3 rejects older/newer versions consistently. Return an explicit success/failure union, declaration summaries and checked diagnostic DTOs; never expose canonical IR/ExecutionPlan or create a runtime as a Compile side effect. Monaco 0.57.0 and its worker are locally bundled using Vite; the editor uses the package's exported editor API/worker paths. Syntax highlighting, editing, markers and symbol display belong to React. No frontend parser, type checker, PLC evaluator or execution controls are introduced. Request/source revisions use local monotonic counters, independent of wall time.

## Alternatives and consequences

Parsing dependencies add unnecessary complexity for this subset; none is added. Translating ST to JavaScript, interpreting AST, compiling editor models directly or adding a second TON violates existing architecture. A self-referential owned ValidatedProgram would add unsafe/lifetime complexity; immutable ownership plus the existing validator is sufficient. Runtime and core dependencies remain unchanged. st-frontend production imports only plc-types/plc-ir; runtime/clock are test-only dependencies. The application adapter imports the ST frontend; no core crate depends on it.

Only IPC consumers migrate together from v2 to v3; no persisted project/IR schema exists. M1 historical validation SHA/evidence are retained. Full existing M1 regressions, source-to-runtime tests, Unicode conversion, stale-result tests and actual native compile IPC/Monaco acceptance must run on independent Windows/Linux CI before M2A PASS. Successful compilation does not authorize runtime UI or process/network/trace/grading implementation.

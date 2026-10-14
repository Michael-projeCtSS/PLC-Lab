# ADR-0002: One canonical PLC IR

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-002, FND-003

## Context

Equivalent ST, LAD, FBD, and STL programs must have equivalent behavior. Executing editor objects directly would couple semantics to presentation.

## Decision

Each language frontend owns parsing, document validation, semantic analysis, and lowering to one vendor-neutral Canonical PLC IR. Only plc-runtime executes that IR. Preserve source locations through lowering for trace and diagnostics. plc-types owns shared semantic vocabulary; plc-ir must not import a frontend, UI, or desktop host.

IR instructions, typing rules, numeric behavior, and version compatibility remain unspecified until M1. Future changes to approved semantics require a proposed ADR with migration impact and explicit approval. Do not claim conformance to a vendor or the full IEC language standard.

## Alternatives

Separate language runtimes cause semantic drift. Interpreting React editor models breaks the core boundary. Translating student programs to JavaScript or native executables violates sandboxing. All are rejected.

## Consequences and migration

Frontends must preserve source mappings and eventually pass cross-language equivalence tests using common event sequences and trace normalization. No programs or IR artifacts exist to migrate.

## Sprint boundary

Create module boundaries and feature plans. Do not implement an IR instruction set, parser, or VM.

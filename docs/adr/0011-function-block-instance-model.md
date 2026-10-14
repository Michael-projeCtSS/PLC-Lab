# ADR-0011: Declared function-block instances and transactional TON

- Status: Authorized by M1C; technical-lead review required before M2
- Date: 2026-10-14
- Scope: M1C-001 through M1C-022

## Context and authority

M1B code review accepted the owned execution plan, entry roles, cloning, last-attempted input image, transactional commit/rollback, clock, arithmetic, budget, and reset. These decisions remain unchanged. M1A's function-block call shape was explicitly provisional. M1C authorizes changing it and implementing standard TON only.

## Decision

Keep definitions, typed signatures, and instance declarations in plc-types. Program owns a function-block catalog in plc-ir. A call contains an instance ID and typed bindings; only the declared instance stores the definition ID. Definition kind explicitly selects StandardFunctionBlock::Ton; numeric definition IDs never imply behavior.

Standard TON has authoritative ordered metadata for IN BOOL, PT TIME, Q BOOL, ET TIME. Both inputs are required; output associations are optional. Parameter IDs are scoped to a definition. Use a unified typed binding vocabulary for Input, Output, and reserved InOut; InOut has no execution behavior in M1C. Constructors reject duplicate bindings; validation also guards duplicates and all semantic references/types/permissions.

Validate the catalog relationship and entry ownership before execution. Definitions/instances can be assembled in either order, but dangling references never pass validation. Instance names are metadata, with deliberate ASCII case-insensitive uniqueness per POU. Runtime identity is FunctionBlockInstanceId. User-defined signature metadata is possible; user definitions are not executable.

Lower calls into owned typed TON instructions. Normalize IN before PT, then Q before ET, independent of producer binding order. Runtime performs no signature/name lookup. All entry TON declarations receive independent private state; non-entry instance declarations remain metadata and cannot be called by the entry.

Clone committed instance state with the accepted working PLC memory. TON reads fixed scan-start virtual time. Publish both memory and instance state only after execution and clock/count completion checks succeed. Any fault discards both working maps. Last-attempted inputs still remain observable. Reset clears every timing epoch and restores Q false / ET zero.

Expose only immutable Q/ET instance snapshots. Private activation timestamps never become PLC symbols or public snapshot fields. Preserve existing per-operation budgeting and add the exact TON charges in the assignment. TON runs in plc-runtime, not simulation-core, React, or trace-core.

## Alternatives

| Alternative                                    | Decision                                                                                     |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Call carries both definition and instance IDs  | Rejected: competing relationships permit one instance to change behavior between call sites. |
| Definition ID 1 means TON                      | Rejected: identity must remain independent of semantic kind.                                 |
| Retain omitted inputs / source binding order   | Rejected for Alpha: both inputs are mandatory and signature order is canonical.              |
| Timer state stored in generated SymbolIds      | Rejected: private implementation state is distinct from PLC variables.                       |
| Mutate committed timer before scan completion  | Rejected: breaks accepted transactional rollback.                                            |
| Shared/unsafe/copy-on-write state optimization | Rejected in this batch: accepted BTreeMap cloning remains appropriate.                       |
| Generic plugin/custom FB executor              | Deferred: only typed standard TON execution is authorized.                                   |

## Migration and consequences

The provisional FunctionBlockCall constructor loses its definition argument and separate binding vectors. CallInput/CallOutput are replaced by typed CallBinding cases. Program adds a checked-at-validation catalog through an explicit ownership API. Existing no-block Program construction remains compatible. Update all direct-IR tests; no persisted IR, language frontend, or IPC schema exists to migrate.

M1B runtime faults keep their codes/meaning. Add compiler diagnostics for catalog/binding failures; reuse existing symbol/write errors only where meanings match. TON normally adds no RuntimeFault code; impossible time/state reversal is APP-CORE-001.

No dependency or crate is added. The accepted runtime -> simulation-core -> plc-types direction remains unchanged. Trace recording, languages, process/network simulation, other blocks, and UI execution remain outside scope. See [STANDARD_FUNCTION_BLOCKS.md](../spec/STANDARD_FUNCTION_BLOCKS.md) and [TON_SEMANTICS.md](../spec/TON_SEMANTICS.md).

# Standard function-block semantic foundation

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [PLC_FEATURE_MATRIX.md](PLC_FEATURE_MATRIX.md). Kept for engineering history.

Status: M1C authorized implementation; review required before M2. [ADR-0011](../adr/0011-function-block-instance-model.md) changes the explicitly provisional M1A API. M1B foundations remain accepted.

## Distinct concepts

| Concept              | Ownership / identity                                                                           |
| -------------------- | ---------------------------------------------------------------------------------------------- |
| Definition           | FunctionBlockId, explicit FunctionBlockKind, ordered typed ParameterDefinitions                |
| Instance declaration | FunctionBlockInstanceId, exactly one definition ID, POU owner, display name, optional source   |
| Call                 | One instance ID and typed parameter bindings, with statement SourceRef                         |
| Binding              | ParameterId belonging to the resolved definition; input expression or output writable SymbolId |
| Persistent state     | Private runtime instance map keyed by FunctionBlockInstanceId, separate from SymbolId memory   |

Program owns a FunctionBlockCatalog of definitions and instances. Calls resolve instance -> definition -> semantic kind/signature; they cannot supply another competing definition. IDs are nonzero typed identities and definitions/instances are immutable after declaration. Catalog insertion rejects duplicate identities and duplicate instance names in an owner's ASCII case-insensitive namespace. Forward definition/owner references can exist during assembly but are rejected during validation.

FunctionBlockKind distinguishes Standard(StandardFunctionBlock::Ton) from UserDefined signature metadata. Only TON is executable. Future standard enum variants/custom bodies can extend this infrastructure when approved; no TOF/TP/counter/body/plugin behavior is implemented. Standard factories own their exact signature; callers cannot alter a TON definition's parameter list.

## Authoritative TON signature

Parameter IDs are local to each definition; the same IDs can appear in another definition. Definition IDs can have any valid value and do not select behavior.

| Signature order | ParameterId | Name | Direction | PlcType | Association rule       |
| --------------- | ----------- | ---- | --------- | ------- | ---------------------- |
| 1               | 1           | IN   | Input     | BOOL    | Required on every call |
| 2               | 2           | PT   | Input     | TIME    | Required on every call |
| 3               | 3           | Q    | Output    | BOOL    | Optional               |
| 4               | 4           | ET   | Output    | TIME    | Optional               |

TonParameter is typed vocabulary for these standard parameter identities; names are metadata only. There are no retained/default omitted inputs. Q/ET update inside the instance even without associations. Output associations copy new instance values into permitted PLC symbols.

## Binding and validation

CallBinding has Input(parameter, Expression), Output(parameter, SymbolId), and reserved InOut(parameter, SymbolId) cases. InOut is representable as metadata for future evolution, but no TON parameter accepts it and no runtime InOut behavior exists. FunctionBlockCall::new checks at-most-once binding across all directions. Its duplicate error exposes the stable diagnostic; validation independently checks the same invariant.

Validation checks instance/definition existence, authoritative relationship, owner/source validity, supported kind, parameter membership/direction, every required input, expression/target type, symbol existence/visibility, and output permission. Entry Inputs cannot be Q/ET targets. No names are used as runtime identity. Invalid calls never produce ValidatedProgram.

Catalog metadata and binding traversal contribute to the existing bounded validation node count. Catalog/source iteration is deterministic. Input binding checks follow signature order where resolved; diagnostics retain the call source and instance/symbol identities where meaningful.

## Lowering and execution

Lowering resolves the validated catalog once and normalizes bindings into signature order: IN, PT; optional Q, ET. The owned runtime instruction contains instance ID, two owned expressions, optional typed target IDs, and the statement source. It contains no string-based signature lookup, second definition identity, source syntax, or editor objects.

Entry TON instances get independent default private state. Non-entry instances are excluded from the executable instance layout; entry calls cannot access them. Unsupported user definitions do not lower as TON. The owned execution representation remains distinct from canonical semantic IR.

See [TON_SEMANTICS.md](TON_SEMANTICS.md) for algorithm, scan time, skipped/repeated calls, rollback, budget, and snapshots. No frontend or IPC is added.

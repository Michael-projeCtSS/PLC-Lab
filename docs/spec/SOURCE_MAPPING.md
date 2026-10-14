# Source identity and mapping

Status: M1A language-neutral anchors and a checked registry implemented in `plc-types::source`. There are no language frontends, editor adapters, runtime highlights, or trace recording yet.

## Model

`DocumentId` identifies a source document/module within a compiled project. `SourceElementId` identifies an element within that document. Both are opaque nonzero u32 values; an element ID is unique per document, and the pair is the complete `SourceRef`. A `SourceLocation` contains that reference, `SourceLanguage` (ST, LAD, FBD, STL), and optional `TextSpan`.

IDs are allocated and retained by the future frontend/project assembler throughout one compiled representation. They do not encode names, line numbers, editor array indexes, coordinates, or language keywords. M1A provides checked ID construction and registry uniqueness, not an editor allocator or guarantees across recompilation. Multiple IR expressions/statements may share the same source reference after lowering. IDs can also distinguish subexpressions when a frontend supplies that detail.

`SourceMap` registers document IDs with language, then elements. It rejects unknown documents, duplicate document/element identity, and element/document language mismatch without replacing existing records. Maps use deterministic ID ordering. Symbol declarations can optionally reference an anchor; every IR expression and statement requires one and validation checks registration.

## Text and graphics

`TextSpan` uses validated one-based line/column positions, with columns counting Unicode scalar values. The start is inclusive, the end exclusive; equal positions represent a point diagnostic. End cannot precede start, including across lines. No filesystem path, UTF-8 byte offset, or Monaco UTF-16 column is assumed. A future Monaco adapter must explicitly convert Unicode scalar columns to its encoding.

ST/STL frontends will normally supply text spans. A synthesized node may share its originating element's anchor; it must not invent an unrelated location. LAD rung/contact/coil identities and FBD block/connection identities can use `SourceElementId` with no text span. Geometry and editor objects remain in document/presentation models. The core does not need a graphical node type or renderer dependency.

The language tag records provenance only. It does not select another type checker or runtime. All four languages use the same IR types and validation.

## Future consumers

| Consumer                            | How the anchor is intended to be used                                                                                  |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Online monitoring                   | Authoritative runtime evidence references source anchors and symbol IDs, resolved against the compiled representation. |
| Active LAD elements                 | Runtime/trace evidence identifies mapped contacts/coils/rungs; the UI highlights supplied evidence.                    |
| ST/STL executed-source highlighting | Execution evidence identifies anchors whose text spans map into the editor.                                            |
| FBD signal visualization            | Frontend lowering preserves node/connection anchors; future runtime evidence supplies signal values/paths.             |
| Compiler diagnostics                | Declaration and IR errors include a resolved source location when available.                                           |
| Timeline/replay                     | Ordered trace events will carry anchors associated with the same compiled project/version.                             |

This batch supplies identity and metadata only. It cannot decide whether a contact is active, whether an expression executed, or how a signal flowed. Frontends must never recompute PLC logic to generate these views. [UI_PRINCIPLES.md](UI_PRINCIPLES.md) remains authoritative for presentation.

Graphical many-to-one lowering may require a richer list of related anchors later. M1A has one primary source anchor per node; a frontend can reuse it on multiple IR nodes. Trace schema, project/version association, normalized cross-language trace comparison, and causality correlation remain future work.

## Boundary and tests

Source metadata has outbound serde serialization only; neither the registry nor IR has inbound deserialization. The M0 source record changes from text-only document strings to the typed document/element model (ADR-0009); no current IPC consumer is affected.

Tests cover zero/max positions, reversed/point/multiline spans, all language tags, text and graphical anchors, duplicate and unknown IDs, language mismatch, unchanged registry after errors, and diagnostic source serialization. Direct construction tests prove identical semantic operations accept each language's anchors; they do not claim that any frontend exists or any program executes.

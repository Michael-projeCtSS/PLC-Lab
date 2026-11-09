# ADR-0045: Project library documents

- Status: Accepted for 0.2.0-dev (CP9-A)
- Date: 2026-11-09

## Context

ADR-0044 compiles user FUNCTIONs and FUNCTION_BLOCKs written in the program document. A
project needs a place for reusable code apart from its program: blocks shared by the
program, kept in their own documents, saved with the project and compiled with every
build. The project format has stored a list of up to 32 Structured Text documents since
format 0, but PLC Lab only ever wrote and compiled the first.

## Decision

### Model

The first program document of a project is the **main program**: the one `PROGRAM`, and
optionally its own POUs. Every further document is a **library document** holding only
`FUNCTION` and `FUNCTION_BLOCK` POUs (up to 31). A `PROGRAM` in a library document is a
compile error that names the document. Library documents have an id and a name, unique
in the project (names ignore case); an empty document is allowed.

### Compilation

`st_frontend::compile_st_documents(main, library, config)` parses each document on its
own and compiles all of them into one canonical program for the one PLC runtime. The
source map already identifies documents (`SourceRef.document_id`): the main program is
document 1, library documents are 2 and above in order. While the compiler declares or
expands a POU it works in that POU's document, so every source anchor, compile
diagnostic and runtime fault location names the right document and line; diagnostics at
a call site stay in the caller's document. Errors of all documents are reported
together. `compile_st` is `compile_st_documents` without a library, so single-document
programs compile exactly as before.

The engine's `check_documents`, `download_documents` and `assess_documents` take the
library sources beside the program; `check`, `download` and `assess` are the same calls
without a library. Exercise checks (`requiredInterface`, `requiredPous`) see the POUs of
all documents.

### Transport (contract 8, additive)

`CompileRequest`, `BuildLoadRequest` and `CheckRequest` carry `library`:
`[{ id, source }]` in project order (default empty). `DiagnosticDto.document` and
`PouDto.document` name the library document id (`null` for the main program). The
workspace remembers the ids of the library in the PLC, so a later runtime fault names
its document. A successful download writes the downloaded library sources into the open
project like the main source.

### Persistence: project format 6

Format 6 gives documents after the first their meaning as library documents. Format 5
files migrate by setting the version; their content is kept as it is (PLC Lab never
wrote more than one document before, and a hand-made extra document is now compiled and
its errors shown instead of being dropped). A format 6 file opened by an older PLC Lab is
refused with "update PLC Lab" instead of being compiled without its library. Validation
adds: document names of 1 to 64 characters, ids and names unique.

### User interface

The project tree lists the main program and a "Project library" section with every
library document and "New library document". The editor shows the selected document;
a bar above it names the document and offers Rename and Delete (Delete asks once more).
Diagnostics are listed for all documents with the document's name, and following one
opens its document; the editor marks only the diagnostics of the shown document, in the
text they were computed for. The Library tab names the document of each user POU.
Library sources are saved like the main program (debounced, per document).

## Alternatives considered

- Concatenating all documents into one text before compiling: rejected, diagnostics and
  runtime faults would point at lines of a text nobody sees.
- A separate library format or file next to the project: rejected, the project format
  already stores several documents and the project is the unit that is saved, exported
  and imported.
- Libraries shared between projects: a later increment; they need versioning and an
  import model, while project-local documents need neither.

## Consequences

Programs can be split into a main program and reusable block libraries without a second
compiler path or runtime. Every document is compiled on every build, bounded by the
existing per-document size limit and the expansion budget of ADR-0044. Format 6 projects
need this or a newer PLC Lab.

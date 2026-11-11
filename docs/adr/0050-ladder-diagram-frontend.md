# ADR-0050: Ladder Diagram frontend and editor (first vertical slice)

- Status: Accepted for 0.2.0-dev (CP9-C); extended by [ADR-0051](0051-industrial-ladder-elements-and-power-observations.md)
  (document version 2, SET/RESET, edges, boxes, comparators, runtime power flow)
- Date: 2026-11-10

## Context

PLC Lab compiles Structured Text into the canonical PLC IR (ADR-0002), and the IR was
prepared for graphical languages (ADR-0021: associative AND/OR of any width, block output
reads). AGENTS.md requires LAD to be a rung/grid model, not generic graph nodes, and to
compile into the same IR and runtime as ST. ADR-0021 set three conditions for graphical
editors: persistent graphical element ids, power flow shown only from runtime data, and
no synthetic symbols visible to students.

## Decision

### Document model

A LAD program is a JSON document (`format: "plclab-lad"`, `version: 1`) stored as the main
program document of a project (`language: "LAD"`):

- `name`: the program name;
- `variables`: BOOL variables with direction `input`, `output` or `local` and an optional
  comment;
- `rungs`: each rung has an id, an optional comment, a **network** and one or more
  **coils**. A network is a tree of contacts (`normallyOpen` / `normallyClosed` with a
  variable), series blocks and parallel blocks. A coil is an output coil writing a
  variable.

Every rung, network element and coil has a persistent positive id, unique in the
document. New elements take the next free id; existing ids never change, so diagnostics
and runtime faults can point at a graphical element.

### Compilation (`languages/lad`, crate `lad-frontend`)

The frontend validates the document and lowers it directly into the canonical IR, with no
Structured Text text in between:

- a normally open contact reads its variable, a normally closed contact is `NOT` of it;
- a series is the AND of its items, a parallel block the OR of its branches (IR
  `Logical`, constant depth for wide rungs); an empty series as the whole network is TRUE
  (left rail to the coils);
- each coil is an assignment of the network's result, rung after rung, coil after coil.

These are exactly the semantics of the equivalent ST assignments. A rung reads the values
earlier rungs wrote in the same scan, as ST statements do.

Validation reports coded diagnostics (`PLC-LAD-001` to `006`) naming the rung and pointing
at the element: a malformed document, invalid or duplicate declarations, non-BOOL
variables (version 1), unknown variables, coils on inputs, rungs without coils, parallel
blocks with fewer than two branches, nested empty series, duplicate or zero ids, and size
and depth limits (256 KiB, 256 rungs, 4096 elements, 256 variables, depth 24). Names
resolve case-insensitively through the same symbol table as ST.

`lab_engine::compile_documents` compiles a main program that is a ladder document (a
JSON object declaring the ladder format; Structured Text cannot start with `{`) with the
LAD frontend into the same `CompiledProgram` as ST. Download, linking to the station's I/O,
tracing, forcing, online writes and grading therefore work unchanged. A ladder program
cannot use project library documents yet. The project model accepts `LAD` for the main
program only.

### Editor

The Program view shows the ladder editor for a LAD main program. Free Lab projects choose
the language (ST or LAD); switching starts a new program in the other language after a
confirmation. A new ladder program declares the scene's BOOL signals.

- Rungs are drawn as rails, series rows and parallel stacks (HTML/CSS, not a node graph).
- Contacts, coils and parallel blocks are selected and edited with explicit commands:
  a NO or NC contact in series after the selection, a NO or NC contact in parallel to it,
  NO/NC toggle, delete, the variable from a list (coils offer outputs and internal
  variables only), add and delete rungs and coils, rung comments, and a variable table.
- Edits keep the network valid (one-branch blocks collapse; only the whole network may be
  empty). The document is the program text: it is autosaved, compiled live (diagnostics
  in the problem list) and loaded with Build & Load like ST.

### Online monitoring

While the PLC runs the very document in the editor, each contact and coil shows the
value of its variable as the runtime reports it (`watch`): TRUE/FALSE, and a highlight
when TRUE. The editor computes no power flow. Showing the power along wires and through
branches needs the runtime probe channel of ADR-0021, which is not built yet.

## Equivalence

Tests prove that LAD and equivalent ST behave the same in the one runtime:

- every one of the 128 input combinations of a nested series/parallel/NC network with a
  rung reading an earlier rung's coil gives the same outputs in both languages;
- a self-holding rung follows a start/stop sequence over scans exactly like its ST
  assignment;
- the motor start/stop exercise graded with a LAD program passes every criterion, like
  its ST reference solution. Without the self-holding branch, LAD and ST fail the same
  criteria.

## Alternatives considered

- **Generating ST text from the rungs.** One more parser pass and source spans in
  generated text instead of element ids. Lowering into the IR keeps the graphical
  identity.
- **React Flow nodes.** Excluded by AGENTS.md for LAD. Rungs are a series/parallel tree,
  which a grid renders naturally.
- **Synthetic variables for per-element power flow.** The runtime would compute them,
  but they add symbols ADR-0021 wants hidden from students. The probe channel is the
  planned mechanism.

## Consequences and limitations

- Version 1 covers BOOL contacts and output coils. Set/reset coils, edge contacts, timers,
  counters, comparisons and other boxes, function blocks in rungs, library documents and
  exercises written in LAD are not part of this slice.
- Monitoring shows variable values, not power flow along wires.
- Switching a program's language replaces it: there is no automatic ST↔LAD conversion.

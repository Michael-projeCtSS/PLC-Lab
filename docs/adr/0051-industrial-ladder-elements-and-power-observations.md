# ADR-0051: Industrial ladder elements, scan order and runtime power observations

- Status: Accepted for 0.2.0-dev (CP9-D)
- Date: 2026-11-11
- Extends: [ADR-0050](0050-ladder-diagram-frontend.md) (document version 2, monitoring)

## Context

ADR-0050 delivered the first Ladder Diagram slice: BOOL contacts in series and parallel and
output coils, compiled into the canonical IR, monitored by variable values only. Real
ladder programs need latching coils, edge detection, timers, counters and comparisons,
and a student must see **where power flows** in the scan the PLC executed. AGENTS.md
forbids evaluating PLC logic in React, and ADR-0021 requires that synthetic symbols stay
invisible to students.

## Decision

### Document version 2

`plclab-lad` documents are written as `version: 2`; version 1 documents are still read
(they are a subset) and are rewritten as version 2 when edited. Version 2 adds:

| Element    | Document                                                                               |
| ---------- | -------------------------------------------------------------------------------------- |
| Variables  | types `BOOL`, `INT`, `REAL`, `TIME`; direction `input`, `output`, `local`              |
| Contacts   | `contact`: `normallyOpen`, `normallyClosed`, `positiveEdge`, `negativeEdge`            |
| Comparator | `compare`: `op` `eq`/`ne`/`gt`/`ge`/`lt`/`le`, `left` and `right` operands             |
| Boxes      | `block`: `TON`, `TOF`, `TP`, `CTU`, `CTD` with `instance`, `preset`, `reset`, `output` |
| Coils      | `kind`: `output` (default, omitted), `set`, `reset`                                    |
| Operands   | `{"variable": "Name"}` or `{"constant": "literal"}` (`TRUE`, `-5`, `2.5`, `T#1s500ms`) |

Every element keeps its persistent id (ADR-0050). Box instance names are unique
(case-insensitive) in the document and may not equal a variable name. A box has one
power input (`IN` for timers, `CU`/`CD` for counters) and passes power on its `Q`
output; `preset` is `PT` (TIME) for timers and `PV` (INT) for counters; `reset` is `R`
(CTU) or `LD` (CTD) and is refused for timers; `output` writes `ET` (TIME) or `CV`
(INT) to a variable of that type that is not an input.

### Compilation and scan order

The LAD frontend lowers the document straight into the canonical IR and calls the
existing standard function blocks; there is no LAD runtime. One scan executes:

1. rungs from top to bottom;
2. within a rung, the network depth-first from left to right. Each element assigns the
   power leaving it to a compiler-owned BOOL symbol `LAD#<id>`:
   - normally open: `power AND v`; normally closed: `power AND NOT v`;
   - positive edge: `power AND v AND NOT EDGE#<id>`; negative edge:
     `power AND NOT v AND EDGE#<id>`; then `EDGE#<id> := v` — the edge memory follows
     the variable **every scan, powered or not**, so an edge is the variable's own
     rising/falling transition between two scans (R_TRIG/F_TRIG semantics, proven
     equivalent by test);
   - comparator: `power AND (left op right)`;
   - box: the standard block is called with the incoming power on its input, then the
     power leaving it is `Q`. `Q` does not require incoming power: a TOF keeps passing
     power for `PT` after its input drops, a counter passes power while `CV >= PV`
     (CTU) or `CV <= 0` (CTD);
   - series: the power leaving its last item; parallel: OR of its branches;
3. then the rung's coils in document order, all reading the one rung power computed in
   step 2: an output coil assigns it; a SET coil assigns TRUE and a RESET coil FALSE
   when it is TRUE, and otherwise leave the variable unchanged.

**SET and RESET of one variable in the same scan:** they are ordinary conditional
assignments, so the instruction executed later wins — a later coil in the same rung or a
coil in a later rung. Placing RESET after SET gives reset priority. Tests prove this and
the exercise `ex.lad-motor-start-stop` grades the consequence (RESET before SET loses the
Stop-priority criterion).

A rung reads what earlier rungs wrote in the same scan, like ST statements. Equivalent
ST and LAD produce the same outputs in the one runtime; tests cover SET/RESET, edges
against R_TRIG/F_TRIG, TON/TOF/TP, CTU/CTD with R/LD, and comparators on INT, REAL, TIME
and BOOL.

### Types and diagnostics

Validation runs in Rust and nothing is silently accepted. A comparison needs two operands
of one type (`INT` and `REAL` are not mixed; `BOOL` allows only `EQ`/`NE`); a literal must
parse as the expected type (finite REAL, INT in range, TIME as `T#`/`TIME#` with
descending units); a preset, reset or output must have the pin's type; contacts and
coils need BOOL variables; coils and box outputs cannot write inputs; `LAD#…`/`EDGE#…`
cannot be declared. Codes: `PLC-LAD-001` document, `002` declaration, `003` unknown
variable, `004` write target, `005` rung structure, `006` limits, `007` type, `008` box.
Every diagnostic names the rung and carries the element id; `DiagnosticDto.element`
delivers it to the editor, which marks that element and selects it when the problem is
chosen.

### Runtime power observations (`ladder_state`)

The power symbols are computed by the runtime in the same transactional scan as the
program, so they are exactly as deterministic as the outputs. The query `ladder_state`
returns, for the **latest recorded cycle**:

- `cycle` and `scanned` (FALSE in STOP or FAULT);
- `power`: one entry per element id from that scan's committed `LAD#<id>` values;
  **empty when the cycle did not scan**, because no rung executed;
- `values`: inputs as that scan read them (the scan input image; without a scan, as the
  field shows them now), outputs, internal variables and box outputs (`Q`, `ET`, `CV`) as
  the last scan committed them.

All values in one response therefore come from one cycle; the editor never combines
values of different cycles or computes power itself. It polls once per live frame and
shows the observations only while the loaded program is the document in the editor;
otherwise it says it is offline. Observation is read-only.

`LAD#…` and `EDGE#…` symbols are hidden at every boundary: the variable table, the
cause/effect view, and the workspace and Build & Load signal lists (so watch tables,
forcing and trends cannot pick them). Students see power on the rungs, not symbols.

### Editor

The editor edits the document only through typed model operations: a palette inserts
contacts (NO/NC/P/N), comparators, TON/TOF/TP/CTU/CTD boxes and output/SET/RESET coils
before, after or parallel to the selection; an element properties panel sets variables,
kinds, operands, instance names, presets, reset/load inputs and ET/CV outputs; rungs move,
duplicate, copy/paste with fresh ids and instance names, and carry comments. Every edit is
undoable (Ctrl+Z / Ctrl+Y). Arrow keys move the selection, Alt+arrows move an element in
its series, Delete removes it. Edits keep the network valid; deleting never removes more
than the selected element or rung.

## Compatibility (what LAD does not do)

- **Library documents and user functions/function blocks.** A LAD main program cannot use
  project library documents; Build & Load reports `PLC-LAD-001` ("A ladder program cannot
  use project library documents in this version."). Rungs cannot call user FBs or
  functions, and LAD documents cannot be library documents.
- **Standard blocks.** Only TON, TOF, TP, CTU and CTD are boxes; PID, R_TRIG/F_TRIG (use
  edge contacts), CTUD and arithmetic boxes are not available in LAD.
- **Retentive memory.** LAD variables and box instances are not retentive: a warm restart
  initializes them like non-RETAIN ST variables.
- **Unchanged:** download, I/O linking, forcing and online writes of LAD variables, watch
  tables, the trace, grading, hardware configuration and project persistence work as for
  ST. Version 1 projects open unchanged.
- Switching a program's language still replaces it; there is no ST↔LAD conversion.

## Alternatives considered

- **The runtime probe channel of ADR-0021.** A separate observation channel would avoid
  extra symbols but needs a new runtime facility. The synthetic symbols reuse the
  transactional scan and trace with bounded cost (at most 4096 elements per document) and
  stay hidden at the boundaries.
- **Power computed in the editor from variable values.** Forbidden: the UI would become a
  second, possibly wrong, PLC.
- **SET/RESET with a fixed priority (e.g. reset always wins).** Vendor ladders differ;
  "later instruction wins" is the plain IEC execution order and matches ST.

## Consequences

- LAD now covers the elements of introductory industrial ladder programming, compiled
  into the one runtime and observed from it.
- Five graded LAD exercises use existing scenes (see the knowledge pack).
- A document written by this version (version 2) is rejected with `PLC-LAD-001` by older
  builds that know only version 1.

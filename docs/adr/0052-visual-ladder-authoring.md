# ADR-0052: Visual ladder authoring — placement engine, derived layout and in-place editing

- Status: Accepted for 0.2.0-dev (CP9-E)
- Date: 2026-11-11
- Builds on: [ADR-0050](0050-ladder-diagram-frontend.md), [ADR-0051](0051-industrial-ladder-elements-and-power-observations.md)

## Context

After CP9-D the ladder editor covered the industrial elements, but every insertion meant
selecting an element, choosing Before/After/Parallel and clicking a palette button, and
every operand was set in a side panel. Students expect the workflow of industrial
engineering tools: drag an element onto the rung, drop it beside a contact for AND or
below it for OR, type the operand on the element with completion, type an instruction
name such as TON, edit box pins on the diagram, and work from the keyboard. The ladder
document (ADR-0051), its compiler and its runtime observations stay as they are; this ADR
is about authoring.

## Decision

### Separation

| Module                                                             | Responsibility                                                                    |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `ladModel.ts`                                                      | The ladder document and its existing pure edits (unchanged).                      |
| `lad/placement.ts`                                                 | Semantic drop targets and the one transformation that applies them.               |
| `lad/layout.ts`                                                    | Element boxes, wires, insertion points and drop zones from the tree; hit testing. |
| `lad/complete.ts`                                                  | Completion of variables, literals, instance names and instructions.               |
| `lad/useDrag.ts`                                                   | The pointer-driven drag session.                                                  |
| `lad/RungCanvas.tsx`                                               | Rendering a rung from its layout; pins as editable fields.                        |
| `lad/InlineEditor.tsx`, `InstructionPicker.tsx`, `ContextMenu.tsx` | In-place editing, instruction completion, element actions.                        |
| `LadEditor.tsx`                                                    | State, commands and keyboard dispatch.                                            |

Placement, layout and completion are pure functions of the document and are unit-tested
without a DOM. The DOM is never the source of truth: no pixel coordinate is saved.

### Semantic placement

A drop target names a place in the ladder tree, never a screen position:

- `before` / `after` element _e_ — in series with _e_ (AND);
- `below` element _e_ — a new OR branch of **exactly** _e_. Below A in `A AND B` gives
  `(A OR C) AND B`, never `A OR (C AND B)`; below an element that is already a branch of a
  parallel block adds a sibling branch right after it;
- `below` the rung's network — in parallel with all of it;
- `branch` of parallel block _p_ — one more branch of _p_;
- `start` of an empty rung; `coil` position _i_ of a rung.

`applyDrop(document, source, target)` applies a new palette element, a pasted element, or
a moved or copied existing element as one transformation, or returns why it cannot:
`coilInNetwork`, `elementInCoils`, `intoItself`, `unknown`, `noChange`. A refused drop
changes nothing. A move keeps the element's id and its configuration (variable, instance,
pins); a copy gets fresh ids and fresh instance names. After the transformation only the
rungs it changed are tidied — empty series removed, one-child blocks collapsed, a series
inside a series and a parallel block inside a parallel block flattened (AND and OR are
associative) — and the network stays a series; other rungs stay byte-identical.

### Derived layout, wiring and hit testing

The layout of a rung is computed from its tree at logical zoom-100 % coordinates: series
left to right with a wire gap between elements, parallel branches stacked with left and
right junctions, extension wires to the right junction, one coil column aligned across
all rungs, and rails. Each wire records whose power it carries (the element before it, a
junction's block, the rail). Drop zones are prioritized rectangles: element halves for
before/after, the band under an element body for `below`, the junction strips and the
band under a block, the rung head and tail, the band under the network, and coil slots.
`hitTest(layout, x, y, area)` returns a target, `wrong` (a coil over the network or an
element over the coils) or nothing. The canvas is the layout scaled by the zoom; a
pointer is converted to logical coordinates with the canvas rectangle and the zoom, so
hit testing is the same at every zoom and window size.

Online, wire and element colours come only from `ladder_state` (ADR-0051). While
dragging, the preview draws topology — an AND insertion bar or the dashed OR branch around
the exact target — and the status line names the resulting logic of the rung
(`(Button1 OR ?) AND AutoMode`); it never shows power.

### Drag and drop

Pointer events, not HTML drag-and-drop: a press becomes a drag after 5 px; palette
elements, existing elements and parallel blocks can be dragged; Ctrl copies; the target
is validated with `applyDrop` on every move; releasing applies it once (one undo step);
Esc, a release over nothing, or an invalid target change nothing and the reason is shown.
Moving near the edges of the scroll area scrolls it. The interface runs no timers
(architecture rule): autoscroll follows pointer movement.

### In-place editing and completion

Operands and box pins are fields on the diagram (contact and coil operands, comparison
IN1/IN2, box instance, PT/PV, R/LD, ET/CV). Double-click, Enter/F2, a typed character or
a click on a pin opens a combobox there. Completion comes from the program's declarations
and is filtered by what the field accepts: BOOL for contacts, writable (non-input) BOOL
for coils, the other operand's type for comparisons, TIME or INT for presets, writable
TIME/INT for ET/CV; it shows type, kind, comment and the station's I/O channel, ranks
exact, prefix, word-start and substring matches, and adds IEC literals (`T#5s` for `5`).
Instance names are proposed unique. An undeclared name is offered only as an explicit
"Create variable" whose type and kind are confirmed before it is declared and bound in
one undo step. Enter accepts, Tab accepts and moves to the next field, Esc cancels;
a literal draft is kept and the Rust compiler reports what is wrong with it.

Typing at an insertion point (a click on a series wire) or F8 opens instruction
completion: contacts, coils, comparators and the boxes the LAD compiler supports, with a
description and pins — box pins taken from the runtime's standard block signatures
(`LibraryDto.standardBlocks`). PID and user function blocks are not offered: LAD does not
compile them.

### Keyboard and context menu

F9 / F10 insert a NO / NC contact after the selection, Shift+F9 a NO branch below it,
F8 or typing an instruction, Enter/F2/typing edits, Ctrl+Space opens suggestions,
Ctrl+C / Ctrl+V copy and paste the selected element or rung, Ctrl+Z / Ctrl+Y undo and
redo, arrows select, Alt+arrows move, Delete removes, Shift+F10 or the context-menu key
opens the element menu (insert before/after, add branch, instruction, NO/NC, edit,
duplicate, delete, properties). Diagram commands apply only while focus is on the
diagram; toolbar, header and panel controls keep their own keys. A shortcut reference is
in the toolbar. The click-to-insert palette with an explicit position and the properties
panel remain as the keyboard and screen-reader fallback.

## Alternatives considered

- **React Flow or another node graph.** Excluded for LAD by AGENTS.md; a ladder is a
  series/parallel tree, and a free graph would allow layouts the compiler cannot express.
- **HTML5 drag and drop.** No control over the drag image and hit testing, inconsistent
  pointer coordinates under scaling, and hard to automate; pointer events behave the
  same for mouse, pen and test input.
- **Hit testing on DOM element rectangles.** Ties semantics to rendering details and
  cannot be unit-tested; the layout model gives the same zones the renderer draws.
- **Inferring topology from absolute coordinates.** Ambiguous for nested branches; the
  zones always resolve to one tree target, and the preview states the resulting logic.

## Consequences and limitations

- The document format (version 2) and the compiler are unchanged; documents edited this
  way are ordinary ladder documents. Version 1 documents still open.
- Rungs are reordered with their buttons or Alt+arrows, not dragged. There is no
  multiple selection. A branch is always added below its target (OR is commutative;
  move a branch to reorder it). The palette's comparator starts as GE; the picker offers
  each operator.
- Autoscroll happens while the pointer moves near an edge, not while it rests there.

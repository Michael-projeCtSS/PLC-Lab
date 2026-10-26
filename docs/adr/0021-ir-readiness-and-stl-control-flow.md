# ADR-0021: IR readiness for graphical languages and STL control flow

- Status: Accepted (IR extensions implemented; LAD/FBD/STL frontends not yet implemented)
- Date: 2026-10-26
- Input: independent audit `feedback.md`, finding H6

## Decision: IR extensions (implemented)

- `ExpressionKind::Logical { operator, operands }`: AND/OR/XOR over two or more operands, eagerly evaluated left to right with unchanged fault ordering. Wide LAD parallel branches and long ST interlock chains keep constant tree depth. The ST parser builds flat chains for repeated operators; 200-term chains compile.
- `ExpressionKind::FunctionBlockOutput { instance, parameter }`: reads the current output of a standard block instance (`Timer.Q`, `Counter.CV`). This removes the need for synthetic temporaries when an FBD block output fans out or a LAD rung reads a timer contact.
- `ExpressionKind::Conversion`: `INT_TO_REAL` (exact), `REAL_TO_INT` (round to nearest, ties to even, fault on overflow; PLC Lab generic profile choice), `TRUNC` (toward zero).
- Standard blocks TON, TOF, TP, R_TRIG, F_TRIG, CTU, CTD execute in the existing transactional working copy. Optional block inputs persist between calls. F_TRIG reports no edge on its first call (documented generic choice; editions and vendors differ).
- Operators XOR and MOD (truncated remainder; MOD by zero is a division fault). ST precedence follows IEC 61131-3: relational above equality.

## Decision: still required before LAD/FBD production editors

1. A project/document model with persistent graphical `SourceElementId`s and multi-POU assembly (`DocumentId` allocation). ST keeps ephemeral traversal IDs plus text spans.
2. A bounded runtime probe channel reporting evaluated values for selected `SourceRef`s per scan, so power flow is displayed from runtime data. React never recomputes power flow.
3. Compiler-generated symbols with a `Synthetic` origin, hidden from student variable lists, if a lowering needs temporaries beyond block outputs.

## Decision: STL control flow

The educational STL profile will support forward jumps only, inside one POU, and only when the jump structure is reducible to nested conditionals (forward, non-overlapping regions). The STL frontend lowers such regions into structured IR `If` statements. Backward jumps (loops) and computed jumps are rejected with a diagnostic. The canonical IR therefore stays structured; no CFG execution plan is introduced. Vendor STL/IL dialect differences are labeled in the language profile.

## Alternatives

- CFG IR with arbitrary jumps: would allow unbounded loops inside a scan and complicate source mapping; rejected for the educational profile.
- Desugaring CASE into a jump table: unnecessary; CASE lowers to IF chains (selector re-evaluation is pure).

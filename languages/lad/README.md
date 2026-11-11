# LAD frontend

Ladder Diagram (ADR-0050, ADR-0051). A rung/grid document (`plclab-lad`, version 2;
version 1 is read) is validated and lowered directly into the canonical PLC IR that the
one PLC runtime executes:

- variables of type BOOL, INT, REAL and TIME;
- normally open, normally closed, positive-edge and negative-edge contacts in series and
  parallel networks;
- comparators (EQ, NE, GT, GE, LT, LE) over variables and literals of one type;
- TON, TOF, TP, CTU and CTD boxes calling the runtime's standard blocks;
- output, SET and RESET coils.

Each element's outgoing power is a compiler-owned symbol `LAD#<id>` (edge memory
`EDGE#<id>`), so the runtime itself computes the power flow that the editor shows;
`is_synthetic` lets the boundaries hide these symbols. Scan order, the SET/RESET rule and
the diagnostics codes are specified in ADR-0051.

The crate depends only on `plc-types` and `plc-ir`; the runtime and the ST frontend are
dev-dependencies for the equivalence tests (`tests/elements.rs`, `tests/lad.rs`).

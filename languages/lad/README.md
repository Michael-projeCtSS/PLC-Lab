# LAD frontend

Ladder Diagram, first vertical slice (ADR-0050). A rung/grid document (`plclab-lad`,
version 1: BOOL variables, normally open and normally closed contacts in series and
parallel, output coils) is validated and lowered directly into the canonical PLC IR that
the one PLC runtime executes. The crate depends only on `plc-types` and `plc-ir`; the
runtime and the ST frontend are dev-dependencies for the equivalence tests.

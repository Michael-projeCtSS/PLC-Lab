# PLC feature matrix

Current implementation after the product rebuild. Milestone history is in the M-reports
and ADRs.

## Languages

| Language               | Status                                                   |
| ---------------------- | -------------------------------------------------------- |
| Structured Text        | Implemented (subset below), compiled to the canonical IR |
| Ladder Diagram         | Implemented (elements below), ADR-0050 to ADR-0052       |
| Function Block Diagram | Planned; IR prepared (ADR-0021)                          |
| Statement List (edu)   | Planned; control-flow design in ADR-0021                 |

## Structured Text

| Feature                                                                       | Status      |
| ----------------------------------------------------------------------------- | ----------- |
| One PROGRAM, VAR_INPUT / VAR_OUTPUT / VAR sections                            | Implemented |
| BOOL, INT (16-bit), REAL (finite f32), TIME (µs)                              | Implemented |
| Assignments, IF / ELSIF / ELSE                                                | Implemented |
| CASE with values, ranges, lists, ELSE                                         | Implemented |
| NOT, AND, OR, XOR, arithmetic, MOD, comparisons                               | Implemented |
| INT_TO_REAL, REAL_TO_INT (round half to even, overflow faults)                | Implemented |
| TIME literals (`T#1s500ms`)                                                   | Implemented |
| Standard FB instances: TON, TOF, TP, R_TRIG, F_TRIG, CTU, CTD, PID (ADR-0046) | Implemented |
| FB calls with named inputs, `=>` outputs, member reads (`T.Q`)                | Implemented |
| Comments `(* *)` and `//`                                                     | Implemented |
| DINT and other wide types, ARRAY, STRUCT                                      | Planned     |
| User functions and function blocks, library documents (ADR-0044, ADR-0045)    | Implemented |
| FOR/WHILE/REPEAT                                                              | Planned     |
| CTUD, retentive memory, multiple tasks                                        | Planned     |

## Ladder Diagram

Compiled by the LAD frontend into the canonical IR and run by the one runtime; online
power flow and values come from the runtime (`ladder_state`, ADR-0051).

| Feature                                                                      | Status        |
| ---------------------------------------------------------------------------- | ------------- |
| Rungs with series and parallel networks, multiple coils per rung             | Implemented   |
| Variables BOOL, INT, REAL, TIME (input, output, internal)                    | Implemented   |
| Normally open / normally closed contacts                                     | Implemented   |
| Positive / negative edge contacts (R_TRIG/F_TRIG semantics)                  | Implemented   |
| Output, SET and RESET coils (later instruction wins)                         | Implemented   |
| Comparators EQ, NE, GT, GE, LT, LE (type-checked in Rust)                    | Implemented   |
| TON, TOF, TP boxes (PT, ET output); CTU (PV, R, CV), CTD (PV, LD, CV)        | Implemented   |
| Power flow per element and values online from the executed scan              | Implemented   |
| Diagnostics on elements; palette, properties, undo/redo, keyboard, copy rung | Implemented   |
| Drag and drop from the palette and on the diagram; beside = AND, below = OR  | Implemented   |
| Wiring, junctions and coil column derived from the ladder tree (ADR-0052)    | Implemented   |
| Operands and box pins typed in place with type-filtered completion           | Implemented   |
| Instruction completion at an insertion point (F8), F9/F10/Shift+F9, menu     | Implemented   |
| Graded LAD exercises (5)                                                     | Implemented   |
| PID, R_TRIG/F_TRIG, CTUD and arithmetic boxes                                | Not available |
| User functions/FBs in rungs, library documents with a LAD program            | Not available |
| Retentive LAD variables, ST↔LAD conversion                                   | Not available |

## Runtime

Transactional scans (a faulting scan commits nothing), operation budget per scan,
runtime faults with source locations (division by zero, overflow), PLC modes RUN / STOP /
FAULT with warm restart and memory reset, forcing of inputs and outputs, online writes of
variables between scans and a control-loop query for PID tuning (ADR-0049), simulation-time
timers. See [runtime semantics](PLC_RUNTIME_SEMANTICS.md) and
[simulation fidelity](../SIMULATION_FIDELITY.md).

## Hardware, process, network

See [hardware model](../HARDWARE_MODEL.md), [scene model](../SCENE_MODEL.md) and
[network model](../NETWORK_MODEL.md). Modbus TCP is the only simulated protocol.

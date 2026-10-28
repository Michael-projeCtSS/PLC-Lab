# PLC feature matrix

Current implementation after the product rebuild. Milestone history is in the M-reports
and ADRs.

## Languages

| Language               | Status                                                   |
| ---------------------- | -------------------------------------------------------- |
| Structured Text        | Implemented (subset below), compiled to the canonical IR |
| Ladder Diagram         | Planned; IR prepared (ADR-0021)                          |
| Function Block Diagram | Planned; IR prepared (ADR-0021)                          |
| Statement List (edu)   | Planned; control-flow design in ADR-0021                 |

## Structured Text

| Feature                                                        | Status      |
| -------------------------------------------------------------- | ----------- |
| One PROGRAM, VAR_INPUT / VAR_OUTPUT / VAR sections             | Implemented |
| BOOL, INT (16-bit), REAL (finite f32), TIME (µs)               | Implemented |
| Assignments, IF / ELSIF / ELSE                                 | Implemented |
| CASE with values, ranges, lists, ELSE                          | Implemented |
| NOT, AND, OR, XOR, arithmetic, MOD, comparisons                | Implemented |
| INT_TO_REAL, REAL_TO_INT (round half to even, overflow faults) | Implemented |
| TIME literals (`T#1s500ms`)                                    | Implemented |
| Standard FB instances: TON, TOF, TP, R_TRIG, F_TRIG, CTU, CTD  | Implemented |
| FB calls with named inputs, `=>` outputs, member reads (`T.Q`) | Implemented |
| Comments `(* *)` and `//`                                      | Implemented |
| DINT and other wide types, ARRAY, STRUCT                       | Planned     |
| User functions and function blocks, FOR/WHILE/REPEAT           | Planned     |
| CTUD, retentive memory, multiple tasks                         | Planned     |

## Runtime

Transactional scans (a faulting scan commits nothing), operation budget per scan,
runtime faults with source locations (division by zero, overflow), PLC modes RUN / STOP /
FAULT with warm restart and memory reset, forcing of inputs and outputs, simulation-time
timers. See [runtime semantics](PLC_RUNTIME_SEMANTICS.md) and
[simulation fidelity](../SIMULATION_FIDELITY.md).

## Hardware, process, network

See [hardware model](../HARDWARE_MODEL.md), [scene model](../SCENE_MODEL.md) and
[network model](../NETWORK_MODEL.md). Modbus TCP is the only simulated protocol.

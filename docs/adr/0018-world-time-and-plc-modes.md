# ADR-0018: World time, PLC task time and PLC operating modes

- Status: Accepted for the product rebuild (supersedes the process-coupling parts of ADR-0014 and the Stop semantics of ADR-0016)
- Date: 2026-10-26
- Input: independent audit `feedback.md`, finding H3

## Context

Until M4B the simulation clock lived inside the PLC runtime and the process advanced only as a side effect of a successful PLC scan. Consequences found by the audit:

- Reset Runtime rewound time to zero while the tank kept its level, so history contained time regressions.
- "Stop" froze the whole world and kept outputs energized, which is neither a real PLC STOP nor a meaningful simulation control.
- Multiple PLCs, network delivery independent of scans, and devices that keep evolving while a PLC is stopped could not be expressed.

## Decision

Three time bases exist and are never mixed:

| Time base                                          | Owner                             | Purpose                                                                                                   |
| -------------------------------------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Host wall time                                     | desktop host (`execution_driver`) | Pacing only. Never enters simulation state.                                                               |
| World time (`simulation_core::WorldTime`)          | `lab-engine` session              | Authoritative chronology of the whole lab: physics, network, devices, scenario actions, trace timestamps. |
| PLC task time (`simulation_core::SimulationClock`) | `plc-runtime`                     | The semantic time a PLC task observes (TON/TOF/TP). Advances only by committed scans.                     |

World events are ordered by `(time, phase, participant, sequence)` with the documented phase order Stimulus < Fault < Network < Device. Continuous physics is integrated exactly between events with fixed-point arithmetic, so partitioning time never changes results.

One engine cycle (`LabSession::step_cycle`):

1. scan with the input image staged at the cycle start (unchanged transactional `Runtime::single_scan`);
2. drive outputs at the cycle start: in RUN the committed outputs (plus forces), otherwise each channel's configured STOP behavior (default: de-energized);
3. advance the world by one cycle period, interleaving scheduled actions, network events and physics;
4. sample all inputs at the cycle end and stage them for the next scan;
5. record one causal observation (ADR-0019).

PLC operating modes are explicit and separate from simulation control:

- The CPU has a mode switch (RUN/STOP) and an actual mode RUN / STOP / FAULT.
- STOP to RUN performs a warm restart: non-retentive memory and block state return to initial values, the PLC run segment increments, task time restarts. No retentive memory is modeled yet.
- In STOP and FAULT the world keeps running; outputs follow the STOP behavior.
- A runtime fault keeps the accepted transaction (no commit, no task time, no scan count) and puts the CPU in FAULT. The faulting step does not advance the world so the student can inspect the fault instant; later steps advance the world with the CPU in FAULT. An explicit memory reset is required; it never rewinds world time.
- Simulation controls (Run, Pause, Step, Stop) are separate: Stop Simulation resets the world to its initial physical state and power-cycles the CPU, keeping the downloaded program, hardware configuration and mode switch position.

Trace records carry world start/end time, PLC task time and the PLC run segment, so history never contains a silent time regression.

## Alternatives

- Keep the PLC-owned clock and add offsets: still couples physics to scans and cannot express STOP.
- Async per-component tasks: nondeterministic ordering.
- Vendor-specific startup semantics in the generic core: rejected; vendor differences belong to profiles and are currently documented, not simulated.

## Migration impact

Runtime semantics, the transactional scan and TON are unchanged. The desktop session, IPC contract and UI are rebuilt on the engine. Historical M3A/M4 behavior ("process does not advance if the scan faults") still holds for the faulting step itself.

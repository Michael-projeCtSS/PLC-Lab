# ADR-0027: Engine-evaluated commissioning checklist

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-28

## Context

PLC Lab teaches commissioning: power, hardware, network, inputs, outputs, drives and
process are verified in order before a machine is handed over. A checklist of static
checkboxes would teach nothing and could be ticked without testing. The checklist must
reflect the simulated installation, stay deterministic, never reveal a lab's injected
fault and never re-evaluate machine behavior in the user interface.

## Decision

- `lab-engine` owns the checklist (`LabSession::commissioning`, module `commissioning`).
  It is a projection of authoritative state: module views and status LEDs, field
  supplies, active diagnostics, input readings and quality, output states, terminal
  voltages, device quantities, network views and the PLC mode. It compares them with the
  drawing, which is the scene's documented wiring and device definitions. It never reads
  `active_faults`.
- Checks that need a test (operate a device, switch an output) use a bounded tracker,
  `IoTests`, updated once per cycle after the inputs are sampled: per drawn input device
  the readings seen at rest and actuated on each candidate input, per drawn output the
  command, terminal, device response and feedback values while energized and idle. A
  channel follows a device only if its reading is determined by the device's state, so
  machine reactions (a contactor pulling in while Start is held) are not mistaken for
  wiring. Feedback contacts of actuators are not candidates for input tests. The tracker
  is cleared by a repair and by a world reset, because the installation then changed.
- Item states: OK, ATTENTION, FAULT, UNTESTED, NOT_APPLICABLE; a category shows its most
  severe item. Findings are plain observations with addresses and tags; each item says
  which tool shows it.
- Transport: `commissioning(WorkspaceRequest) -> CommissioningDto`, an additive query in
  IPC contract 8, re-read by the view on every frame like the other projections.
- Commissioning labs are `ExerciseKind::Commissioning`: the troubleshooting flow
  (symptom, diagnosis, repair, verification, debrief, secrecy rules) applies through
  `ExerciseKind::is_fault_finding`. The COMMISSION navigation section lists them.

## Alternatives considered

Evaluating the checklist in React from frames (rejected: the UI must not own semantics,
and it would need the drawing and history); deriving tests from the bounded trace
(rejected: old observations fall out of the retained window); edge correlation of input
changes (rejected: the PLC reacts in the same cycle as an operator action, so
consequences were reported as wiring).

## Consequences

The checklist covers digital I/O, field supplies, hardware configuration, network status
and the generic drive model. Analog scaling checks and drive faults are shown in their
current depth and grow with the analog (P3) and drive (P4) milestones.

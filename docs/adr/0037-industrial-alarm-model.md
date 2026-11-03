# ADR-0037: A generic industrial alarm model

- Status: Accepted for 0.2.0-dev
- Date: 2026-11-03

## Context

Process alarms were boolean component quantities rendered in red (`overflowing`,
`highHigh`, `lowLow`, `dry`, `tripped`). They had no identity beyond the quantity name, no
priority, no acknowledgment and no history, so a lab could not teach the difference
between seeing an alarm, acknowledging it and removing its cause.

## Decision

- A vendor-neutral crate `alarm-core` (depends only on `simulation-core`) holds the
  model: definitions (canonical id, generic code, source, source label, category,
  priority, English message, on-delay), one lifecycle per alarm and a bounded history.
- Lifecycle: `NORMAL`; `ACTIVE_UNACKNOWLEDGED` when the condition holds past its on-delay;
  acknowledgment turns it `ACTIVE_ACKNOWLEDGED`; the condition clearing returns it to
  `NORMAL` (acknowledged) or `RETURNED_UNACKNOWLEDGED` (not yet acknowledged, until it is);
  a returned alarm whose condition holds again activates again. Acknowledging an alarm
  with nothing to acknowledge, or an unknown alarm, is refused. Acknowledgment never
  touches the condition.
- Priorities `INFORMATION`, `WARNING`, `ALARM`, `CRITICAL`; categories `PROCESS`,
  `HARDWARE`, `NETWORK`. No vendor alarm classes, latching or shelving are modeled.
- The lab engine derives the plant's alarms from the model it already owns, never from
  scene-chosen names: tank overflow, high-high and low-low level, pump dry run, motor
  overload, drive fault and drive not ready, valve position not reached within twice
  its travel time (at least 1 s), invalid analog input signals, missing field supply and
  faulted modules of the configured station, and failed Modbus connections. Ids are
  `<component>.<code>`, `io.<module>.<code>`, `io.<module>.ch<n>.<code>` and
  `modbus.<connection>.<code>`. Messages name observable symptoms only.
- Conditions are evaluated at the end of every cycle against the authoritative world
  state, in alarm id order. Only transitions are recorded (with a strictly increasing
  sequence number, the world time and the states before and after), so a held condition
  produces one activation; every transition is also written to the event log. The
  history keeps the newest 500 transitions and counts all.
- A hardware download re-derives the alarms: alarms that remain keep their state, an
  alarm removed while not normal is withdrawn with a recorded event. A world reset
  (simulation stop) restores the initial plant: every alarm normal and a new alarm
  history; the event log keeps the earlier transitions. Alarms, acknowledgments and
  history are session state and are never written to a project.
- IPC (contract 8, additive): the frame's alarm list carries every alarm that is not
  normal, most urgent first (priority, then newest activation, then id), with id,
  priority, state, condition and times. `alarms` returns that list and history pages (at
  most 200), both filtered by the backend by priority, state (of an alarm, or the state a
  transition led to) and part of the source id or label; unknown filter values are
  refused. `acknowledge_alarm` acknowledges a selection (at most 256 ids, all or none: an
  unknown alarm or one with nothing to acknowledge refuses the whole request) or, without
  a selection, every waiting alarm, and returns the frame. The view never changes an
  alarm state itself.
- The view: Basic mode shows the alarms that need attention with acknowledgment and an
  indicator that names the most urgent alarm and what to investigate; Advanced mode adds
  filters, selection, details (canonical id, code, category, source, times,
  acknowledgments) and the history. Alarm transitions are not CPU diagnostics: the CPU
  diagnostic buffer stays separate.
- A fault-finding lab may list alarm ids the operator must acknowledge
  (`troubleshooting.acknowledge`). Grading scores that duty separately (10 points, the
  verification then 40) from diagnosis, repair and isolated verification, so an
  acknowledgment can never stand in for a repair and a repair never needs one to be
  verified. Isolated verification still runs in a fresh session without live forces.

## Alternatives considered

Alarm states computed in the view (rejected: the backend owns semantics); alarm
definitions written into every scene (rejected for now: the generic derivation covers
every product scene, scene-specific alarm configuration can be added as data later);
persisting alarm states (rejected: they describe a running plant, not the project); an
unbounded history (rejected: memory and IPC size).

## Consequences

Sequence and startup diagnostics can raise alarms through the same engine once they are
modeled. Vendor profiles may later map priorities to vendor alarm classes without
changing the lifecycle.

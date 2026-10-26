# ADR-0019: Causal cycle observation and byte-bounded compact trace

- Status: Accepted for the product rebuild (supersedes the observation shape of ADR-0015)
- Date: 2026-10-26
- Input: independent audit `feedback.md`, blocker B1 and finding H1

## Context

The M4 trace stored one full snapshot per event. Each `ScanCompleted` row mixed the inputs staged for the next scan with the outputs committed by the current scan, so a row could show `LevelHigh = TRUE` and `InletValve = TRUE` for `InletValve := NOT LevelHigh`. The image the PLC actually read was never exposed. The trace was bounded only by entry count: a legal 40 KB program produced 113 MB trace pages.

## Decision

Every engine cycle produces one `CycleObservation` with five layers:

| Layer | Meaning |
| --- | --- |
| `scan_inputs` | Values the PLC read during this scan (input image) |
| `committed` | Outputs, locals and block outputs committed by this scan |
| `applied` | Output channel states and device commands applied for the interval |
| `process_after` | Physical quantities and input channel values at the end of the interval |
| `staged_next` | Inputs sampled at the end of the interval for the next scan |

Invariant: for `Output := NOT Input`, `committed.Output == NOT scan_inputs.Input` holds inside a single observation. No consumer (Timeline, cause-and-effect, grading) infers causal inputs from a neighbouring row.

Operator actions between cycles are applied at the current world instant and re-staged; they appear in the next observation's `scan_inputs` and in the event log, never by rewriting history.

Storage: signal metadata (path, label, kind, type, unit) is held once in a session `SignalRegistry`. `trace-core::SignalTrace` stores records in chunks of 64: the first record of a chunk is a keyframe, the rest store only changed values per layer. Eviction removes whole oldest chunks when an explicit byte budget (default 16 MiB per session) is exceeded and reports exact dropped counts. Discrete events (mode changes, faults, process events, diagnostics, network events, operator actions) live in a separate bounded `EventLog`.

Queries are projections computed in Rust: latest or historical cause-and-effect (bounded sections), paged variables with all images and I/O quality, trends with time windows and min/max decimation, cycle history. The frontend never mirrors history and never reconstructs PLC values.

Grading consumes a dedicated `GradingEvidence` projection built from observations in isolated assessment sessions, not the UI ring buffer.

## Alternatives

- Fix only the snapshot field: leaves cost unbounded and keeps grading coupled to UI history.
- Persistent trace files: not needed yet; the representation is file-friendly when required.

## Migration impact

IPC trace DTOs are replaced (contract v8). Tests that encoded the mixed row as correct are replaced by tests asserting the causal invariant.

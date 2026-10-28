# ADR-0014: Manual PLC/process orchestration and snapshot rendering

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [ADR-0020](0020-headless-engine-boundary.md). Kept for engineering history.

- Status: Authorized by M3A; technical-lead source review required after validation
- Date: 2026-10-14

## Context

M1C/M2A/M2B and the M2B-R1 Load intent correction are accepted. M3A authorizes a manual Tank Filling plant above the unchanged runtime. ADR-0005 reserved process models in simulation-core; this assignment prefers a separate process-simulation crate. Keep simulation-core as the accepted clock leaf and use the new crate for physical state, depending only on plc-types.

## Decision

PLC Runtime committed outputs -> desktop-session orchestration -> process-simulation model -> physical state/sensors -> desktop-session staging -> next PLC scan. Separately, a coherent runtime/process snapshot -> typed IPC -> React/SVG. The renderer never simulates the process. Only SVG geometry/labels derive from supplied normalized values; no integration, sensor derivation, actuator physics or time calculation occurs in React.

Use a private integer tank level in 0..1000 permille, fixed Rust-owned configuration and checked i128 intermediates. Truncate each actuator contribution toward zero at each step, discard fractional permille and retain no remainder/clock. Derive analog feedback through PlcReal without using it for integration. Valid u64 durations and fixed rates fit the intermediate range; impossible invariants are application failures, not student PLC faults.

One optional explicitly attached Tank scene belongs to the loaded RuntimeSessionId. Bind source-visible names case-insensitively through validated runtime descriptors and retain SymbolIds in the application adapter only. PROCESS inputs reject manual writes in Rust. Validate every binding and staging precondition before attachment replacement; failed binding preserves the existing scene/runtime. Successful Load/Unload removes the scene; failed/stale Load preserves it and the accepted intent ordering.

Keep the existing application mutex over each complete mutation/observation. After successful Runtime::single_scan, advance the plant with committed outputs for the configured cycle duration and stage its new sensors. Failed scans leave every plant field unchanged. Never alter runtime transaction semantics or add a same-scan feedback loop. An impossible process orchestration error latches the application session closed to further observations/mutations until explicit replacement or unload; no broad recovery framework is introduced.

Runtime reset calls the unchanged core reset, preserves all plant state and restages its sensors. Process reset returns tank/sensors to initial state and retains the last applied actuator flags; it changes no PLC memory/count/time/fault. Headless model reset returns its exact default state; the application preserves the last applied commands when reinitializing the attached plant. Attachment before a scan initializes actuator flags from actual committed outputs. This specifies reset ambiguity without changing PLC behavior.

IPC v5 adds optional process state and nullable input authority plus three targeted scene commands. The UI requires explicit source replacement confirmation, Compile, Load and Attach. Source/request/session protections remain. Run/Pause/Stop stay disabled; no pacing, recorder, protocol, grading, other language or arbitrary process editor is authorized.

## Alternatives and migration

Frontend physics, hidden PLC tank variables, runtime-to-process dependencies, wall time and generic plugin/graph frameworks violate scope or add unnecessary coupling. Extending the clock crate with application orchestration reverses accepted dependencies. Separate runtime engines or transaction changes are rejected. Only the desktop contract migrates v4 -> v5; no persisted format exists. Existing tests and Windows/Linux/native validation remain mandatory.

See [PROCESS_SIMULATION.md](../spec/PROCESS_SIMULATION.md).

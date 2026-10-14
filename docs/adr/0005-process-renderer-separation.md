# ADR-0005: Process model and renderer separation

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-003, FND-007

## Context

Every meaningful exercise must visibly explain its result. Headless tests and grading must observe the same authoritative process behavior as the UI.

## Decision

Place deterministic process models in simulation-core, independent of React and Tauri. Future models accept commands and virtual deltas and expose state, feedback, faults, and events. The UI renders immutable snapshots and trace data supplied through the desktop adapter. Animation can interpolate display positions only; it must not decide sensor values or control behavior.

Store exercise and scene definitions as versioned data outside React views. Planned initial devices include buttons, lamps, pumps, valves, tanks, motors, conveyors, and sensors, but only approved vertical slices may implement them.

## Alternatives

React component state as process authority prevents reliable headless grading and makes frame rate affect behavior. A premature 3D engine adds complexity and is not approved. Reject both.

## Consequences and migration

Rendering and simulation evolve independently. Process state must be explainable and observable. No process models or scenes exist to migrate.

## Sprint boundary

Reserve a Process panel with an explicit unavailable state. Do not animate a fake process.

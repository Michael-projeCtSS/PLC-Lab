# ADR-0020: Headless lab engine and data-driven industrial model

- Status: Accepted for the product rebuild (supersedes the desktop-session responsibilities of ADR-0013/0014)
- Date: 2026-10-26
- Input: independent audit `feedback.md`, findings H2 and H4

## Context

`desktop-session` had become the place where every feature landed: compilation, I/O binding by hard-coded tank tag names, process stepping, trace recording, DTO projection and the execution controller. IPC transport types (`IpcError`, `RuntimeRole`) were its domain types, and control flow depended on comparing error-code strings. The Tank was hard-coded in the session, trace, contract and UI.

## Decision

Crates and dependency direction:

```
plc-types <- plc-ir <- plc-runtime (+ simulation-core)        PLC semantics (unchanged core)
plc-types <- st-frontend                                       Structured Text frontend
lab-model                                                      versioned data definitions
lab-model <- process-simulation <- hardware-model              physics, station hardware
lab-model <- network-core                                      virtual network, Modbus TCP
trace-core                                                     generic byte-bounded storage
lab-model <- grading-core                                      evidence + rule evaluation
all of the above <- lab-engine                                 headless world, cycles, assessment
lab-model <- knowledge-core, project-model                     content packs, project files
lab-engine, knowledge-core, project-model, app-contract <- desktop-session (adapter) <- Tauri host
```

- `lab-engine` owns sessions, cycles, causal observations, faults, queries and assessment. It has typed errors (`EngineError`) and no dependency on `app-contract`, Tauri or React.
- `desktop-session` is the application adapter: request validation, DTO mapping, error-code mapping, the continuous-execution lease (ADR-0016 mechanism retained), the knowledge library and project storage.
- Scenes, components, wiring, hardware configurations, platform profiles, faults and exercises are versioned data validated by `lab-model`. Bindings resolve tags once to symbol and signal IDs. No component name appears in engine code.
- Vendor behavior is selected only through typed profile fields (addressing notation, analog representation, startup behavior). Generic code never branches on vendor names. Every profile carries a fidelity statement.

## Alternatives

- Keep `desktop-session` and add modules: preserves the coupling the audit identified.
- Trait-object component plugins: unnecessary dynamic dispatch for a closed, typed initial catalog; the enum catalog can be extended with a schema version bump.

## Migration impact

The M2–M4 desktop-session and its tests are replaced; their invariants (stale session rejection, serialized mutation, lease-based continuous execution) are re-tested against the new adapter.

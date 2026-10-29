# Architecture decision records

A superseding ADR and explicit approval are required to change a fundamental decision
(AGENTS.md section 15). Records 0013-0016 describe the pre-rebuild prototype and are kept
as history; their session, process and controller details are superseded as noted.

| ADR                                                  | Decision                                                             | Status                             |
| ---------------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------- |
| [0001](0001-desktop-stack.md)                        | Tauri 2, React, TypeScript, Rust                                     | Accepted (re-confirmed in rebuild) |
| [0002](0002-canonical-plc-ir.md)                     | One canonical PLC IR and one runtime                                 | Accepted                           |
| [0003](0003-deterministic-clock.md)                  | Deterministic SimulationClock                                        | Accepted (extended by 0018)        |
| [0004](0004-student-code-sandbox.md)                 | Bounded student-code VM                                              | Accepted                           |
| [0005](0005-process-renderer-separation.md)          | Process model independent of rendering                               | Accepted                           |
| [0006](0006-trace-first.md)                          | Trace-first architecture                                             | Accepted (refined by 0019)         |
| [0007](0007-virtual-network.md)                      | Virtual industrial network and protocol adapters                     | Accepted                           |
| [0008](0008-local-project-persistence.md)            | Versioned local project persistence                                  | Accepted (refined by 0023)         |
| [0009](0009-semantic-foundation.md)                  | Semantic foundation                                                  | Accepted                           |
| [0010](0010-transactional-alpha-runtime.md)          | Transactional runtime, leaf clock dependency                         | Accepted                           |
| [0011](0011-function-block-instance-model.md)        | Function block instances, transactional standard TON                 | Accepted                           |
| [0012](0012-alpha-st-compiler.md)                    | Bounded Structured Text frontend                                     | Accepted (language extended)       |
| [0013](0013-desktop-runtime-session.md)              | Prototype desktop runtime session                                    | Superseded by 0020, 0024           |
| [0014](0014-process-simulation-boundary.md)          | Prototype Tank process boundary                                      | Superseded by 0020                 |
| [0015](0015-deterministic-session-trace.md)          | Prototype session trace                                              | Superseded by 0019                 |
| [0016](0016-continuous-execution-controller.md)      | Prototype continuous execution controller                            | Superseded by 0018, 0024           |
| 0017                                                 | Not used (reserved for unmerged M5 work; grading is covered by 0020) | -                                  |
| [0018](0018-world-time-and-plc-modes.md)             | World time, scheduler phases, PLC RUN/STOP/FAULT, simulation control | Accepted                           |
| [0019](0019-causal-observation-and-compact-trace.md) | Causal cycle observations and byte-bounded compact trace             | Accepted                           |
| [0020](0020-headless-engine-boundary.md)             | Headless lab engine and data-driven industrial model                 | Accepted                           |
| [0021](0021-ir-readiness-and-stl-control-flow.md)    | IR readiness for LAD/FBD/STL, STL control flow                       | Accepted                           |
| [0022](0022-knowledge-pack-and-search.md)            | Versioned knowledge packs, in-memory search, no database             | Accepted                           |
| [0023](0023-project-format-and-persistence.md)       | `.plclab` format v1 and local persistence                            | Accepted                           |
| [0024](0024-desktop-adapter-and-ipc-v8.md)           | Desktop application adapter, typed errors, IPC v8                    | Accepted                           |
| [0025](0025-distribution-and-root-launcher.md)       | Release staging and root launcher                                    | Superseded by 0030                 |
| [0026](0026-launcher-window-and-project-handoff.md)  | Launcher window and typed project handoff                            | Window superseded by 0030          |
| [0027](0027-commissioning-checklist.md)              | Commissioning checklist evaluated by the engine                      | Accepted                           |
| [0028](0028-network-diagnostics.md)                  | Network diagnostics from the deterministic network model             | Accepted                           |
| [0029](0029-hardware-configuration.md)               | Hardware configuration, stable module identity, derived findings     | Accepted                           |
| [0030](0030-root-single-executable.md)               | The root `PLC Lab.exe` is the application (one executable)           | Accepted                           |

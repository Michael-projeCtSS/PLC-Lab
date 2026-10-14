# Architecture decision records

These records implement the supplied engineering contract and assigned milestones. A superseding ADR and explicit architectural approval are required to change a fundamental decision. M1A/M1B are accepted. M1C authorizes the provisional FB migration and standard TON extension in ADR-0011; M1B behavior remains unchanged.

| ADR                                           | Decision                                                                            |
| --------------------------------------------- | ----------------------------------------------------------------------------------- |
| [0001](0001-desktop-stack.md)                 | Tauri 2, React, TypeScript, Rust                                                    |
| [0002](0002-canonical-plc-ir.md)              | One canonical PLC IR and runtime                                                    |
| [0003](0003-deterministic-clock.md)           | Deterministic SimulationClock                                                       |
| [0004](0004-student-code-sandbox.md)          | Bounded student-code VM                                                             |
| [0005](0005-process-renderer-separation.md)   | Process model independent of rendering                                              |
| [0006](0006-trace-first.md)                   | Trace-first architecture                                                            |
| [0007](0007-virtual-network.md)               | Virtual industrial network and protocol adapters                                    |
| [0008](0008-local-project-persistence.md)     | Versioned local project persistence                                                 |
| [0009](0009-semantic-foundation.md)           | Accepted M1A semantic foundation; provisional FB call shape                         |
| [0010](0010-transactional-alpha-runtime.md)   | Accepted M1B owned execution, transactional scans and leaf clock dependency         |
| [0011](0011-function-block-instance-model.md) | M1C declared instances, typed bindings, transactional standard TON (review pending) |

ADRs 0001–0008 are dated 2026-10-13; ADRs 0009–0011 are dated 2026-10-14. M1C requires technical-lead code review before M2. TON code and tests are present; validation outcomes are recorded separately. Languages, other standard blocks, user FB execution, process/network simulation, trace recording and persistence remain unimplemented.

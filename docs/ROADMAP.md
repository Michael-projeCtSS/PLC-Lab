# Roadmap

## M3A assigned manual Tank slice

M1C, M2A and M2B final gates are accepted, including M2B-R1 intent ordering. M3A proves the first physical loop: PLC outputs -> integer plant -> sensors -> next scan -> authoritative 2D visualization. It remains manual, with explicit attach, ownership, reset/detach and no recording/grading/protocols. This is the physical portion of the eventual DEMO-001 chain; trace and educational grading remain separately authorized work. Stop after M3A validation for technical-lead process/application/UI source review. M3B, M2C and all continuous execution remain unauthorized. Earlier paragraphs preserve milestone history.

## M2B assigned increment

M2A validation and technical-lead source review passed. M2B implements the first explicit desktop runtime session, manual Single Scan and real variable/TON/fault/reset presentation. It stops for technical-lead application/UI source review after independent validation. M2C continuous execution and M3 process simulation require separate authorization. No automatic transition is permitted.

M1C is independently validated. M2A implements the Alpha ST compiler and Monaco compilation surface under source-review gating. It proves ST -> existing IR/runtime headlessly while desktop Compile returns diagnostics/declarations only. M2B/runtime UI and subsequent process/trace/education work remain separately authorized dependencies; no later milestone starts automatically. See [M2A report](M2A_REPORT.md).

The engineering contract controls scope and implementation order. M0/M1A/M1B are approved. M1C adds standard-FB declarations, normalized calls, transactional state and headless TON code/tests, then stops for technical-lead review. This document does not authorize M2. Actual validation outcomes are in [M1C_REPORT.md](M1C_REPORT.md).

| Milestone                   | Deliverable                                                                                                               | Dependency / evidence                                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| M0 Foundation               | ADRs, feature matrix, monorepo, real Tauri/React/Rust shell, typed information IPC, diagnostics vocabulary, quality gates | Completed and reviewed; human test tooling remains supported                                 |
| M1 PLC Runtime Foundation   | Approved IR/scan/type semantics, bounded VM, SimulationClock, source mapping, trace primitives                            | M0 review; unit/negative/budget/determinism tests                                            |
| M2 First ST Vertical Slice  | Minimal ST frontend lowering to IR and executing in the single runtime                                                    | M1; Alpha 0.1 types, assignment, expressions, IF, TON; semantic tests                        |
| M3 Process Visualization    | First deterministic process components plus authoritative UI rendering                                                    | M2; process state/feedback independent of frame rate; early visible result                   |
| M4 Trace and Education      | Monitoring/timeline, versioned exercise data, behavior-based grading                                                      | Trace infrastructure already exists from M1; this milestone expands its education/UI use     |
| M5 DEMO-001 Tank Filling    | ST → compiler → IR → VM → tank/valve/pump process → animation → trace → grading                                           | M2–M4; end-to-end tank filling, stop/target behavior, emergency override and timing evidence |
| M6 LAD                      | Dedicated rung/grid model, frontend to shared IR, online monitoring                                                       | DEMO-001 passes; ST/LAD equivalence tests                                                    |
| M7 FBD                      | Validated logical graph frontend to shared IR, graph presentation where appropriate                                       | LAD foundation; ST/LAD/FBD equivalence                                                       |
| M8 STL                      | Vendor-neutral educational STL/IL frontend and monitoring                                                                 | FBD; all-language equivalence                                                                |
| M9 Virtual Network + Modbus | Virtual kernel, links/faults/scheduler/trace, Modbus TCP then RTU adapters                                                | Runtime/clock/trace proven; protocol/fault/determinism tests; educational simulation only    |

Trace remains a prerequisite for educational execution and cannot be deferred until M4. The M1B assignment explicitly excludes recording; [ADR-0010](adr/0010-transactional-alpha-runtime.md) documents this scoped exception to ADR-0006. Source-aware snapshots/faults exist, but trace schema/recording do not. Every meaningful exercise must yield visible understandable results; headless M1B is an infrastructure dependency, not a completed lesson or ST vertical slice.

Future OPC UA, CANopen, IO-Link and other protocol work requires separate approval. Physical-device connectivity, cloud services, authentication, multiplayer, AI-generated solutions, database servers, 3D engines, and full PROFINET/EtherCAT/EtherNet/IP stacks remain unapproved.

## M1 progress

| Item                                                                         | Status                                                |
| ---------------------------------------------------------------------------- | ----------------------------------------------------- |
| M1A Alpha BOOL/INT/REAL/TIME values and conservative compatibility           | Accepted by technical-lead review                     |
| M1A language-neutral symbols/scopes/identity                                 | Accepted by technical-lead review                     |
| M1A shared diagnostics and cross-language source anchors                     | Accepted by technical-lead review                     |
| M1A typed canonical semantic IR, validator, direct-construction golden tests | Accepted; provisional FB shape superseded by ADR-0011 |
| PLC entry memory / input and output images                                   | M1B accepted by technical-lead review                 |
| SimulationClock checked runtime behavior                                     | M1B accepted by technical-lead review                 |
| One cyclic task / transactional scan / Ready-Faulted state                   | M1B accepted by technical-lead review                 |
| Owned execution plan / checked operators / per-scan budget                   | M1B accepted by technical-lead review                 |
| Standard-FB definitions, instances, typed bindings and validation            | M1C code and validation tests present; review pending |
| Standard TON, transactional instance state and Q/ET snapshots                | M1C code present; runtime validation pending          |
| Other timers/counters, user FB/function execution                            | Not implemented                                       |
| Trace schema / recording                                                     | Not implemented                                       |

M1 is not complete: M1C validation/review and trace infrastructure remain required dependencies. M1C preserves accepted [ADR-0010](adr/0010-transactional-alpha-runtime.md) and records the authorized provisional-call migration in [ADR-0011](adr/0011-function-block-instance-model.md). No language frontend, other block execution, process/network model, protocol, recorder, grading, or UI execution has been added. Run/Pause/Stop/Single Scan remain disabled; the human checklist is unchanged. Review is required before the first ST frontend can be authorized; do not begin M2.

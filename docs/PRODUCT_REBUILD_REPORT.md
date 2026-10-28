# Product rebuild report

Branch `claude/product-rebuild-v1`, based on the M4B prototype (`db5e357`, report
`ed5ea38`) and the independent audit [`docs/audit/feedback.md`](audit/feedback.md).
Validation evidence is in the [CI and native validation](#ci) section; open items are
listed honestly at the end.

## Final architecture

```
ST source -> st-frontend -> canonical IR -> plc-runtime (one runtime)
lab-engine: WorldTime scheduler + PLC participant + hardware-model + process-simulation + network-core
         -> causal cycle observations -> trace-core (byte-bounded) -> projections
         -> grading evidence -> grading-core
knowledge-core (packs, search)   project-model (.plclab, progress)
desktop-session (application adapter, typed AppError) -> app-contract (IPC v8 DTOs)
Tauri host (commands, trusted paths, pacing) -> packages/ipc (strict validators) -> packages/ui
apps/launcher -> root PLC Lab.exe
```

Details: [product architecture](PRODUCT_ARCHITECTURE.md), ADR-0018..0025.

## Audit findings resolution matrix

| Finding | Topic                                                        | Resolution | Evidence                                                                                                                                                               |
| ------- | ------------------------------------------------------------ | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1      | Observation mixed next inputs with current outputs           | RESOLVED   | `CycleObservation` (scan inputs, committed, applied, process, staged next); invariant test `causal_invariant_holds_inside_every_single_observation`; UI cause & effect |
| H1      | Trace bounded by count, not size                             | RESOLVED   | `trace-core` keyframe/delta store with byte budget, bounded event log; test with 1 500 variables                                                                       |
| H2      | `desktop-session` god crate, string-coded control flow       | RESOLVED   | Headless `lab-engine`; adapter with typed `AppError`; typed `TickOutcome`; architecture tests on the cargo graph                                                       |
| H3      | Time owned by the PLC, process advanced as scan side effect  | RESOLVED   | `WorldTime` scheduler, phases, PLC modes; world runs in STOP/FAULT; ADR-0018                                                                                           |
| H4      | Tank hard-coded                                              | RESOLVED   | Data-driven scenes, components, wiring, profiles; four scenes in the pack                                                                                              |
| H5      | Infrastructure-centric workflow, Stop/Pause semantics        | RESOLVED   | Basic Build & Load; separate simulation and PLC mode controls; Stop Simulation resets the plant                                                                        |
| H6      | IR/runtime not ready for LAD/FBD                             | MITIGATED  | ADR-0021, IR node types for logic/FB outputs/conversions; editors not built                                                                                            |
| M1      | Tank model depended on step size, silent overflow truncation | RESOLVED   | Exact fixed-point volumes, overflow/dry-run/HH/LL events, dt-invariance test                                                                                           |
| M2      | Reset/attach inconsistencies                                 | RESOLVED   | Scenes are part of the workspace; world reset keeps program/config/switch; memory reset defined                                                                        |
| M3      | Continuous error path asymmetric                             | RESOLVED   | One `step_cycle` for manual and paced runs; PLC fault pauses; internal error fails closed                                                                              |
| M4      | Load intent ordered by arrival                               | RESOLVED   | Client revisions; older revisions return `SUPERSEDED` (tested)                                                                                                         |
| M5      | IPC schema duplicated by hand in TS validators               | MITIGATED  | Validators are typed against generated declarations (drift fails compilation); still hand-written                                                                      |
| M6      | Frontend trace mirror and timeline did not scale             | RESOLVED   | Backend trend projections with decimation, paged events, virtualized list                                                                                              |
| M7      | Frontend duplicated scene interface rules                    | RESOLVED   | Binding and interface checks only in Rust (`BINDING_FAILED`, `INTERFACE_MISMATCH`)                                                                                     |
| M8      | Repository hygiene and governance                            | RESOLVED   | Placeholders removed, audit moved to `docs/audit`, root kept to entry points                                                                                           |
| M9      | Documentation drift                                          | RESOLVED   | Current-state docs; prototype specs marked historical                                                                                                                  |
| M10     | CI and supply-chain gaps                                     | RESOLVED   | cargo-deny, npm production audit (found and fixed a critical DOMPurify issue), exact Node pin, release build and smoke, required artifacts                             |
| M11     | Regex architecture tests                                     | RESOLVED   | `boundaries.test.mjs` uses `cargo metadata`; remaining source scans are security/time checks                                                                           |
| M12     | ST subset too small for a curriculum                         | RESOLVED   | ELSIF, CASE, XOR, MOD, conversions, FB outputs, TOF, TP, R_TRIG, F_TRIG, CTU, CTD                                                                                      |
| L1      | REAL `/0` reported as non-finite                             | DEFERRED   | Unchanged; documented                                                                                                                                                  |
| L2      | Relational and equality precedence                           | RESOLVED   | IEC precedence in the parser (equality below comparison)                                                                                                               |
| L3      | Trace sequence saturation machinery                          | RESOLVED   | Trace store rewritten                                                                                                                                                  |
| L4      | `app-contract` contained a compile service                   | RESOLVED   | Contract is DTO-only (architecture test)                                                                                                                               |
| L5      | Process codes on the transport error channel                 | RESOLVED   | Process events are typed engine events                                                                                                                                 |
| L6      | Poisoned mutex                                               | MITIGATED  | Fails closed with an explicit restart message (`APP-UNAVAILABLE`)                                                                                                      |
| L7      | Operation budget equals validation node limit                | DEFERRED   | Unchanged; relevant once loops exist                                                                                                                                   |
| L8      | Stale capability description                                 | RESOLVED   | Capability text updated; permissions unchanged                                                                                                                         |
| L9      | TIME literals without fractions/underscores                  | DEFERRED   | Unchanged                                                                                                                                                              |
| L10     | Lone UTF-16 surrogate in editor                              | DEFERRED   | Not investigated                                                                                                                                                       |

## Preserved foundations

One canonical IR and one runtime; transactional scans with rollback; BOOL/INT16/finite
REAL/TIME (u64 µs) semantics; TON on simulation time; ordered maps for determinism; Rust
authority over all semantics; `SourceRef` source mapping; `unsafe_code = "forbid"`;
minimal Tauri capabilities and strict CSP; bounded student-code execution.

## Replaced prototype architecture

`RuntimeSessionService` (load/attach/scan/trace/controller in one crate), the Tank process
slice, the session trace, IPC v1-v7, the `LabShell` UI and its hooks, the regex
architecture tests and the v7 native smoke modules were removed and replaced as described
here. No compatibility shims remain for removed contracts.

## World and time model

Host wall time paces only; `WorldTime` orders all events (time, phase, participant,
sequence; phases Stimulus < Fault < Network < Device); PLC task time advances with
committed scans. PLC RUN/STOP/FAULT with mode switch, warm restart (segment counter) and
memory reset; the world keeps running in STOP/FAULT. Simulation Run/Pause/Step/Stop is
separate from the PLC mode.

## Engine boundary

`lab-engine` is headless: it owns cycle execution, observations, trace, faults and
assessment, exposes typed errors and has no dependency on transport, knowledge, projects
or Tauri.

## Causal observation model

Each cycle records `scan_inputs`, `committed`, `applied`, `process_after`, `staged_next`,
header (cycle, start/end world time, segment, outcome) and fault. Cause & effect, variables
(scan value, committed, next input, source, quality, force), trends and grading evidence
are projections of it.

## Trace scaling

Stable signal identifiers, keyframes every 64 records with per-layer deltas, 16 MiB
default budget with oldest-first eviction and reporting, bounded event log, decimated trend
queries, paged events.

## Component and scene architecture

Ten component types with typed ports and quantities, couplings, installed hardware and
wiring, reference station, network installation, recommended interface and HMI layout,
all validated data. See [scene model](SCENE_MODEL.md).

## Hardware architecture

Generic module catalog, configured vs installed modules, field power, channel modes,
analog bands per profile, wire break, qualities, LEDs, diagnostic buffer, multimeter,
profile-specific addressing. See [hardware model](HARDWARE_MODEL.md).

## Platform profiles

Nine data-driven simulation profiles (Siemens S7-1200/S7-1500, Rockwell
CompactLogix/ControlLogix, Beckhoff TwinCAT 3, Schneider M200/M580 as approximated;
Mitsubishi FX5 and OMRON NX/NJ as conceptual) plus `generic.iec`; eight vendors and sixteen
families in the knowledge base. See [vendor profiles](VENDOR_PROFILES.md).

## Network simulation

Deterministic Ethernet segment with subnet/gateway/ARP/conflict rules and a behavioral
Modbus TCP client/server (FC03/FC04, exceptions, unit id, word order, timeouts). Other
protocols are knowledge only. See [network model](NETWORK_MODEL.md).

## Troubleshooting model

Hidden physical faults and seeded configuration errors, evidence recording, diagnosis
attempts, repairs, isolated verification with remaining faults, 30/20/50 scoring; six
labs. See [troubleshooting model](TROUBLESHOOTING_MODEL.md).

## Knowledge-pack architecture and source policy

Pack format 1 with schemas, strict loading, cross-validation, isolation of bad files,
in-memory search, deterministic ZIP with verification. Original text; source records with
verified/unverified status; simulation approximations labeled. See
[knowledge pack](KNOWLEDGE_PACK.md).

## Learning model

20 lessons with concept, diagram, example, mistake, question, simulation or exercise link,
summary with troubleshooting note and checkpoint; 9 practice exercises; Basic and Advanced
modes. See [learning model](LEARNING_MODEL.md).

## Grading model

Isolated scenario runs, evidence layers from causal observations, behavioral rules,
deterministic reports; every shipped exercise proven solvable and not solved by its
starter. See [grading model](GRADING_MODEL.md).

## Persistence

`.plclab` v1 with v0 migration and future-version rejection, atomic writes, autosave of
source and watch list, Save As, import, export, progress file. See
[project format](PROJECT_FORMAT.md).

## UX redesign

New shell (Home, Learn, Practice, Troubleshoot, Free Lab, Knowledge, Progress, About),
engineering workspace with project tree, five center tabs, inspector and monitoring dock,
dark industrial design system with semantic status colors, HMI process symbols, waveform
view, safe Markdown, developer details only on About.

## Security

No `unsafe` code; student code only in the bounded VM; web view limited to event
listen/unlisten; no shell/filesystem/HTTP/dialog plugins; CSP without `unsafe-eval`;
knowledge loading without symlinks and with size limits; Markdown rendered without HTML;
release-only builds contain no repository paths (architecture test); npm production audit
and cargo-deny in CI.

## Performance

Engine cycle work is proportional to signals in the scene and program; trace memory is
bounded by budget; frames, variables and trends are fetched with at most one request in
flight and bounded page sizes; Monaco is loaded lazily.

## CI

Windows lane: format, typecheck, ESLint, clippy, architecture, frontend and Rust tests,
production build, release-profile Rust tests, knowledge archive verification, debug native
smoke, release build and staging, release native smoke, root-launcher smoke, publication of
the validated release. Linux lane: format, clippy and debug/release tests for every crate
except the WebView host, frontend tests, architecture tests, knowledge verification, npm
production audit and cargo-deny. Actions are pinned by SHA, permissions are read-only, Node
is pinned exactly and evidence uploads fail when files are missing.

## Native validation

- Local (developer machine, Windows 11 with Smart App Control): the debug native smoke
  test passed against the real WebView2 application on 2026-10-27 (11 steps: navigation,
  contract, knowledge discovery, Monaco editing, Build & Load, operator input, cause &
  effect, isolated grading, troubleshooting, search). After a release cache clean, Smart
  App Control blocked freshly compiled build scripts and proc-macro DLLs (os error 4551),
  so local release builds, local root-launcher validation and some freshly linked test
  binaries are **BLOCKED** locally. Windows security was not changed.
- Hosted Windows CI is the authoritative native and release validation; see the
  [handoff](REBUILD_HANDOFF.md) for the run status.

## Known simulation fidelity limits

Fixed cycle time and single task; no retentive memory; simplified physics; analog bands and
diagnostic texts approximated; Modbus TCP is behavioral only; no drives, conveyors or
cylinders yet; vendor firmware and instruction sets not emulated. See
[simulation fidelity](SIMULATION_FIDELITY.md).

## Legacy cleanup

- Removed: v7 contract modules and tests (`compilation`, `process`, `session`, `trace`),
  the prototype adapter (`execution`, `process`, `trace` and their tests), the `LabShell`
  UI, its hooks, fixtures and tests, the v7 IPC client modules, the regex architecture
  tests, the v7 native smoke modules, `RUN_PLC_LAB_RELEASE.bat`, the Sprint 0 `exercises/`
  and `scenes/` placeholders, unused dependencies (`simulation-core` in desktop-session and
  st-frontend dev, `serde_json` in lab-engine dev).
- Retained intentionally: milestone reports and prototype ADRs/specs (marked historical),
  `.plclab` v0 migration (supported format), the audit report (`docs/audit`).
- Remaining technical debt: hand-written TypeScript validators (typed against generated
  declarations), ts-rs warnings about `deny_unknown_fields` on tagged enums (harmless),
  L1/L7/L9/L10 from the audit.

## Distribution / launcher

| Item                     | Path                                                                                    |
| ------------------------ | --------------------------------------------------------------------------------------- |
| Root launcher            | `C:/Users/mjgom/Documents/PLC_Student/PLC Lab.exe` (generated)                          |
| BAT fallback             | `C:/Users/mjgom/Documents/PLC_Student/RUN_PLC_LAB.bat`                                  |
| Development launcher     | `C:/Users/mjgom/Documents/PLC_Student/RUN_PLC_LAB_DEV.bat`                              |
| Canonical staged release | `C:/Users/mjgom/Documents/PLC_Student/releases/current/` (generated)                    |
| Knowledge ZIP            | `C:/Users/mjgom/Documents/PLC_Student/releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip` |

ROOT LAUNCHER (local): **BLOCKED** by Smart App Control on the development machine; the
launcher smoke runs on hosted Windows CI (`npm run test:launcher`).

## Future work

See the [roadmap](ROADMAP.md): signed distribution, drive/conveyor/cylinder components,
ST growth, LAD/FBD/STL editors on the shared IR, replay, more protocol adapters,
localization.

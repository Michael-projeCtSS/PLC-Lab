# Product rebuild report

Branch `claude/product-rebuild-v1`, based on the M4B prototype (`db5e357`, report
`ed5ea38`) and the independent audit [`docs/audit/feedback.md`](audit/feedback.md).
Every statement below is backed by a named test, a CI stage or a file. The final CI run is
recorded in [Final validation](#final-validation). The validated source is
`fb57c7bb8746ed908c57b9fe85ef6decc4a0b4ec`; an earlier green candidate was rejected by a release-candidate audit (see
[Release candidate audit](#release-candidate-audit)).

## Final architecture

```
ST source -> st-frontend -> canonical IR -> plc-runtime (one runtime)
lab-engine: WorldTime scheduler + PLC participant + hardware-model + process-simulation + network-core
         -> causal cycle observations -> trace-core (byte-bounded) -> projections
         -> grading evidence -> grading-core
knowledge-core (packs, search)   project-model (.plclab, progress)
desktop-session (application adapter, typed AppError) -> app-contract (IPC v8 DTOs)
Tauri host (commands, trusted paths, pacing) -> packages/ipc (strict validators) -> packages/ui
apps/launcher -> root PLC Lab.exe -> releases/current/PLC Lab Desktop.exe
```

Details: [product architecture](PRODUCT_ARCHITECTURE.md), ADR-0018..0025.

## Preserved foundations

One canonical IR and one runtime; transactional scans with rollback; BOOL/INT16/finite
REAL/TIME (u64 µs) semantics; TON and the other standard blocks on simulation time;
ordered maps for determinism; Rust authority over all semantics; `SourceRef` source
mapping; `unsafe_code = "forbid"`; minimal Tauri capabilities and strict CSP; bounded
student-code execution (operation budget per scan). The prototype specifications that
describe these rules carry a "prototype-era specification" banner.

## Replaced prototype architecture

`RuntimeSessionService` (load/attach/scan/trace/controller in one crate), the Tank process
slice, the session trace, IPC v1-v7, the `LabShell` UI and its hooks, the regex
architecture tests and the v7 native smoke modules were removed and replaced as described
here. No compatibility shims remain for removed contracts.

## Audit findings resolution matrix

RESOLVED means implemented and proven by the named test or CI stage. MITIGATED means the
risk is reduced but the finding's full remedy is not in place. DEFERRED means unchanged.

| Finding | Topic                                                        | Resolution | Evidence                                                                                                                                                        |
| ------- | ------------------------------------------------------------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1      | Observation mixed next inputs with current outputs           | RESOLVED   | `CycleObservation` layers; `causal_invariant_holds_inside_every_single_observation`; `tests/conformance/tests/grading_causality.rs` (see Causal observation)    |
| H1      | Trace bounded by count, not size                             | RESOLVED   | `trace-core` keyframe/delta store with byte budget; engine test with 1 500 variables; adapter scale test with measured sizes (see Bounded trace)                |
| H2      | `desktop-session` god crate, string-coded control flow       | RESOLVED   | Headless `lab-engine`; adapter with typed `AppError`; typed `TickOutcome`; `tests/architecture/boundaries.test.mjs` on the cargo graph                          |
| H3      | Time owned by the PLC, process advanced as scan side effect  | RESOLVED   | `WorldTime` scheduler with phases; `plc_stop_de_energizes_outputs_while_the_world_keeps_running`; ADR-0018                                                      |
| H4      | Tank hard-coded                                              | RESOLVED   | Data-driven scenes, components, wiring and profiles; five scenes; energization from port metadata (`output_energization_follows_port_direction_not_port_names`) |
| H5      | Infrastructure-centric workflow, Stop/Pause semantics        | RESOLVED   | Basic Build & Load; separate simulation and PLC mode controls; native smoke builds, loads, runs and grades through the UI                                       |
| H6      | IR/runtime not ready for LAD/FBD                             | MITIGATED  | ADR-0021 and IR node types for logic, FB outputs and conversions; no LAD/FBD frontend exists, so equivalence is unproven                                        |
| M1      | Tank model depended on step size, silent overflow truncation | RESOLVED   | Exact fixed-point volumes, overflow/dry-run/HH/LL events, dt-invariance test in `process-simulation`                                                            |
| M2      | Reset/attach inconsistencies                                 | RESOLVED   | Scenes belong to the workspace; `world_reset_restores_the_plant_and_keeps_program_and_signal_ids`                                                               |
| M3      | Continuous error path asymmetric                             | RESOLVED   | One `step_cycle` for manual and paced runs; `faulted_scan_commits_nothing_freezes_that_step_and_requires_reset`                                                 |
| M4      | Load intent ordered by arrival                               | RESOLVED   | Client revisions; older revisions return `SUPERSEDED` (`practice_workflow_build_load_run_and_grade`)                                                            |
| M5      | IPC schema duplicated by hand in TS validators               | MITIGATED  | Validators are typed against the generated declarations, so drift fails compilation; they are still hand-written                                                |
| M6      | Frontend trace mirror and timeline did not scale             | RESOLVED   | Backend trend decimation, paged events and variables; the UI keeps only the latest frame (see Performance)                                                      |
| M7      | Frontend duplicated scene interface rules                    | RESOLVED   | Binding and interface checks only in Rust (`BINDING_FAILED`, `INTERFACE_MISMATCH`)                                                                              |
| M8      | Repository hygiene and governance                            | RESOLVED   | Placeholders and dead code removed (see Cleanup); root holds only entry points; architecture test for root BAT paths                                            |
| M9      | Documentation drift                                          | RESOLVED   | Current-state README and model documents; prototype specifications and milestone reports marked historical                                                      |
| M10     | CI and supply-chain gaps                                     | RESOLVED   | cargo-deny, npm production audit, exact Node pin, release build, staging and native/launcher smoke on every push (see CI)                                       |
| M11     | Regex architecture tests                                     | RESOLVED   | `boundaries.test.mjs` uses `cargo metadata`; remaining source scans are security and time checks                                                                |
| M12     | ST subset too small for a curriculum                         | RESOLVED   | ELSIF, CASE, XOR, MOD, conversions, FB outputs, TOF, TP, R_TRIG, F_TRIG, CTU, CTD; every practice exercise solved by a reference solution in conformance        |
| L1      | REAL `/0` reported as non-finite                             | DEFERRED   | Behavior unchanged; documented                                                                                                                                  |
| L2      | Relational and equality precedence                           | RESOLVED   | IEC precedence in the parser (equality below comparison), parser tests                                                                                          |
| L3      | Trace sequence saturation machinery                          | RESOLVED   | Trace store rewritten; saturation code removed                                                                                                                  |
| L4      | `app-contract` contained a compile service                   | RESOLVED   | Contract is DTO-only (architecture test)                                                                                                                        |
| L5      | Process codes on the transport error channel                 | RESOLVED   | Process events are typed engine events                                                                                                                          |
| L6      | Poisoned mutex                                               | MITIGATED  | Fails closed with an explicit restart message (`APP-UNAVAILABLE`); the state is not recovered                                                                   |
| L7      | Operation budget equals validation node limit                | DEFERRED   | Unchanged; becomes relevant once loops exist                                                                                                                    |
| L8      | Stale capability description                                 | RESOLVED   | Capability text updated; permissions unchanged                                                                                                                  |
| L9      | TIME literals without fractions/underscores                  | DEFERRED   | Unchanged                                                                                                                                                       |
| L10     | Lone UTF-16 surrogate in editor                              | DEFERRED   | Not investigated                                                                                                                                                |

## Release candidate audit

The previous candidate `c2dfed3b2773f14b0ebf51a7fd85547c6c87bba3` passed both CI lanes
(run 40695358656) and was then **rejected** by a read-only release-candidate audit. Green
CI was not enough: the audit found a defect no test looked for.

**B1 (blocker): troubleshooting references revealed every root cause.** The Investigation
panel listed each lab's knowledge references from the moment the lab opened, and those
included the lab's own troubleshooting guide, whose title names the fault: "Stop button
contact replaced by a normally open block", "Missing load (field) supply on an I/O
module", "Sensor output type differs from the program's assumption", "Variable frequency
drive left in local control", "Analog channel configured for 0-20 mA instead of 4-20 mA",
"Device address outside the PLC subnet", "Modbus register address offset by one or
wrong". The correct repair label was visible as well. The existing smoke test only
checked that an internal fault type string was absent.

Fix (data and API model, not CSS):

- troubleshooting labs carry `learningRefs` (general concepts, shown as "Helpful
  concepts") and `debriefRefs` (the specific guide); exercise-level `knowledgeRefs` must
  be empty for labs;
- pack loading rejects a lab whose learning references are troubleshooting guides or
  relate to a guide about its fault, and the general articles no longer link to those
  guides;
- the debrief, including its references, is part of a passed check result only;
- repair options are sent only after a correct diagnosis, and the backend refuses a
  repair before one;
- tests: `every_troubleshooting_lab_hides_its_cause_until_verified` (all seven labs,
  against the guide titles, debrief text, correct repair labels and internal fault types,
  then diagnosis, repair or configuration download, verification, debrief with the guide,
  reopen), the conformance and architecture content rules, two UI tests, and the native
  smoke (no guide title, repair label or debrief text before the diagnosis; the guide link
  after verification).

Other findings fixed in the same corrective pass:

| Finding                                            | Change                                                                                                                                                                                    | Evidence                                                                                              |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| M1 "Alpha" in user-visible diagnostics             | Compiler, IR, runtime and identifier messages use "PLC Lab Structured Text" or state the rule                                                                                             | architecture test `user-visible diagnostics use product terminology, not milestone names`             |
| M2 forcing without a safety warning                | Lesson 19's forcing example states the hazard (unexpected motion), requires a made-safe machine and site procedure, removal of forces, and that simulation says nothing about real safety | lesson content                                                                                        |
| M3 PLC RUN/STOP vs simulation controls unexplained | Every button has an accessible explanation matching the implementation; lesson 2 explains the difference before the first exercise                                                        | UI test `PLC and simulation controls explain what they do`, native smoke step                         |
| M4 two instances could overwrite each other's data | `tauri-plugin-single-instance`: a second launch hands over and exits, the running window comes to the front                                                                               | architecture test on the host, native smoke `a second launch activates the running instance`          |
| L1 corrupt progress silently replaced              | `progress.json` that cannot be used is renamed to `.unreadable-<seconds>`, a notice is shown; saving is disabled if the rename fails                                                      | `corrupt_progress_is_kept_aside_and_never_overwritten`, `progress_from_a_newer_format_is_kept_aside`  |
| L3 applied layer chose quantities by name          | Commanded quantity declared per sink port in the component kind                                                                                                                           | `commanded_quantities_come_from_sink_port_direction`, `applied_layer_follows_port_metadata_not_names` |
| L5 four-step cause and effect in content           | Lesson 3 and the scan-cycle article describe the five steps                                                                                                                               | content                                                                                               |
| L6 exercise order by difficulty and duration       | `curriculumOrder` per exercise; lists follow it                                                                                                                                           | conformance (unique order per kind)                                                                   |
| L7 rapid commands not serialized                   | One use case in flight, controls disabled while pending, numbered frame requests (older responses dropped), re-read after every command                                                   | UI tests for Run→Pause, Run→Stop, double Step, PLC RUN→STOP, failure, stale read; mutation-checked    |

Deferred intentionally (low): L2 lesson checkpoint scoring in the frontend, L4 trace
memory accounting is an estimate, L8 launcher message box needs PowerShell.

## World time

Host wall time only paces; `WorldTime` orders every event by time, phase, participant and
sequence (phases Stimulus < Fault < Network < Device). PLC task time advances with
committed scans. The same program, inputs, faults, seed and timing produce the same
observations (`identical_sessions_produce_identical_observations`,
`grading_runs_are_deterministic`).

## PLC modes

PLC RUN/STOP/FAULT with a mode switch, warm restart (segment counter) and memory reset.
In STOP outputs are de-energized while the world keeps running; a faulted scan commits
nothing and needs a reset. Simulation Run/Pause/Step/Stop is separate from the PLC mode;
Stop Simulation restores the plant and keeps the program.

## Causal observation

Each cycle records `scan_inputs`, `committed`, `applied` (commands as received by field
devices after module power and forces), `process_after`, `staged_next`, a header (cycle,
start/end world time, segment, outcome) and the fault. Cause & effect (five steps),
Variables (Scan input, Committed, Next input), trends and grading evidence are
projections of it.

Single-cycle proof (`single_cycle_proof_scan_input_committed_output_and_next_input`,
program `Lamp1 := NOT Button1`, S1 pressed at 2000 ms and released at 2050 ms):

| Cycle 20, 2 000 000..2 100 000 µs | Value |
| --------------------------------- | ----- |
| Scan input `Button1`              | TRUE  |
| Committed output `Lamp1`          | FALSE |
| Next staged input `Button1`       | FALSE |

Grading pairs each committed output with the input image of the same cycle: the rule
"lamp = NOT button" passes over every scanned cycle for the inverter and fails for
`Lamp1 := Button1` (`inverter_output_is_the_negation_of_the_same_cycles_scan_input`,
`non_inverting_program_fails_the_same_causal_rule`).

Process chain (`scan_input_command_process_effect_and_next_sensor_value`, motor station,
Start pressed at 500 ms): cycle 5 scans `StartPB` = TRUE and `MotorFeedback` = FALSE,
commits `MotorContactor` = TRUE, the process shows `motor.command` = TRUE, and the staged
next input has `MotorFeedback` = TRUE; `RunLamp` follows in cycle 6.

## Bounded trace

Stable signal identifiers, keyframes every 64 records with per-layer deltas, a 16 MiB
default budget with oldest-first eviction and reporting, a bounded event log, decimated
trend queries and paged events. Measured through the application adapter
(`long_runs_of_large_programs_keep_every_response_bounded`: 3 000 cycles of a program with
1 000 INT variables updated every scan, 1 007 variables in total):

| Measurement                    | Result                                        |
| ------------------------------ | --------------------------------------------- |
| Trace bytes / configured limit | 16 664 672 / 16 777 216                       |
| Cycles retained / evicted      | 824 / 2 176 (evicted cycles reported missing) |
| Largest live frame             | 3 817 bytes (asserted < 64 KiB)               |
| Variables page, 100 of 1 007   | 16 247 bytes (asserted < 64 KiB)              |
| Trend, 200-point limit         | 345 bytes                                     |
| Event page                     | 1 857 bytes, 14 events                        |

The run completed all 3 000 cycles within the configured limits.

## Engine boundary

`lab-engine` is headless: it owns cycle execution, observations, trace, faults and
assessment, exposes typed errors and has no dependency on transport, knowledge, projects
or Tauri (architecture test on the cargo graph).

## Scene and component model

Eleven component types with typed ports and quantities, couplings, installed hardware and
wiring, reference station, network installation, recommended interface and HMI layout,
all validated data. See [scene model](SCENE_MODEL.md).

## Hardware model

Generic module catalog, configured versus installed modules, field power, channel modes,
analog bands per profile, wire break, qualities, LEDs, diagnostic buffer, multimeter and
profile-specific addressing. Output energization follows port direction: the hardware
model de-energizes every wired digital sink and energizes those whose output channel is ON
on a powered, correctly configured module; which ports are sinks comes from the component
kind's port metadata, never from port names. Tests in
`crates/hardware-model/tests/station.rs`:
`output_energization_follows_port_direction_not_port_names` (valve solenoid, contactor
coil, drive run terminal and pump motor are energized; source ports such as
`prox.output` or `drive.running` never are; one channel switches exactly its device) and
`wiring_against_port_direction_or_to_unknown_ports_is_rejected` (source ports on output
channels, sink ports on input channels, unknown ports and components, unknown component
types and unknown parameters are rejected). See [hardware model](HARDWARE_MODEL.md).

## VFD model

`variableFrequencyDrive`: run terminal (digital sink); relay outputs ready, running and
remote (dry contacts); linear ramp (`rampMs`); approximate current; local (keypad) control
as the fault `driveLocalControl`, in which the drive ignores the run terminal. Ready is
always TRUE because drive trips are not simulated. Switching back to Remote with the run
terminal ON starts the drive at once. Tests:
`drive_ramps_on_its_run_terminal_and_ignores_it_in_local_control` (process-simulation),
`drive_left_in_local_control_lab_from_symptom_to_debrief` (adapter). See
[simulation fidelity](SIMULATION_FIDELITY.md).

## Network and Modbus

Deterministic Ethernet segment with subnet, gateway, ARP and address-conflict rules and a
behavioral Modbus TCP client/server (FC03/FC04, exceptions 01/02/0B, unit identifier,
word order, timeouts). Adapter workflow
`modbus_commissioning_success_wrong_register_wrong_address_and_correction`: success with
register 0 equal to the tank level and a ping reply; start address 1 keeps status OK with
a wrong value; start address 2 returns exception 02; an absent address times out (ping
timeout); another subnet reports no route (ping no route); network events appear in the
event log; downloading the documented station restores OK and grading follows the
downloaded station. Unit identifiers: directly addressed devices ignore them, gateway-style
devices answer exception 0B (`unit_identifier_is_checked_only_by_devices_that_route_by_unit`).
Other protocols are knowledge only. See [network model](NETWORK_MODEL.md).

## Troubleshooting

Hidden physical faults and seeded configuration errors, evidence recording, diagnosis
attempts, repairs, isolated verification with remaining faults, 30/20/50 scoring and a
debrief returned only with a passed verification. Final audit of the seven labs; each
debrief statement was checked against the simulated behavior, and two statements were
corrected in this pass (`ts.outputs-dead`: the run lamp is dark because it follows the
contactor feedback; `ts.remote-level-wrong`: the visible evidence is the first raw word,
not register 0):

| Lab                        | Symptom                         | Evidence the simulation provides                                              | Repair                | Verification               |
| -------------------------- | ------------------------------- | ----------------------------------------------------------------------------- | --------------------- | -------------------------- |
| `ts.motor-wont-start`      | Start does nothing              | StopPB FALSE at rest, 0 V at the Stop terminal, no diagnostics                | NC contact block      | start, run, stop           |
| `ts.part-sensor-inverted`  | Early batch lamp, false jam     | PartPresent TRUE with no part, FALSE with a part                              | NO sensor             | batch count, jam detection |
| `ts.tank-level-reads-high` | Filling stops at about 50 %     | 5.6 mA at 10 % level, HMI 28 %, error shrinking toward full                   | channel to 4-20 mA    | scaling, fill, stop        |
| `ts.outputs-dead`          | Contactor does not pull in      | MotorContactor TRUE, no-field-power diagnostic, 0 V at the output             | restore load supply   | motor runs after Start     |
| `ts.vfd-local`             | Conveyor still, no drive fault  | DriveRun TRUE, run terminal ON, DriveReady TRUE, DriveRemote FALSE, LOC, 0 Hz | drive back to Remote  | command, motion, stop      |
| `ts.remote-level-wrong`    | Tank stays empty, level > 100 % | Comm OK, first raw word about 16 700 (float high word)                        | start address 0       | fill, stop, comm guard     |
| `ts.remote-level-lost`     | Tank does not fill, comm fault  | Timeouts, ping timeout to 192.168.0.20, no route to 192.168.1.20              | documented IP address | fill, stop, comm guard     |

`troubleshooting_exercises_fail_until_repaired_and_pass_afterwards` proves every lab fails
with the fault and passes after the repair, and requires a complete debrief. Visibility
(after the B1 correction): symptom, method and diagnosis options from the start; general
learning references ("Helpful concepts") from the start; repair options after a correct
diagnosis; debrief and the cause-specific guide after a passed verification. This is
enforced by the backend and tested for all seven labs
(`every_troubleshooting_lab_hides_its_cause_until_verified`). The native smoke completes
`ts.motor-wont-start` in the real UI on the live simulation: no guide title, repair label
or debrief text before the diagnosis, Run, press Start, observe the motor stopped,
diagnose, repair, Verify repair, read the four debrief headings and the guide link. See
[troubleshooting model](TROUBLESHOOTING_MODEL.md).

## Grading

Isolated scenario runs, evidence layers from causal observations, behavioral rules and
deterministic reports. Every practice exercise passes with its reference solution and
fails with its starter (`every_practice_exercise_is_solvable_and_the_starter_is_not`).
See [grading model](GRADING_MODEL.md).

## Project format

`.plclab` v1 with v0 migration and future-version rejection, atomic writes, autosave of
source and watch list, Save As, import, export and a progress file. Adapter test
`projects_persist_source_watch_station_and_platform_and_reject_bad_files`: New, Save,
Close, a restarted application instance, Open and Save As keep source, watch list,
station, platform, scene and speed, and Save As leaves the original unchanged; a corrupt
file and a future-version file in the projects folder are listed as problems without
failing the listing; importing a future version returns `APP-PROJECT-VERSION`; imports
for an unknown scene and oversized imports are rejected. The native smoke round-trips a
project through export and import and rejects a corrupt import. An unusable
`progress.json` is kept aside as `progress.json.unreadable-<seconds>` with a notice, never
overwritten. Only one instance runs per user (single-instance plugin), so two application
states cannot overwrite each other's progress or projects. See
[project format](PROJECT_FORMAT.md).

## Knowledge pack

Pack `plclab.core` 1.0.0: 74 articles (21 fundamentals, 7 instructions, 24 vendor and
family pages for 8 vendors, 9 protocols, 13 troubleshooting guides), 20 lessons,
17 exercises (10 practice, 7 troubleshooting), 5 scenes, 24 source records and 9 platform
profiles. Original text; source records with verified/unverified status; simulation
fidelity levels on protocols, profiles and the simulation pages.

Final archive `releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip`: 125 files, sha256
`90fc3b44126279fd7609cd72d3c7bbf63e4cf564eca2cb413366ec7b2141fe93` (also in the
`.sha256` file). `npm run knowledge:verify` proves it is the deterministic build of the
committed pack. `npm run knowledge:install-check` extracts it into
`%TEMP%\PLC Lab knowledge check <random>\`, validates 140 items against the schemas
(0 problems), and the application discovers the pack there: 111 indexed items, and every
catalog page (74), lesson (20) and exercise (17) opens with resolved references. First
search hits: `4-20 mA` -> lesson.4-20ma, fundamentals.4-20ma; `PROFINET device name` ->
protocol.profinet; `Modbus register offset` -> troubleshooting.modbus-wrong-register,
protocol.modbus-tcp; `VFD remote mode` -> troubleshooting.vfd-local-remote,
fundamentals.drives-basics; `ET200SP` -> family.siemens-et200sp; `CompactLogix` ->
family.rockwell-compactlogix. Ranking rules are also tested on synthetic pages in
`knowledge-core` (a page about a topic outranks pages that repeat its words, no bonus
without a matching term, stop words do not dilute the title share, ties break by
identifier).

Content audit in this pass: no empty sections, placeholders or stale "not simulated"
claims; the shared boilerplate "In PLC Lab" sections of 5 families and 8 protocols were
replaced by page-specific statements limited to what PLC Lab actually does (profile
address formats and analog representations, related labs). Tier-B vendor pages remain
short overviews (35-58 words) because longer text would need unverified vendor facts.
See [knowledge pack](KNOWLEDGE_PACK.md).

## Learning model

20 lessons, 10 practice exercises including the minimum set (pushbutton and lamp, motor
jog, seal-in start/stop, TON delayed start, edge counter, tank filling) listed in
`curriculumOrder`, Basic and Advanced modes. See [learning model](LEARNING_MODEL.md).

## UX

Shell with Home, Learn, Practice, Troubleshoot, Free Lab, Knowledge, Progress and About;
engineering workspace with project tree, center tabs, inspector (task or investigation,
cause & effect, properties) and monitoring dock (variables, watch, waveforms, events,
messages); HMI symbols including the drive keypad. The PLC mode switch and the simulation
controls explain themselves, and only one command runs at a time. The native smoke checks
the Free Lab workspace at 1366x768, 1920x1080 and 2560x1440: no horizontal overflow, and
toolbar, editor and inspector fully visible.

## Security

No `unsafe` code; student code only in the bounded VM; web view limited to event
listen/unlisten; no shell, filesystem, HTTP or dialog plugins; CSP without `unsafe-eval`;
knowledge loading without symlinks and with size limits; Markdown rendered without HTML;
release builds contain no repository paths (architecture test); `PLC_LAB_DATA_DIR` is
read by the trusted host only and `PLC_LAB_KNOWLEDGE_ROOT` by a test only; npm production
audit and cargo-deny in CI. The only host plugin is `tauri-plugin-single-instance`
(reviewed: no web-view commands or permissions; Windows named mutex and message-only
window, Linux session D-Bus; the second launch's arguments and working directory are
ignored). Smart App Control on the development machine was never disabled or bypassed.

### Dependency advisory exception

| Item            | Value                                                                                                                                          |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Package         | `glib` 0.18.5 (gtk-rs)                                                                                                                         |
| Advisory        | RUSTSEC-2024-0429 (unsoundness in `glib::VariantStrIter`)                                                                                      |
| Dependency path | transitive: `tauri` 2.12.3 -> Linux GTK/WebKitGTK stack (`gtk` 0.18.2, `webkit2gtk` 2.0.2) -> `glib` 0.18.5; Linux targets only                |
| Why accepted    | the fix requires glib >= 0.20, which Tauri 2 does not support; PLC Lab never calls the affected API; the Windows product does not compile glib |
| Where           | `deny.toml` `[advisories] ignore` with the reason inline; every other advisory still fails the Linux lane                                      |
| Upgrade plan    | remove the exception when Tauri moves its Linux stack to gtk-rs >= 0.20                                                                        |

## Performance

Engine cycle work is proportional to the signals of scene and program; trace memory is
bounded by its budget (measured above). The frontend keeps only the latest live frame;
variables are fetched in pages, trends are decimated by the backend to the requested point
count, the event list is capped at 5 000 entries, messages at 200 and the watch list at 64
signals; reads have at most one request in flight; Monaco loads lazily.

## Cleanup

- Earlier in the rebuild: v7 contract modules and tests, the prototype adapter
  (`execution`, `process`, `trace`), the `LabShell` UI with its hooks, fixtures and tests,
  the v7 IPC client modules, the regex architecture tests, the v7 native smoke modules,
  `RUN_PLC_LAB_RELEASE.bat`, the Sprint 0 `exercises/` and `scenes/` placeholders and
  unused dependencies.
- In the final pass: the port-name heuristic for output energization (replaced by port
  metadata), dead public accessors (`SceneRuntime::component_kind` and its field,
  `StationRuntime::diagnostics_mut`, `StationRuntime::set_profile`,
  `LabSession::scene_definition`, `NetworkRuntime::plc_last_activity`, `World::cancel`),
  the unused UI helper `isTrue`, five unused CSS rules, current-tense statements in
  prototype-era specifications (now marked), the name-based selection of device commands
  in the applied layer, and the "Alpha" milestone name in user-visible messages. Checks: no unreferenced Rust or npm
  dependencies, no tracked screenshots, logs or scratch files, no absolute developer
  paths outside historical documents.
- Retained intentionally: milestone reports and prototype ADRs/specifications (marked
  historical), `.plclab` v0 migration (supported format), the audit report
  (`docs/audit/feedback.md`), test fixtures in use.

## CI

Windows lane (`npm run ci:windows`): canonical check (format, typecheck, ESLint, clippy,
architecture, frontend and Rust tests, production build), release-profile Rust tests for
the whole workspace, knowledge archive verification, installed knowledge pack check, debug
native smoke, release build, release staging, release native smoke, root `PLC Lab.exe`
smoke, `RUN_PLC_LAB.bat` smoke, and smoke runs of `PLC Lab.exe` and `RUN_PLC_LAB.bat`
copied with `releases/current/` into `%TEMP%\PLC Lab Test <random>\`. The validated release
is published as the artifact `plc-lab-windows-release-<attempt>`. Every native smoke run
executes 16 steps against the real WebView2 application: shell navigation, contract and
knowledge discovery, Monaco editing, Build & Load, operator input, cause & effect,
isolated grading, a complete troubleshooting lab with debrief, a project round trip, the
Free Lab workspace at three resolutions, knowledge search, the toolbar explanations, a
second launch that must hand over to the running instance, and a close request (the
same WM_CLOSE as the window's close button) after which the application must exit, with
exit code 0 when the smoke started it directly. Linux lane (`npm run
ci:core`): format, clippy, debug and release tests for every crate except the WebView
host, frontend tests, architecture tests, knowledge verification, installed knowledge pack
check, npm production audit and cargo-deny. Actions are pinned by SHA, permissions are
read-only, Node is pinned exactly, no stage uses `continue-on-error`, and evidence uploads
fail when files are missing.

## Release

| Item                   | Path (relative to the repository)                             |
| ---------------------- | ------------------------------------------------------------- |
| Staged release         | `releases/current/` (generated, not in Git)                   |
| Application            | `releases/current/PLC Lab Desktop.exe`                        |
| Bundled knowledge pack | `releases/current/knowledge-packs/PLC_Lab_Knowledge_Pack_v1/` |
| Release manifest       | `releases/current/release-manifest.json`                      |
| Root launcher          | `PLC Lab.exe` (generated, not in Git)                         |
| BAT fallback           | `RUN_PLC_LAB.bat`                                             |
| Knowledge ZIP          | `releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip`            |

The executables are not code-signed; the Microsoft Edge WebView2 runtime is required.

## Launcher

The root launcher resolves `releases/current/PLC Lab Desktop.exe` from its own location and
does not depend on the working directory (the smoke starts it from `%TEMP%`). The smoke
runs use the release build only: no npm, cargo, Vite or development server is involved,
in the relocated runs the knowledge pack is asserted to load from the copy, and every run
ends by closing the application and waiting for its process to exit.

Local result on the development machine: **BLOCKED**. A release build attempted on
2026-10-28 failed because Smart App Control refused the freshly compiled proc-macro DLLs
`serde_derive` and `phf_macros` (os error 4551), and the debug native build of the final
source failed the same way on `tauri_macros`; Windows security was not changed. The
launcher results for the final source are in Final validation.

## Known limitations

- Executables are not signed; Smart App Control or SmartScreen may block a locally built
  copy, and local release builds on the development machine are blocked.
- Fixed cycle time and a single task; no retentive memory.
- Simplified physics; analog bands and diagnostic texts approximated.
- The drive has no speed reference, parameters, trips or safety functions.
- Modbus TCP is behavioral; all other protocols are knowledge only.
- Vendor firmware, dialects and instruction sets are not emulated; tier-B vendor pages are
  short overviews.
- Only Structured Text; LAD/FBD/STL editors do not exist (H6).
- TypeScript validators are hand-written against generated types (M5).
- Audit items L1, L7, L9 and L10 of `feedback.md` are deferred.
- Release-candidate audit lows deferred: L2 (lesson checkpoints scored in the frontend),
  L4 (trace memory accounting is an estimate), L8 (the launcher's error message box needs
  PowerShell).
- Single-instance protection is per user and application identifier; a second launch's
  arguments are ignored (no project path is forwarded).

## Final validation

Final source commit `fb57c7bb8746ed908c57b9fe85ef6decc4a0b4ec` (this report is recorded in a separate documentation-only
commit; `docs/` is not part of the staged release). GitHub Actions run
[40706676741](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/40706676741) validated
that exact SHA on both lanes:

| Lane    | Job          | Result  | Stages                                                                                                                                                                                                 |
| ------- | ------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Windows | 122880558327 | success | canonical-check, rust-release-tests, knowledge-pack, knowledge-install, native-desktop, release-build, release-stage, release-native, launcher-native, launcher-bat, launcher-relocated, bat-relocated |
| Linux   | 122880558857 | success | rust-format, core-clippy, rust-debug-tests, rust-release-tests, frontend, architecture, knowledge-pack, knowledge-install, npm-audit, cargo-deny                                                       |

| Evidence                     | Result                                                                                                                                                                                                                                                                                                                                                                                  |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust tests                   | Windows 306 passed, 0 failed in each of the debug and release runs (whole workspace); Linux 305 passed, 0 failed in each of the debug and release runs (all crates except the WebView host)                                                                                                                                                                                             |
| Frontend tests               | 33 passed (5 files) on both lanes                                                                                                                                                                                                                                                                                                                                                       |
| Architecture tests           | Windows 29 passed; Linux 27 passed, 2 skipped (the two BAT tests run only on Windows)                                                                                                                                                                                                                                                                                                   |
| Knowledge                    | archive verified (125 files, 140 items, 0 problems); installed-pack check PASS on both lanes                                                                                                                                                                                                                                                                                            |
| Supply chain                 | npm production audit: 0 vulnerabilities; full npm install audit: 0 vulnerabilities; cargo-deny: advisories, bans, licenses, sources ok                                                                                                                                                                                                                                                  |
| Native smoke (16 steps each) | PASS for `target\debug\plc-lab-desktop.exe`, `releases\current\PLC Lab Desktop.exe`, root `PLC Lab.exe`, `RUN_PLC_LAB.bat`, and `PLC Lab.exe` and `RUN_PLC_LAB.bat` copied into `%TEMP%\PLC Lab Test <random>\`; each run includes the troubleshooting secrecy and debrief flow, the project round trip, the three Free Lab resolutions, the second-launch handover and the clean close |
| Release artifacts            | `plc-lab-windows-release-1` (id 12824889051, sha256:be17e2a90d49a72757e59af4c51c49c3fefd2a5fe370ee94478c202083c1d449: staged release, root launcher, knowledge archive built from this SHA); `windows-validation-1` (id 12824662958); `linux-core-1` (id 12824826404)                                                                                                                   |

ROOT LAUNCHER: PASS on hosted Windows CI for the final SHA (root `PLC Lab.exe`,
`RUN_PLC_LAB.bat` and both relocated copies). Local root launcher on the development
machine: BLOCKED (Smart App Control, see Launcher).

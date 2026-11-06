# Development handoff (0.2.0-dev)

Branch `claude/product-development-v2`, started from the accepted rebuild report commit
`84918f1` (validated source `fb57c7b`). This file records where development stands so
work can continue from a clean, buildable state.

## Checkpoint 1: Launcher.exe (green)

Done:

- One canonical version, `0.2.0-dev`: the Cargo workspace version, inherited by every
  crate, the npm packages and the Tauri bundle (`tauri.conf.json` has no own version).
  Enforced by `tests/architecture/version.test.mjs`.
- Typed project hand-over (ADR-0026): `--open-project <file.plclab>` parsed only by
  `apps/desktop/src-tauri/src/startup.rs`; second instances forward it through the
  single-instance plugin; `DesktopApp::request_open` opens store files in place and
  imports other files; the UI pulls the result with `take_open_request` (contract 8,
  additive) on start and on the `lab-open-request` event.
- Recent projects: `recent.json` in the data folder (`project_model::RecentProjects`),
  written when a workspace opens, read by the launcher.
- `Launcher.exe` (`apps/launcher/src/bin/launcher.rs`, logic in `apps/launcher/src/lib.rs`):
  native dark window (winsafe, custom-drawn buttons, Common Controls 6 manifest), status,
  recent project, knowledge pack, installation checks, diagnostics with copy, data and
  knowledge folders, CLI modes `--start`, `--diagnostics`, `<file.plclab>`. `PLC Lab.exe`
  remains as alias. Release staging installs both and records their hashes.
- Windows CI stage `launcher-gui` (`npm run test:launcher:gui`,
  `tests/e2e/launcher-smoke.mjs` + `tests/e2e/launcher-window.ps1`): PE subsystem, the
  diagnostics report, corrupt manifest, missing release (CLI and window), refused
  arguments and files, the window contents, Diagnostics + Copy, Start, hand-over to a
  running instance, Open Recent, second start, Open Project dialog, cold start with a
  project argument, clean close.

Not done (deliberately deferred, documented in ADR-0026): installing a knowledge pack
ZIP from the launcher. "Manage Knowledge Packs" opens the trusted user folder.

Validated: CI run 40753806573 (Windows and Linux lanes green, launcher-gui 11 steps).

## Checkpoint 2: commissioning framework and digital I/O lab (P1, P2)

- Engine-evaluated checklist (ADR-0027, `crates/lab-engine/src/commissioning.rs`): Power,
  Hardware, Network, Inputs, Outputs, Drives, Process; states OK / ATTENTION / FAULT /
  UNTESTED / NOT_APPLICABLE; I/O tests tracked per cycle; never reads injected faults.
- `commissioning` query (contract 8, additive), Commissioning tab in every workspace,
  COMMISSION navigation section, `ExerciseKind::Commissioning` with the troubleshooting
  flow (`is_fault_finding`).
- Scene `scene.dio-commissioning` and six labs (`cm.dio-*`): input field supply, guard
  switch NO/NC, swapped Start/Stop, swapped outputs, missing contactor feedback, module in
  the wrong slot. Article updates: `fundamentals.commissioning` (checklist and fidelity),
  new `troubleshooting.missing-feedback`.
- Tests: `crates/lab-engine/tests/commissioning.rs`, the secrecy and verification workflow
  over all 13 fault-finding labs, UI tests, a native smoke step.

Validated: CI run 40757889061 (both lanes green; native smoke 17 steps on every entry point).

## Checkpoint 3: analog commissioning (P3)

- Model: `TransmitterWiring` (2-wire / 4-wire) on level transmitters and `loopSupply` per
  current input channel (contract field `ChannelConfigDto.loopSupply`, editable in the
  Hardware view, Advanced mode). Generic loop rule in `hardware-model` (see
  SIMULATION_FIDELITY).
- Checklist: analog signal type and loop against the drawing, raw value with measured
  current or voltage and condition, and the transmitter value against the sight glass.
- Scene `scene.analog-commissioning` and five labs (`cm.ai-*`): broken loop, missing loop
  supply, 0-10 V sensor on a current input, 0-20 versus 4-20 mA, zero offset. Guides
  `troubleshooting.analog-loop-supply`, `troubleshooting.analog-calibration`; sections on
  2-/4-wire transmitters (4-20 mA) and commissioning checks, zero/span and reversed ranges
  (scaling).
- Not covered by a lab yet: reversed range and wrong program scaling (taught in the
  scaling article; a program-fix lab needs a verification path for student programs in the
  secrecy workflow).

Validated: `2b6b8779106c9bf167214a12a4dbc338dfe1ac92`, CI run 40830883517 attempt 2 (Windows
job 123306737809 and Linux job 123306740081 PASS; native smoke 17 steps on every entry
point, launcher smoke 11 steps). Attempt 1 lost the Windows job to an `npm ci` process
crash (exit 0xC0000409) before any project code ran.

## Checkpoint 4: drive commissioning (P4)

- Generic VFD behavioral model (`process-simulation` `Drive`): optional run enable, fault
  relay and acknowledge terminals, preset speed, keypad reference, latched trip, mains
  supply; ramp continues from the current speed when the reference rises.
- Faults: `driveTrip`, `driveKeypadReference`, `driveNoSupply` (plus the existing
  `driveLocalControl` and `openCircuit` for a missing enable wire).
- Checklist Drives: ready (with the missing condition), local/remote, run enable, fault,
  run command, speed reference (value and source), speed and feedback; Power shows the
  drive mains supply.
- Content: `scene.drive-commissioning`, six labs with different evidence: `cm.vfd-local`
  (ready, LOC, command received but not accepted), `cm.vfd-trip` (latched fault, reset
  refused until the cause is removed), `cm.vfd-enable` (commanded, not ready: enable
  missing at the drive), `cm.vfd-keypad` (runs at the keypad speed, selector ignored),
  `cm.vfd-no-supply` (display dark, every relay open), `cm.vfd-run-wire` (ready and remote,
  PLC output at 24 V, command not received). Lesson 21 (drive commissioning), guides
  `drive-not-ready`, `drive-trip-acknowledge`, `drive-reference-source`,
  `drive-command-path` with fidelity classification, the generic model section and
  fidelity of `fundamentals.drives-basics`, `fundamentals.transmitter-wiring`.
- Review fixes (separate commit after `72e074d`): relays of an unpowered drive are open,
  undeclared terminals are refused, drive speed values are validated, port roles are model
  metadata (`ComponentKind::port_role`) instead of name lists in the checklist, display
  readings of an unpowered drive are not shown; tests for drive invariants over a
  deterministic random sequence, held reset, causal chain PLC -> drive -> process ->
  feedback -> next scan.
- Not done: communication timeout of a fieldbus-controlled drive (needs the network work of
  P5).

Validated: `ca8d02f08712472f9d108b73148d176bbed9599d`, CI run 40833648513 (Windows job
123313305166 and Linux job 123313305607 PASS; native smoke 17 steps on every entry point,
launcher smoke 11 steps).

## Checkpoint 5: network diagnostics (P5)

- Network projection (ADR-0028, contract 8 additive): per connection the downloaded
  request, backend-decoded values and last request/response times; `traffic` (newest 60
  structured events); device subnet membership. The text list of recent messages was
  removed in favor of `traffic`.
- Network view: nodes, connections, inspector (request, 4xxxx notation, raw words,
  decoded values), Modbus traffic with standard exception names, ping.
- Content: `scene.network-commissioning` (LT-401 direct, LI-402 behind gateway GW-402 with
  unit ID 5, REAL, 40 ms processing), six labs `cm.net-*` (wrong IP, unit ID, timeout, word
  order, register 40101 vs 101, device offline), guides `modbus-wrong-address`,
  `modbus-unit-id`, `modbus-timeout`, `modbus-word-order`, `modbus-device-offline` with
  fidelity; `modbus-wrong-register` now also simulated in `cm.net-register`.
- Tests: projection test (request, values, paired traffic, exception 02, bound), per-lab
  network evidence, secrecy and verification over all 30 fault-finding labs, Network
  view UI test, native smoke step (gateway exception in the traffic).
- Not done: a fieldbus-controlled drive with communication monitoring (P4 follow-up), write
  functions.

Validated: `4ffe42296ce28ea89738dcbf3c4e95bcecc57110`, CI run 40835722311 (Windows job
123319920831 and Linux job 123319921115 PASS; native smoke 18 steps on every entry point,
launcher smoke 11 steps).

## Checkpoint 6: hardware configuration and station building (P6)

- Model (ADR-0029): stable module ids (`<class>-r<rack>s<slot>`, unique, kept on move)
  and channel ids `<moduleId>.ch<n>`; `StationConfig::move_module` moves the I/O mappings;
  module electronics supply separate from field supply (`NO_MODULE_POWER`, fault
  `modulePowerMissing`); analog outputs bound to `INT`/`REAL`, driven in RUN only,
  measurable; project format 2 with migration from 1 (progress and recent lists keep
  their own versions); canonical I/O signal identity `io.<moduleId>.ch<n>` (position only
  as label, a moved module keeps its signal ids) with project format 3 migrating stored
  watch entries.
- Diagnostics: `StationRuntime::compare` derives typed findings (missing, unexpected,
  wrong type, wrong slot, module power, field power, channel mode and loop supply against
  the drawing, address overlap, mapping problems); the commissioning Power and Hardware
  categories consume them (items module electronics supply, modules present in their
  slots, addresses and I/O mapping) instead of their own rules.
- Projection and UI: hardware view with module/channel ids, findings and a signal map
  (symbol, channel, address, value and quality, drawn device and its state). Hardware tab:
  Compare, Configured (project), Actual (detected), Signals; inspector Overview,
  Configuration, Channels, Power, Diagnostics; module cards are keyboard-selectable
  buttons; Advanced mode adds, removes, moves and retypes modules (first-fit start
  addresses from the catalog), each edit a validated project save shown from the
  backend's answer, downloaded explicitly. Diagnostic entries carry typed slot, channel
  and module id. CPU and module LEDs are labeled generic PLC Lab indicators.
- Content: `scene.hardware-station` (modular CPU, PS, DI16, DO16, AI4, AO2; tank skid with
  inlet valve and limit switches, high level switch, 2-wire LT-303, lamps; AO channel 0
  retransmits the level), six labs `hw.do-field-supply`, `hw.do-module-power`,
  `hw.wrong-module`, `hw.sensor-channel`, `hw.ai-mode` (configuration lab), `hw.wrong-slot`;
  guides `module-electronics-supply`, `wrong-module-type`, `signal-on-wrong-terminal`,
  `analog-mode-mismatch` (with fidelity); `missing-field-power` and `wrong-module-slot`
  now also simulated in hardware labs; `fundamentals.station-configuration`; lesson 22.
- Tests: hardware-model (AO, two supplies, comparison of project, installation and
  drawing, large station 1 CPU + 8 DI + 8 DO + 4 AI + 4 AO over 2000 scans with a time
  budget and determinism), project format v1 -> v2 migration, per-lab hardware evidence
  and commissioning consistency, large-station hardware view bound, moved module id
  survives save and reopen, secrecy and verification over every fault-finding lab of the
  pack (36, now derived from the library and including the Hardware and Commissioning
  views), Hardware tab and station editor UI tests, native smoke step (compare, keyboard
  module selection, inspector diagnostics, detected station, signal mapping).
- Not done: remote I/O and communication modules as configurable station members (only
  the CPU interface exists); no scene device consumes an analog output yet (the AO
  terminal is measured with the multimeter).

Validated (final, including canonical I/O identity and project format 3):
`e8fc58d5311d625a064dcb36ccdde62e14f9200f`, CI run 40885264980 (Windows job 123487147173
and Linux job 123487147505 PASS; native smoke 19 steps on every entry point, launcher
smoke 11 steps). An earlier candidate (`5740f19`, run 40880723450) passed before the
identity requirement was added. All Rust of this checkpoint was compiled and tested only
on hosted CI: the local Rust toolchain could not be reinstalled (`rustc.exe` refused with
access denied, see Local machine limits).

## Checkpoint 7-A: root single-executable distribution

- The root `PLC Lab.exe` is the Tauri application itself (ADR-0030), staged from
  `target/release/plc-lab-desktop.exe`; its data lives in `resources/` beside it
  (`knowledge-packs/` from the verified archive, `product.json` with the DLL audit and
  SHA-256 of the executable and every file). The distribution has exactly one application
  executable (`scripts/distribution.mjs`, CI stage `release-distribution`).
- The application finds `resources/` from `current_exe()`; a release without its
  knowledge pack, an unusable data folder or a window that cannot be created (WebView2
  hint) is reported in a native message box (`winsafe`). `PLC Lab.exe "<file.plclab>"`
  opens a project; a second launch hands it to the running instance.
- Retired: `apps/launcher` (`Launcher.exe`, the alias), `releases/current/`, the launcher
  smoke and stages. Staging removes old `Launcher.exe` and `releases/current/`.
  `RUN_PLC_LAB.bat` only starts `"%~dp0PLC Lab.exe"`.
- Windows CI: native smoke of the root `PLC Lab.exe` from another working directory, of
  a copy of the distribution in a folder with spaces, and through the BAT; each checks
  the distribution, the knowledge source in `resources/`, single instance, the project
  file hand-over (not through the BAT) and a clean close.

Validated: `1bad6c3a993e6ebf970c616d732dcb0d66185ce3`, CI run 40916706482 (Windows job
123598751308 and Linux job 123598750901 PASS; distribution check PASS with one application
executable; native smoke of the root `PLC Lab.exe` 20 steps, of the relocated copy in
`%TEMP%\PLC Lab Product Test With Spaces …` 20 steps, through the BAT 19 steps, debug
build 20 steps).

## Checkpoint 7-B: saved versus downloaded configuration across reopen

- Project format 4 (ADR-0031): `downloadedStation` beside `station`. A workspace opens
  with the PLC holding the last downloaded configuration and records it; a download sets
  both; saving changes only the expected `station`. Format 3 migrates unchanged.
- Tests: project-model (format 3 without, round trip with both, invalid downloaded
  configuration refused, format 5 rejected); workflow: a saved, not downloaded edit stays
  pending after reopen, a downloaded one survives an application restart, the exported
  file carries `downloadedStation`.

Validated: `7a31ebe2e0224b4199e15055a73a456a25f89a6f`, CI run 40920318148 (Windows job
123611455125 and Linux job 123611455530 PASS).

## Checkpoint 7-C: watch tables

- Project format 5 (ADR-0032): named `watchTables` (16 x 64 rows of canonical signal
  paths with an integer display format) replace the flat watch list; format 4 lists
  migrate into "Watch 1".
- Typed `watch` query (IPC contract 8, additive): per signal the canonical path, label,
  kind, type, unit, value, scan input, committed value, next input, source, quality and
  force, for PLC tags, block outputs, process values and I/O channels; unknown ids are
  left out; more than 64 are refused.
- Watch panel: table tabs, new, rename, delete, find and add any signal (including I/O
  channels), per-row format, configurable columns (per-viewer preference), rows that no
  longer resolve shown as not available. The active table feeds the frame and waveforms.
- Tests: project model (migration, round trip, limits), workflow (every signal kind, a
  force, unknown ids, the 64 limit, a moved module's channel row still resolves),
  persistence of watch tables, UI (values, quality, force, columns, editing).

Validated: `4a3ba91e8023724ca5173d818bd87fed407a0b92`, CI run 40925230579 (Windows job
123628821057 and Linux job 123628820816 PASS; native smoke 20 steps on the debug build,
the root `PLC Lab.exe` and the relocated copy, 19 through the BAT).

## Checkpoint 7-D: forcing

- ADR-0033: forces target the PLC input image or the PLC output image of I/O tags,
  typed and validated by the backend; locals and the field state are not forceable.
  Lifetime: kept over STOP/RUN and downloads that keep the symbol, ended by a download
  without the symbol and by a memory reset (each removal logged), not persisted (a
  reopened workspace starts without forces).
- Engine `active_forces`, `remove_all_forces`; IPC `forces` and `remove_all_forces`.
- Workspace: persistent FORCES ACTIVE banner (count, list with target and value, Remove,
  Remove all forces); typed INT/REAL force entry beside BOOL in Variables.
- Tests: engine (typed forces, targets, STOP/RUN, memory reset, remove all, download
  pruning, all logged), workflow (list, remove all, reset, reopen), UI (banner, list,
  removal, typed entry).

Validated: `41f450bced90b4963782ef8496defbb6360b86f7`, CI run 40928237207 (Windows job
123639422507 and Linux job 123639422092 PASS; native smoke 20 steps on the debug build,
the root `PLC Lab.exe` and the relocated copy, 19 through the BAT).

## Checkpoint 7-E: diagnostic buffer

- ADR-0034: the bounded CPU buffer also records single events (CPU operating state
  changes with before and after, program and hardware downloads). The backend derives
  severity, category, structured details and, for incoming conditions, active or the
  time they cleared.
- Typed `diagnostic_buffer` query (newest first, filters severity, category, module id,
  pages of at most 200); Diagnostics tab view with filters, states, details and paging.
- Tests: engine (mode changes and downloads recorded as events), workflow over the
  field supply lab (active error of `do-r0s3`, cleared after the repair, CPU events with
  details, filters, order and bounds), UI (states, details, filters, paging).

Validated: `61c3f9aecc23b76bd6f4a9b16ad36190cb858c99`, CI run 40959373131 (Windows job
123747799783 and Linux job 123747800120 PASS; native smoke 20 steps on the debug build,
the root `PLC Lab.exe` and the relocated copy, 19 through the BAT). The first candidate
`83c8268` (run 40958561936) failed only the rustfmt check; `61c3f9a` formats it.

## Checkpoint 7-F: online variable detail and the native engineering smoke

- Choosing a variable in Variables or a watch row (Details) shows its online detail in
  the inspector: canonical id, kind, type, value, scan input, committed value, next
  input, quality, source, force state with force controls for PLC I/O, the I/O binding
  (channel id, address, device and port on the drawing, device state) and the recent
  value changes from the trace (last 20 s). Every value comes from the `watch`, `trend`
  and `hardware` projections; the view evaluates nothing. No IPC change.
- Native smoke step "engineering tools" on every entry point: a Free Lab project on the
  hardware station is built and loaded, a watch table gets `LevelRaw` and shows its
  online detail, `InletValve` is forced (banner and force list), all forces are removed
  (banner gone), and the diagnostic buffer shows the program download and the STOP to
  RUN change.
- Tests: UI (detail values, quality, binding, recent changes, force controls), native
  smoke.
- Limitation: the declaration's source location is not shown; the ST compiler keeps no
  declaration spans yet.

Validated: CP7-F (`4c4618e`, `5b0b6f6`, `b25e595`) never ran on its own; it was pushed
together with the toolchain fail-fast fix. Its first green source is
`3f0840b4565ab8d51430d60c74d36081f8965715`, CI run 40962333885 (Windows job 123758082757
and Linux job 123758082588 PASS; native smoke with the engineering tools step 21 steps
on the debug build, the root `PLC Lab.exe` and the relocated copy, 20 through the BAT).

## After Checkpoint 7-F: local build recovery and Polish interface (Codex)

- `3f0840b` (run 40962333885 PASS): `scripts/toolchain.mjs` accepts a project-local or
  user toolchain only when the exact pin from `rust-toolchain.toml` and its components
  execute; an incomplete toolchain stops the human build at once instead of looping in
  rustup. Evidence: `docs/validation/LOCAL_BUILD_RECOVERY.md`.
- `54c97b9` (run 40967852784 PASS): interface preferences (English, Polish), the student
  workspace and Basic/Advanced presentation (ADR-0035, `docs/UI_LOCALIZATION.md`).
- `ad54bc5` (run 40972186144 PASS, native smoke 24 steps, 22 through the BAT): Polish
  learning content as guarded overlays over the canonical English pack (ADR-0036,
  `knowledge/locales/pl.json`, `docs/validation/POLISH_CONTENT_COVERAGE.json`).

## Checkpoint 7: final gate

Reviewed against the source of `ad54bc5`:

| Criterion                                                         | Evidence                                                                                                                                                                                              |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Saved versus downloaded configuration across reopen               | 7-B workflow test, ADR-0031                                                                                                                                                                           |
| Watch tables, canonical targets, persistence, unavailable rows    | 7-C tests, ADR-0032                                                                                                                                                                                   |
| Input and output image forcing, typed values                      | engine and workflow force tests, ADR-0033                                                                                                                                                             |
| Field value versus forced PLC input                               | a forced input replaces only the staged and scanned image; the channel signal and device state keep the field value (workflow test `forces_keep_field_and_program_values_apart_and_never_pass_a_lab`) |
| Program result versus forced output                               | the committed value stays the program's, only the terminal follows the force (same test)                                                                                                              |
| Remove all forces, global banner, explicit lifetime               | workflow and UI tests, native smoke                                                                                                                                                                   |
| Forces and grading                                                | grading runs in an isolated session with the faults still unrepaired and no live forces; forced feedback does not pass the drive lab, the real repair does, and the forces stay untouched (same test) |
| Bounded diagnostic buffer, transition events, deterministic order | 7-E engine and workflow tests, ADR-0034                                                                                                                                                               |
| Online variable detail, native engineering smoke                  | 7-F UI test, smoke step on every entry point                                                                                                                                                          |
| Single-instance root executable, knowledge from `resources/`      | native smoke on every entry point since 7-A                                                                                                                                                           |

Validated: CP7 FINAL GATE PASS on `135612551294a58d0ffe3b29965241cfc08a9e28`, CI run
41691014122 (Windows job 126109292632 and Linux job 126109293202 PASS; native smoke 24
steps on the debug build, the root `PLC Lab.exe` and the relocated copy, 22 through the
BAT). That source contains the gate test (`e25a0d0`), this record (`ab7a9a2`) and the
lockfile fix `1356125`: run 41687714171 on `ab7a9a2` passed on Windows but failed on Linux
only because cargo-deny rejected `async-executor 1.14.0`, yanked from crates.io after the
previous green run; `1356125` moves it to 1.14.1 and changes nothing else.

## Checkpoint 8-A: industrial alarm management

- ADR-0037: crate `alarm-core` (generic lifecycle `NORMAL`, `ACTIVE_UNACKNOWLEDGED`,
  `ACTIVE_ACKNOWLEDGED`, `RETURNED_UNACKNOWLEDGED`; priorities information to critical;
  on-delay in world time; atomic acknowledgment of a selection; bounded history of 500
  transitions with sequence numbers). The engine derives the plant's alarms from the
  scene (overflow, high-high, low-low, dry run, motor fault, drive fault, drive not ready,
  valve position timeout), the configured station (invalid analog signal, missing field
  supply, module fault) and the Modbus connections (communication lost), evaluates them
  at the end of every cycle and logs every transition. Acknowledgment never changes a
  condition; PLC STOP and memory reset leave process alarms; a world reset restarts them.
- IPC (contract 8, additive): richer frame alarms (most urgent first), `alarms` (list and
  history pages filtered by priority, state and source in the backend) and
  `acknowledge_alarm` (a selection, all or none, or every waiting alarm).
- Alarms view: Basic lists what needs attention with acknowledge and acknowledge all;
  a Basic-mode indicator names the most urgent alarm and what to investigate; Advanced adds
  filters, selection with acknowledge selected, details with inspect source, and the paged
  history. Columns: time, priority, source, message, condition, acknowledgment, action.
  The CPU diagnostic buffer stays separate. English and Polish throughout.
- Content: lab `ts.tank-overfill-alarm` (failed high level switch; the acknowledgment is
  a separately graded duty via `troubleshooting.acknowledge`), lesson 23
  `lesson.industrial-alarms`, articles `fundamentals.industrial-alarms` and
  `troubleshooting.level-switch-failed`; complete Polish overlays, coverage report
  regenerated.
- Tests: alarm-core (lifecycle, on-delay, refused and atomic acknowledgments, same-instant
  order, bounded history, reconfiguration), engine (activation from the real process at
  the exact world time, one activation per condition, acknowledgment leaves the process
  unchanged, STOP and memory reset keep process alarms, return only after correction,
  event log, world reset, determinism), workflow (the lab from symptom to debrief, most
  urgent first, backend filters, atomic selection, a forced level switch never passes),
  grading-core (separate acknowledgment criterion), UI (basic and advanced lists, filters,
  selection, details, history, empty states, Polish, indicator, keyboard-reachable
  controls), native smoke (normal, active, acknowledged but present, repaired, returned,
  history in order, verified) on every entry point.
- Validated: CP8-A PASS on `1fc2bbc3965e59db835f0c693dd6c96ae1257cf4`, CI run 41695556939
  (Windows job 126125006743 and Linux job 126125006135 PASS; native smoke 25 steps on the
  debug build, the root `PLC Lab.exe` and the relocated copy, 23 through the BAT, the new
  alarm lifecycle step on each; advisories ok). The local release build of this source
  was staged as the root `PLC Lab.exe` (byte-identical to `target/release`), but Smart App
  Control refuses to start the new unsigned binary on the development machine (Code
  Integrity events 3077 and 3033), so it was not launched locally.
- Local runs (Smart App Control allowed these executables): alarm-core, grading-core,
  lab-engine engine tests, desktop-session workflows, knowledge conformance, all
  architecture and frontend tests; rustfmt and clippy clean except the host crate, whose
  build script the policy blocks.

## Checkpoint 8-B: PLC startup, restart and retentive memory (in progress)

- ADR-0038. ST: `VAR RETAIN` (scalar program variables), constant initializers, reserved
  read-only `FIRST_SCAN`; RETAIN on inputs, outputs and FB instances refused. Canonical
  symbols carry retention and initial values; the runtime has one start initialization
  (warm keeps retained variables, cold initializes everything). STOP to RUN stays a warm
  start; download and memory reset start cold; warm and cold restart operations (RUN
  only, refused in FAULT); forces survive restarts, memory reset clears them. CPU
  diagnostic buffer: start (kind, retained count), first scan, memory reset, once each.
- IPC: `plc_restart`; PLC status (last start, retained count, first scan pending);
  retention, initial value and restart effect on variable, watch and declaration rows;
  scenario actions `warmRestart` and `coldRestart`.
- UI: confirmed warm restart, cold restart and memory reset (Advanced), start status,
  retention in the online variable detail (now English and Polish) and the variables table.
- Content: labs `ex.retentive-counter` and `ex.safe-startup` with reference solutions,
  lessons 24 and 25, article `fundamentals.plc-startup`, complete Polish overlays.
- Tests (all run locally and pass): ST retention (9), engine restarts (19 total), workflows
  (19), knowledge conformance incl. restart lab grading (5), architecture (50), frontend
  (83).
- Remaining before CP8-B can close: the native smoke step for the restart scenario
  (Free Lab retentive counter: RUN, warm restart, first scan, cold restart, diagnostic
  buffer), hosted CI PASS on the final SHA, then rebuild and stage the root `PLC Lab.exe`.

## Product consolidation (Claude, 2026-11-04 to 2026-11-06)

No PLC functionality was added. Commits on `claude/product-development-v2`:

- `c81bd49`, `966c036`: `TEST_PLC_LAB.bat` starts the native Tauri application with its
  Rust backend without signing; the release smoke waits for the closed WebView to
  release its debugging port before a restart.
- `90662fb`: the standalone browser preview is retired (ADR-0040 superseded); the Tauri
  frontend (React, Vite, WebView2, Monaco, IPC) is unchanged.
- `aaebc26`, `5b00579`, `b96e0cc`, `1c46b30`: every educational text is in Polish —
  49 exercises (practice, troubleshooting, 29 commissioning labs), 25 lessons,
  25 fundamentals, 7 instructions, 9 protocols, 30 troubleshooting and 24 vendor/family
  articles; 0 missing prose fields, every group declared complete, identical texts need
  a `keep` reason.
- `ffed1c5`, `9bd1da1`, `2500031`: every screen takes its labels from the EN/PL
  dictionaries (no literal left in the UI audit); diagnostic buffer, compiler/runtime
  codes, connection states, qualities and module names are worded by code; commissioning
  findings, hardware findings, the signal map and events carry coded messages from Rust
  (ADR-0041, IPC contract 8 additive, 307 Polish message and term codes).
- `2aaf54a`: light and system themes on semantic colour tokens, Monaco light theme,
  WCAG AA for 57 token pairs per theme (ADR-0042).
- Repository cleanup: see `docs/validation/REPOSITORY_CLEANUP.md`.

## Local machine limits

Smart App Control blocks freshly built release build scripts and some generated test
binaries on the development machine (os error 4551), so the complete local
`BUILD_PLC_LAB_RELEASE.bat` validation does not finish. It is never disabled or
bypassed. Since the 2026-10-29 recovery the exact pinned Rust works locally again
(rustfmt, clippy, the native release build and staging, and the test binaries the policy
allows); hosted Windows and Linux CI remain authoritative for the full suites and every
native smoke test. Antivirus settings are never changed by the agent.

## Next checkpoints

1. Checkpoint 8, one validated slice at a time: A industrial alarm engine (lifecycle,
   acknowledgment, bounded history, alarm view, an alarm lab); B PLC startup and restart
   (cold and warm restart, retained data, first scan, startup diagnostics, two labs);
   C sequence diagnostic foundation (steps, transitions, timeouts, hold and resume,
   interlocks and permissives, a sequence inspector, a process sequence). All new
   content in English and Polish.
2. Checkpoint 9: ST depth, multiple POUs and FB instances (P11), then LAD readiness.

## How to continue

```
npm ci
npm run check
node scripts/rust.mjs clippy --locked --workspace --all-targets -- -D warnings
node scripts/rust.mjs test --locked --workspace --exclude plc-lab-desktop
```

Push the branch and read the `validation` workflow (Linux core lane and Windows lane).
Never force-push; never weaken a check to get a green run.

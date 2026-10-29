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

## Local machine limits

Smart App Control blocks freshly built release build scripts and some test binaries on
the development machine (os error 4551). It is never disabled or bypassed. The launcher
release binaries themselves run locally; the full Tauri release build, staging and every
native smoke test run on hosted Windows CI, which is authoritative.

## Next checkpoints

1. Checkpoint 6: P6 hardware configuration / station building.
2. Then P7 watch
   and force, P8 diagnostic buffer, P9 alarms and events, P10 startup and restart,
   P11 ST depth, P12 LAD readiness.

## How to continue

```
npm ci
npm run check
node scripts/rust.mjs clippy --locked --workspace --all-targets -- -D warnings
node scripts/rust.mjs test --locked --workspace --exclude plc-lab-desktop
```

Push the branch and read the `validation` workflow (Linux core lane and Windows lane).
Never force-push; never weaken a check to get a green run.

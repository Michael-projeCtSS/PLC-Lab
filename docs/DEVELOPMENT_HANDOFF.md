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

## Local machine limits

Smart App Control blocks freshly built release build scripts and some test binaries on
the development machine (os error 4551). It is never disabled or bypassed. The launcher
release binaries themselves run locally; the full Tauri release build, staging and every
native smoke test run on hosted Windows CI, which is authoritative.

## Next checkpoints

1. Checkpoint 3: P3 analog commissioning lab.
2. Then P4 VFD, P5 network diagnostics, P6 hardware configuration, P7 watch
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

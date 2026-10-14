# M0 closeout and human-test tooling report

Date: 2026-10-14 (Europe/Warsaw). Scope: HT-001 through HT-008 and M0 architectural closeout. M1 was not started.

## Files changed

- Added the four root BAT launchers: RUN_PLC_LAB_DEV.bat, RUN_ALL_CHECKS.bat, BUILD_PLC_LAB_RELEASE.bat, RUN_PLC_LAB_RELEASE.bat.
- Added scripts/human-launcher.bat, scripts/human-test.mjs and scripts/stage-release.mjs for shared prerequisite/command handling, running-application checks and verified release staging.
- Added [HUMAN_TESTING.md](HUMAN_TESTING.md), [HUMAN_TEST_CHECKLIST.md](HUMAN_TEST_CHECKLIST.md) and releases/README.md; updated README.md and [DEVELOPMENT.md](DEVELOPMENT.md).
- Updated package.json and the Windows quality workflow to reuse npm run validate. Added npm run build:release. Updated .gitignore for generated releases and .gitattributes for Windows BAT line endings.
- Extended crates/app-contract, its generated TypeScript DTOs, packages/ipc and the existing LabShell Diagnostics detail line with authoritative buildContext. Updated [the IPC boundary](architecture/FRONTEND_BACKEND_BOUNDARY.md) and corresponding frontend/Rust/native tests.
- Added tests/architecture/human-test.test.mjs for dependency drift, exact process-name recognition, artifact-byte preservation/replacement, missing release and launcher failure propagation.
- Changed only the Tauri development-server invocation to call the installed Vite CLI directly, using the same application configuration. This removes misleading nested npm shutdown failures after normal DEV closure.

[UI_PRINCIPLES.md](spec/UI_PRINCIPLES.md) already contained every requested closeout principle and remains unchanged. AGENTS.md and shell CSS remain unchanged. Launcher/helper/documentation source is tracked/staged; generated release binaries and manifests are ignored by Git.

## Human Test workflow

| Root launcher             | Behavior                                                                                                                         | Result |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------ |
| RUN_PLC_LAB_DEV.bat       | Check tools/dependencies and launch the actual npm run dev Tauri application from current source; keep terminal output visible   | PASS   |
| RUN_ALL_CHECKS.bat        | Run canonical npm run validate; display ALL CHECKS PASSED only after complete success, otherwise CHECKS FAILED with nonzero exit | PASS   |
| BUILD_PLC_LAB_RELEASE.bat | Run full validation, compile real optimized Tauri release, audit imports and stage byte-verified artifacts to releases/current   | PASS   |
| RUN_PLC_LAB_RELEASE.bat   | Launch the last successful staged release; explain a missing release without building automatically                              | PASS   |

All four resolve the checkout from their own BAT location and preserve failure exit codes. Valid installed npm dependencies were reused during the real runs; none of these runs required reinstalling them. Windows testing covered launchers invoked from C:\Users\mjgom\Documents as well as from the repository, plus a temporary fixture path containing spaces for the missing-release case.

Normal double-click use pauses at completion so the owner can read output. Tests set PLC_LAB_NO_PAUSE=1 only to suppress that keypress; no validation/build bypass exists. DEV and RELEASE are distinguished by launcher output and the actual Rust-supplied context in Diagnostics. Close existing PLC Lab windows before validation/builds; prerequisite checks prevent rebuilding locked executable files without terminating the owner's application.

## Release artifact

The successful Tauri release source is:

```text
C:\Users\mjgom\Documents\PLC_Student\target\release\plc-lab-desktop.exe
```

The human-test executable is:

```text
C:\Users\mjgom\Documents\PLC_Student\releases\current\PLC Lab.exe
```

SHA-256 of both final executables:

```text
7D211DD904B8E0EE745A403D54C098650AC1C21FAE03D4DAB848D53C275F024A
```

They are byte-identical. Staging does not patch the executable. The current import audit found Windows system/API-set dependencies and no additional local DLL that needs copying. current/ also contains release-manifest.json and RUNTIME_REQUIREMENTS.txt. Future required local DLL imports will be copied as part of the verified artifact set.

**Startup verified:** RUN_PLC_LAB_RELEASE.bat opened the final executable with no Vite server running. Native window inspection confirmed the embedded application at http://tauri.localhost/, all seven panels, disabled controls, CORE CONNECTED, application/core 0.1.0, IPC v2 and RELEASE. Normal closure returned exit code 0. Actual startup and visible Rust IPC were checked; compilation alone was not used as acceptance evidence.

## Validation

| Actual command/script or check                      | Final result                                                                                                            |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| RUN_ALL_CHECKS.bat                                  | PASS; ALL CHECKS PASSED, exit 0                                                                                         |
| BUILD_PLC_LAB_RELEASE.bat                           | PASS; full checks then optimized release build and staging, exit 0                                                      |
| RUN_PLC_LAB_DEV.bat                                 | PASS; real native development app at http://127.0.0.1:1420/, Rust IPC v2 and DEVELOPMENT; clean closure exit 0          |
| RUN_PLC_LAB_RELEASE.bat                             | PASS; actual final copied release started and rendered correctly; exit 0 on normal closure                              |
| npm run check (through canonical validation)        | PASS; Prettier, Rust fmt, TypeScript, ESLint, clippy with warnings denied, frontend build and 37 tests                  |
| npm run test:desktop (through canonical validation) | PASS; actual debug Tauri/WebView2 startup and successful/rejected IPC; all panels, disabled controls and no page errors |
| Missing-release BAT fixture in a path with spaces   | PASS; expected nonzero and exact build-first explanation; no automatic build                                            |
| Missing-tool checks BAT test                        | PASS; expected nonzero and CHECKS FAILED; no false success marker                                                       |
| RUN_ALL_CHECKS.bat while a release window was open  | PASS; expected exit 1 and clear close-window instruction before any rebuild                                             |
| Release source/copied SHA-256 comparison            | PASS; final executable bytes match                                                                                      |
| Git artifact policy/source tracking                 | PASS; generated output ignored, BAT/helper/documentation sources tracked/staged                                         |
| Documentation local links and Git whitespace checks | PASS                                                                                                                    |

The 37 tests consist of 18 frontend, 11 substantive Rust and 8 architecture/tooling tests. Native smoke is additional real application acceptance. Staging unit tests use ordinary byte fixtures and do not claim to be executable application tests; the real release above was separately built and launched.

During validation, initial runs correctly returned failure for an unformatted helper, a Rust dependency procedural-macro loading error and a rebuild attempted while DEV was open. Formatting was corrected, only the affected generated release macro artifacts were rebuilt, and the locked-executable case led to the explicit preflight guard. A BAT run disrupted by line-ending normalization during execution was rerun with stable files. Final required validation passes; no compiler/security setting was bypassed and no unsuccessful build was staged.

## Architectural audit

- PASS: repository is initialized as Git on main; no remote was configured.
- PASS: core Rust dependency boundaries exclude React/Tauri and application/renderer code.
- PASS: no frontend PLC evaluator, source parser, PLC VM, timers, SimulationClock, process/network model or protocol implementation was introduced.
- PASS: 21 existing core/language reservation files are byte-identical to their accepted M0 state, including the diagnostics vocabulary already present before this task.
- PASS: packages/ipc remains a typed transport/contract layer with response validation and explicit errors.
- PASS: LabShell remains presentation/application-shell code. The only UI addition is build context appended to the existing Diagnostics information line; layout/styles were not redesigned.
- PASS: application/core versions come from Cargo metadata; IPC version and DTO declarations come from Rust; development/release derives from the actual debug_assertions build configuration. The required new transport field advances the IPC contract to v2 and rejects old v1 requests explicitly. No IR semantics changed.
- PASS: UI principles cover visible results, simultaneous code/values/process understanding, authoritative visualization, functional physical consequences, presentation density over one engine and future causal explanation. No causality engine was implemented.

No architectural violations were found, so no architectural correction or redesign was needed.

## Known limitations

- Native human-tooling validation covers Windows x64 on this machine. The release requires an installed Microsoft Edge WebView2 Evergreen Runtime and compatible Windows system libraries; WebView2 is not bundled. This is a local executable, not an installer or universally portable package.
- First-time development use still requires Node/npm, the pinned Rust toolchain, MSVC C++ tools/Windows SDK and WebView2. The standard workflow requires the default native Cargo target and target directory used by the existing smoke test.
- The quality workflow is configured, but no remote CI run occurred in this local repository. Runtime execution remains unavailable by accepted M0 scope.

## Human Test Ready

**HUMAN TEST WORKFLOW: PASS**

## M0 Gate

**M0 CLOSEOUT: PASS**

Stopped at M0. No M1 implementation was performed.

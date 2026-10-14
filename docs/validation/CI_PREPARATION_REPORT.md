# M1C-CI preparation report

Historical preparation report. Subsequent owner-authorized publication and actual independent execution passed in run 38533787674 on 2026-10-14. Current authoritative status: [M1C INDEPENDENT VALIDATION: PASS / M1C GATE: PASS](M1C_INDEPENDENT_VALIDATION.md). Preparation-only statements and the original HOLD below describe the earlier task and remain preserved.

Date: 2026-10-14, Europe/Warsaw. Scope: CI-001 through CI-016. CI files are prepared and locally checked; no external run, remote creation, publication, authentication or dispatch was performed. M1C implementation review is accepted, local validation remains host-policy blocked, and M2 is not authorized.

## Completed

Prepared one canonical GitHub Actions workflow with independent Windows and Linux jobs. Reused existing validation commands through a small repository-owned logger/orchestrator, added fail-closed hosted Windows prerequisites and scoped failure artifacts, tested YAML/command/failure handling, and documented platform-specific gate requirements. No PLC behavior or existing test assertion changed.

## Files changed

| File                                     | Change                                                                                                           |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| .github/workflows/validation.yml         | New push/PR/manual/reusable workflow, Windows canonical and Linux core jobs                                      |
| .github/workflows/quality.yml            | Existing workflow becomes a manual compatibility entry delegating to validation.yml; no duplicate automatic runs |
| scripts/ci.mjs                           | Repository command plans, declared Rust setup, logging/nonzero failure propagation and local status command      |
| scripts/ci-windows-prerequisites.ps1     | Hosted-only Visual Studio/SDK/WebView2 prerequisite checks and supported conditional Runtime setup               |
| tests/architecture/ci.test.mjs           | Seven new YAML/configuration and real Node/npm orchestration tests                                               |
| package.json                             | Added ci:windows, ci:core, ci:status and test:ci; existing canonical commands/dependencies unchanged             |
| docs/DEVELOPMENT.md                      | CI usage/reference and accepted M1C review/current validation status                                             |
| docs/validation/CI_VALIDATION.md         | Lane model, stages, native prerequisites, security/artifacts and gate policy                                     |
| docs/validation/CI_PREPARATION_REPORT.md | This report                                                                                                      |

No dependency, lockfile, toolchain pin, core semantic/runtime file, existing test, frontend/application code, IPC contract or release artifact changed. The pre-existing staged/untracked repository state was preserved; no commit, branch or remote change was made.

## Windows CI job

Runner family: windows-2022, x64. Time limit: 45 minutes. Setup uses official full-SHA-pinned checkout/setup-node/upload-artifact actions, Node major 24, npm ci and rustup installation of the rust-toolchain.toml channel/profile/components (1.99.0/minimal/rustfmt/clippy). Actual Node patch/platform/revision and command results are captured. No cache is used.

Exact validation sequence:

1. Checkout with persisted credentials disabled.
2. Configure Node 24 according to the existing CI policy and >=24 package engine.
3. `node scripts/ci.mjs dependencies` -> logged `npm ci`.
4. `node scripts/ci.mjs setup-rust` -> exact declared Rust/components install and version output.
5. `node scripts/ci.mjs windows-prerequisites` -> hosted-only C++/SDK/WebView2 checks/setup.
6. `npm run ci:windows`, which executes:
   - `npm run check`: formatting/Prettier, Rust fmt, TypeScript, ESLint, workspace/all-target clippy with -D warnings, architecture/tooling tests, frontend tests, all Rust workspace tests and production frontend build.
   - `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core`: all focused debug semantic/runtime tests.
   - `node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core`: all optimized runtime and clock tests.
   - `npm run test:desktop`: unchanged real Tauri debug build and native IPC smoke.
7. Always attempt scoped artifact upload after success/failure, retaining available stage logs/results and native smoke files for 14 days.

Canonical check/native sequencing is preserved; focused/optimized requirements are added between the original commands. A failed stage returns nonzero, is recorded FAIL and leaves subsequent stages NOT_RUN. There is no continue-on-error or test logic duplicated in YAML.

## Linux core job

Runner family: ubuntu-24.04, x64. Time limit: 30 minutes. Checkout/Node/lockfile/Rust setup follows the same policy. `npm run ci:core` executes:

1. `node scripts/rust.mjs fmt --all -- --check`
2. `node scripts/rust.mjs clippy --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core --all-targets -- -D warnings`
3. `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core`
4. `node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core`
5. `npm run test:architecture`
6. Always attempt scoped CI log/result upload.

Portable architecture tests run on Linux; the two existing Windows BAT-specific tests retain their existing platform guard and remain covered by Windows. Linux selects only the headless core packages and does not need desktop GTK/WebKit build prerequisites. Linux cannot substitute for native Windows validation.

## Native desktop smoke

The workflow attempts the actual unchanged `tests/e2e/desktop-smoke.mjs` via `npm run test:desktop`. It launches target/debug/plc-lab-desktop.exe and attaches to its real WebView2 CDP endpoint. It retains the existing successful/rejected Rust IPC, shell, disabled controls, layout and screenshot assertions.

Hosted prerequisite setup checks supported Visual Studio x64 C++ tools and Windows SDK, then checks documented WebView2 registry versions. If the Runtime is absent, the script uses only Microsoft's official Evergreen bootstrapper, validates its Microsoft Authenticode signature, runs supported silent installation and verifies registration. The script refuses local execution outside an explicitly started GitHub Actions Windows job; it was not executed on the owner machine.

Native hosted compatibility has not been tested externally. No genuine hosted-runner limitation is claimed from this preparation task. A future actual hosted limitation must be documented as `NATIVE DESKTOP CI: HOSTED RUNNER LIMITATION`, fail/hold the application gate and preserve the test for an explicitly approved Windows validation VM. Compilation, browser-only tests or older release launches cannot replace it.

See [CI_VALIDATION.md](CI_VALIDATION.md) and [Microsoft WebView2 deployment guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).

## Cross-platform REAL tests

Both Windows and Linux execute the existing full plc-runtime suites in debug and release. These include fixed binary32 bit patterns/subnormals/canonical zero, operator nesting and non-finite-result rejection, plus repeated-program/fault/reset determinism tests. Same test source and assertions run on MSVC Windows x64 and GNU Linux x64. Any bit-pattern mismatch fails; no expected-value normalization, soft-float or f32 redesign was added.

The logs will provide actual per-OS/profile results for the exact commit after an owner-authorized external run. No cross-platform PASS evidence was collected in this preparation-only task.

## Security

SECURITY SETTINGS MODIFIED: NO

CI WRITE PERMISSIONS REQUIRED: NO

SECRETS ADDED: NO

Workflow token permissions are contents: read. Checkout credentials are not persisted. Actions use verified full commit hashes. No signing, deployment, trust-store, antivirus, Smart App Control or Defender changes are part of the workflow. No authentication was performed; public official action refs/documentation were inspected anonymously. No caches or prebuilt PLC Lab binaries can act as test evidence.

Artifact paths are limited to CI `.log`/result.json files and native.log/result.json/desktop.png. Certificate stores, user profiles, browser profiles, .env files, unrelated system data and target/release executables are excluded. The supported Microsoft Runtime installer, if needed, is an application prerequisite with a valid-signature check, not an arbitrary executable workaround.

## Local validation

Only executed outcomes from this task are listed:

| Command/check                                                                  | Actual result                                                                                                                                                                                       |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| npm.cmd run test:ci                                                            | PASS: seven tests; YAML/trigger/permissions/pinning/artifact guards, command coverage, Rust pin parsing, failure/process-creation handling, actual Node/npm process success and CLI nonzero failure |
| npm.cmd run test:architecture                                                  | PASS: 17/17 tests, including the seven new CI tests and all existing Windows tooling/boundary checks                                                                                                |
| npm.cmd run test:frontend                                                      | PASS: 18/18 tests                                                                                                                                                                                   |
| npm.cmd run typecheck                                                          | PASS                                                                                                                                                                                                |
| node --check scripts/ci.mjs                                                    | PASS                                                                                                                                                                                                |
| .\node_modules\.bin\eslint.cmd . --max-warnings 0                              | PASS                                                                                                                                                                                                |
| node scripts/rust.mjs clippy --workspace --all-targets --locked -- -D warnings | PASS static analysis; not claimed as executed PLC tests                                                                                                                                             |
| PowerShell Language.Parser.ParseFile of scripts/ci-windows-prerequisites.ps1   | PASS: zero parse errors; setup body was not run locally                                                                                                                                             |
| npm.cmd run build                                                              | PASS: TypeScript and production Vite assets; 20 modules, unchanged frontend asset hashes                                                                                                            |
| npm.cmd run ci:status                                                          | PASS: requested local-host notice and document pointers                                                                                                                                             |
| npm.cmd run format:check                                                       | PASS after formatting new/updated source, workflow and documents; includes Prettier YAML syntax and Rust fmt                                                                                        |
| Protected source/test hash comparison                                          | PASS: all 67 existing files under core/languages/packages/desktop/tests remain byte-identical; only the new CI test file was added there                                                            |
| git remote -v                                                                  | Empty output; no remote configured                                                                                                                                                                  |

There are 35 unique passing local tests (17 architecture/tooling and 18 frontend). test:ci's seven overlap the architecture count and are not counted twice. The CI orchestration tests intentionally execute ordinary local Node/npm child processes to check logging/exits; they do not mock or replace the PLC, clock or native acceptance tests.

An initial logging test exposed an undefined optional field being omitted by JSON serialization. The result record now uses explicit null for absent launch errors; the unchanged test expectation passes. No PLC assertion was changed to obtain success.

No blocked Rust test executable or native smoke launch was retried. No full local npm run validate, hosted job, Linux job or runner prerequisite installation is claimed executed by this task. YAML syntax/configuration checks are local preparation evidence, not a complete GitHub scheduler/schema or hosted-execution proof.

## Acceptance criteria

| Task                         | Preparation result                                                                                             |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------- |
| CI-001 Canonical commands    | PASS: check/validate/test:desktop/rust wrapper unchanged; new runner calls repository commands                 |
| CI-002 Workflow events       | PASS: push, pull_request and workflow_dispatch; additional workflow_call for compatibility                     |
| CI-003 Windows stages        | PASS as preparation: all required stages represented; actual external execution pending                        |
| CI-004 Focused debug/release | PASS as preparation: complete required packages/profiles, no compile-only substitution                         |
| CI-005 Linux core lane       | PASS as preparation: core fmt/clippy/debug/release/portable architecture; native excluded explicitly           |
| CI-006 REAL evidence         | PASS as preparation: unchanged full deterministic tests on both lanes; results pending                         |
| CI-007 Native compatibility  | PARTIAL: real unchanged test and supported prerequisite setup prepared; hosted execution unverified            |
| CI-008 Artifacts             | PASS: scoped logs/results/screenshot, always retained where produced, 14-day retention                         |
| CI-009 Security              | PASS: read-only workflow, no secrets/signing/security changes, SHA-pinned actions                              |
| CI-010 Cache policy          | PASS: no cache; clean setup is the only planned route                                                          |
| CI-011 Lane documentation    | PASS: local/Windows/Linux responsibilities and non-substitution rules                                          |
| CI-012 Gate policy           | PASS: all required actual executions and Windows acceptance are mandatory; blocked is not pass                 |
| CI-013 Local status command  | PASS: explicit engineering command; UI untouched                                                               |
| CI-014 Scope audit           | PASS: protected source/assertions unchanged; no M2 or security changes                                         |
| CI-015 Local validation      | PASS: allowed static/configuration/frontend/build/tooling checks actually executed; hosted outcome not claimed |
| CI-016 Remote state          | PASS: no remote, push, authentication, credentials or invented repository URL                                  |

## Assumptions and known limitations

Use Node major 24, matching the prior workflow and package engine. Keep the exact existing Rust policy and lockfiles. Choose maintained explicit x64 runner image families; their patch contents and Node patch revisions can change and must be read from actual run metadata. Native/CDP behavior, the guarded Runtime bootstrapper setup and GitHub workflow execution remain unverified externally. Local static checks do not establish those outcomes.

## Git remote

REMOTE PRESENT: NO

No remote was created, no files were pushed, no credentials requested and no repository URL invented. An external publication/run requires explicit owner authorization in a future task.

## CI state

CI WORKFLOW: READY FOR REMOTE EXECUTION

This means prepared and locally checked, not externally executed or passing.

## Next dependency

Owner-authorized publication/execution in a chosen repository or explicitly approved independent validation environment, followed by review of both actual CI lanes and native Windows evidence. No further local policy diagnosis is needed in this task.

## Project gate

M1C IMPLEMENTATION REVIEW: PASS

M1C VALIDATION: HOST POLICY BLOCKED

M1C GATE: HOLD

Stopped. M2 has not begun.

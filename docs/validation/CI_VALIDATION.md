# Independent canonical validation lanes

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [BUILD_AND_RUN.md and the CI workflow](../BUILD_AND_RUN.md). Kept for engineering history.

Status: both independent lanes actually executed and passed for code SHA a4a22f19c7bcdbd1c1d78a38804c29e15da32828 in run 38533787674 on 2026-10-14, including native Windows smoke and cross-platform REAL assertions. M1C implementation review, independent validation and gate are PASS; see the authoritative [gate record](M1C_INDEPENDENT_VALIDATION.md). Owner-host validation remains historically HOST POLICY BLOCKED; policy is unchanged. M2A requires technical-lead review and explicit authorization. No PLC, TON, clock, budget, memory or REAL semantics changed.

## Validation model

| Lane                | Purpose                                                                                                        | Boundary                                                                                                                                                                       |
| ------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| LOCAL OWNER MACHINE | Development, human UI testing and release human testing where permitted                                        | Smart App Control may block newly generated unsigned executables. See [diagnosis](M1C_WINDOWS_CODE_INTEGRITY.md). Static/build success does not substitute for executed tests. |
| WINDOWS CI          | Canonical Windows Rust execution, debug/optimized runtime and clock checks, frontend/tooling, Tauri native IPC | Windows x64, pinned Rust and Node 24. The real existing WebView2/CDP smoke must launch and pass.                                                                               |
| LINUX CI            | Independent headless semantic/runtime validation and cross-platform REAL evidence                              | Linux x64 core only. Does not replace Windows application/native acceptance.                                                                                                   |

No lane silently substitutes for another's platform-specific requirements. A blocked or unrun requirement remains incomplete.

## Workflow and toolchain

`.github/workflows/validation.yml` supports push, pull_request and workflow_dispatch. workflow_call additionally supports the old `.github/workflows/quality.yml`, now a manual compatibility wrapper; automatic events belong only to the new workflow to avoid duplicate pipelines.

Windows uses windows-2022 and Linux uses ubuntu-24.04. These explicit image families avoid windows-latest/ubuntu-latest family migration; their maintained image patch revisions still change. Both image families executed this project's recorded code SHA in run 38533787674. GitHub's published Windows image includes Visual Studio C++ tools, Windows SDK and rustup. [Runner image manifest](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)

Both jobs use official actions pinned to verified full commit hashes: checkout v6, setup-node v6 and upload-artifact v7. Node major 24 follows the existing quality workflow and the declared >=24 engine; the selected patch is recorded in CI result metadata. Node dependency installation is `npm ci` against the existing lockfile. No dependency or lockfile changes were needed.

`node scripts/ci.mjs setup-rust` reads rust-toolchain.toml (currently 1.99.0, minimal, rustfmt/clippy), installs that exact channel/components using the runner's rustup and records Cargo version output. The repository override selects the installed pin; no global default, semantic Rust flags, or floating-point behavior is changed. Unsupported/incomplete toolchain-file syntax fails explicitly rather than guessing. [Rustup toolchain overrides](https://rust-lang.github.io/rustup/overrides.html)

## Repository-owned Windows stages

Setup: checkout with persisted credentials disabled -> Node 24 -> logged npm ci -> logged pinned Rust setup -> hosted-only Windows prerequisites. `npm run ci:windows` then executes, in order:

1. `npm run check`: Prettier + Rust fmt; TypeScript; ESLint + workspace/all-target clippy with warnings denied; architecture/tooling tests; frontend tests; all Rust workspace tests; TypeScript and production frontend build.
2. `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core`: complete focused debug tests, including FB validation, TON timing/rollback, determinism, budgets, arithmetic/REAL and clock overflow.
3. `node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core`: complete optimized runtime and clock tests, including existing TON and fixed REAL expectations.
4. `npm run test:desktop`: the original `npm run build:desktop` -> real Tauri debug build with production frontend -> unchanged `tests/e2e/desktop-smoke.mjs`.

Stages 1 and 4 preserve the existing `npm run validate` sequence; focused/optimized tests are additional requirements between them. The YAML delegates to repository scripts, not duplicated test implementations. Each stage must return success; a failure propagates a nonzero CLI/job result, retains stdout/stderr/exit diagnostics and leaves subsequent stages explicitly NOT_RUN. Job timeouts remain failures/incomplete evidence. Neither continue-on-error nor an unconditional success wrapper is used.

## Linux core stages

Setup uses the same checkout, Node/dependency and pinned Rust policy. `npm run ci:core` executes:

1. `node scripts/rust.mjs fmt --all -- --check`
2. `node scripts/rust.mjs clippy --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core --all-targets -- -D warnings`
3. The same complete focused debug command as Windows.
4. The same complete optimized runtime/clock command as Windows.
5. `npm run test:architecture`: portable core dependency/configuration/tooling checks and CI configuration tests. The two existing Windows BAT-specific tests retain their original platform guard and are skipped on Linux; they remain required/executed in Windows CI. No assertion was changed.

No Tauri/native build or Windows WebView2 smoke is run on Linux. Core checks select the four semantic/runtime/clock packages, avoiding an unnecessary Linux desktop GTK/WebKit installation. Cargo metadata used by architecture tests does not execute desktop build scripts.

## Native desktop prerequisites and smoke

The existing smoke file remains byte-identical. It launches target/debug/plc-lab-desktop.exe, assigns an isolated WebView2 profile and local CDP port, checks seven panels/disabled controls, real Rust IPC success/rejection, unavailable execution, browser errors and layout, and writes native.log/result.json/desktop.png. No browser-only or mock replacement exists.

`scripts/ci-windows-prerequisites.ps1` runs only when GITHUB_ACTIONS=true and RUNNER_OS=Windows. It verifies Visual Studio x64 C++ tools via vswhere and Windows SDK availability, and rejects a custom CARGO_TARGET_DIR because the smoke expects the default path.

The script checks Microsoft's documented per-machine/per-user WebView2 `pv` registrations. If a valid Runtime is missing, it downloads only Microsoft's official Evergreen bootstrapper, requires a valid Microsoft Authenticode signature, performs supported silent installation and verifies the registration afterward. This is ordinary application-prerequisite setup on an explicitly started runner, not security-policy modification or an unsigned test workaround. It is not executed on the owner machine by this task. [Microsoft deployment reference](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution), [Microsoft bootstrapper sample](https://github.com/MicrosoftEdge/WebView2Samples/blob/main/SampleApps/WebView2APISample/AppWindow.cpp)

No runner can be declared compatible without an actual external run. Prerequisite/setup/native launch/CDP/assertion failures fail the job; the smoke is never disabled or marked passed from compilation. If execution evidence later establishes a genuine hosted-runner limitation, record `NATIVE DESKTOP CI: HOSTED RUNNER LIMITATION`, retain the unchanged test, and arrange an explicitly approved Windows validation VM. Such a limitation cannot make M1C PASS. Recorded status is **native hosted execution PASS for run 38533787674**, with real WebView2/CDP/Rust IPC artifacts reviewed.

## Cross-platform REAL evidence

Both OS jobs execute all plc-runtime tests in debug and release, including:

- real_results_have_fixed_binary32_bits_subnormals_and_canonical_zero
- real_operation_nesting_is_not_reassociated_or_constant_folded
- real_nonfinite_arithmetic_cannot_enter_authoritative_memory
- Existing repeated-program, fault and reset determinism tests.

The same existing test source and exact bit-pattern assertions run on MSVC Windows x64 and GNU Linux x64. A mismatched bit pattern fails the lane; no normalization, soft-float, new f32 semantics or expected-value changes are introduced. Passing both lanes begins cross-platform evidence for these targets/profiles, not a claim about untested CPU architectures or arbitrary floating-point environments.

## Failure artifacts and security

The CI runner writes per-stage stdout/stderr `.log` files and `result.json` under artifacts/ci. Metadata contains lane, OS, Node version, revision, process exits and stage outcomes. Failed stages are FAIL; later unexecuted stages are NOT_RUN. The pinned Rust setup/version output and canonical build/test output are retained. A catastrophic runner cancellation may leave RUNNING metadata; it is not interpreted as PASS.

Official upload-artifact steps use always(), so available evidence is retained after failure. Windows uploads only CI logs/results and the three explicit smoke files. Linux uploads only CI logs/results. No entire target/release binary, WebView2 browser profile, certificate store, owner profile, .env file or unrelated system inventory is uploaded. Missing optional screenshots/files warn without changing an earlier failure into success; early failures can occur before screenshot/native files are produced. Retention is 14 days.

Permissions are contents: read only, checkout does not persist credentials, no secrets/signing/deployment credentials are added, and no repository writes are required. Actions are full-SHA pinned. There are no caches: a fresh runner starts from checkout and lockfile installation, and stale compiled artifacts cannot satisfy execution requirements. No security policies, Defender settings, Smart App Control, trust stores or signing keys are changed. [GitHub security guidance](https://docs.github.com/en/actions/reference/security/secure-use)

## Gate policy and external execution

M1C implementation review is already PASS. Final M1C PASS requires actual executed evidence for all required semantic/runtime tests, debug tests, optimized tests, TON, SimulationClock, full core regressions and existing Windows application checks. Windows-specific checks must execute and pass either in this CI lane or another explicitly owner-approved Windows validation environment. Linux results cannot replace them. BLOCKED and NOT_RUN never mean PASS.

Preparing files or passing configuration tests proves no hosted test outcome. The owner subsequently authorized M1C-CI-RUN publication/execution. Private PLC-Lab has HTTPS origin/main and its push triggered run 38533787674. Both jobs/profile logs/native artifacts were reviewed for the exact code SHA before PASS was recorded. Later documentation-only commits identify the gate record separately and are not represented as the tested code revision. See [M1C_REMOTE_VALIDATION_REPORT.md](M1C_REMOTE_VALIDATION_REPORT.md).

## Local commands and limitations

`npm run test:ci` checks YAML syntax/selected workflow configuration, required command coverage, logging, failure propagation and real local Node/npm process creation. It is included in architecture tests. It does not execute PLC tests or simulate hosted actions. YAML checks use the already installed locked Prettier parser; GitHub scheduler/schema behavior still requires an actual run.

`npm run ci:status` prints:

```text
Local validation status:
HOST POLICY MAY BLOCK GENERATED EXECUTABLES
See docs/validation/M1C_WINDOWS_CODE_INTEGRITY.md and docs/validation/CI_VALIDATION.md.
```

The message is an explicit engineering command only and does not alter application UI or normal startup. Do not repeatedly invoke locally blocked PLC binaries as a substitute for external validation. Local results and the preparation audit are in [CI_PREPARATION_REPORT.md](CI_PREPARATION_REPORT.md).

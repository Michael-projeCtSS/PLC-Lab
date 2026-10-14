# Human testing on Windows

The project owner can test PLC Lab by double-clicking the four launchers in the repository root. Routine testing does not require typing development commands. Keep the terminal window open while testing: it explains the selected mode and preserves useful errors. At the end, press a key to close it.

## Fast development test

Double-click **RUN_PLC_LAB_DEV.bat**.

This opens the real Tauri development application from the current source tree. It checks Node/npm and the pinned Rust tools, reuses valid installed dependencies, and runs npm ci only when dependencies are missing, incompatible, or differ from the lockfile. Vite and Rust build output remain visible. A first native build can take several minutes.

Use [the human test checklist](HUMAN_TEST_CHECKLIST.md). Close the application window when finished. The Diagnostics panel must show DEVELOPMENT with application/core versions and the IPC contract version supplied by Rust.

## Validate the project

Double-click **RUN_ALL_CHECKS.bat** before a human testing session.

It runs the canonical npm run validate command: npm run check followed by npm run test:desktop. These cover formatting, TypeScript, ESLint, frontend tests, Rust fmt/clippy/tests, architecture/tooling tests, the frontend production build, and actual native application/IPC acceptance. A temporary desktop window may appear during the automated smoke test and close automatically.

Close any existing DEV and RELEASE application windows before validation or a release build. Windows locks running executable files, and the native smoke test rebuilds the debug executable. The launchers check for running PLC Lab applications and explain which window must be closed; they never terminate your application automatically.

The terminal ends with **ALL CHECKS PASSED** only when every required command succeeds. **CHECKS FAILED** means the validation failed; read the first failing command and its error output. Do not treat a successful-looking application window as evidence that failed automated checks passed.

## Build a production-like executable

Double-click **BUILD_PLC_LAB_RELEASE.bat**.

It runs the same complete validation first, then builds the actual Tauri application in optimized release mode, with embedded frontend assets and no installer. A failed validation/build/staging operation returns an error and never reports a new release ready. The first optimized build can take substantially longer than a development rebuild.

After success the terminal prints the full path:

```text
C:\Users\mjgom\Documents\PLC_Student\releases\current\PLC Lab.exe
```

The path is resolved from this checkout, so moving the repository changes the printed absolute path automatically. Staging audits executable DLL imports, copies any required local DLLs alongside the executable, and verifies SHA-256 equality with the actual build outputs. The executable is copied without modifying its bytes. A fresh artifact set replaces current/ only after copying and verification succeed. Close an already running release before rebuilding it.

current/release-manifest.json records source executable/version, release profile, system dependencies, and file hashes. current/RUNTIME_REQUIREMENTS.txt states runtime requirements. A failed build leaves the previous successful release available; it does not mean that previous release includes new source changes.

## Test the release build

Double-click **RUN_PLC_LAB_RELEASE.bat**.

This opens releases/current/PLC Lab.exe and waits for you to close it. It uses the latest successfully staged local release, which may be older than the current source tree. It does not build or install dependencies and does not require Node or Rust to launch the existing executable.

If no release exists, it explains:

```text
No local PLC Lab release exists. Run BUILD_PLC_LAB_RELEASE.bat first.
```

The Diagnostics panel must show RELEASE, application/core versions, and the Rust-supplied IPC contract version. Complete the checklist again for the release; compilation alone does not prove startup, IPC, or layout correctness.

## Development, release, and installer

| Kind              | What is tested                                                                | Requirements                                                         |
| ----------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Development       | Current source, Vite development server, Rust debug profile                   | Installed development tools; terminal stays open                     |
| Release           | Last successfully compiled optimized native executable with embedded assets   | Compatible Windows x64 and installed WebView2; no development server |
| Installer/package | Future distribution package that can install the app and manage prerequisites | Not produced by the current workflow                                 |

This local executable is **not a universally portable package**. The current Windows x64 application relies on the installed Microsoft Edge WebView2 Evergreen Runtime and Windows system libraries. Any required local DLLs are staged with it; WebView2 itself is not bundled. Other machines still need the applicable runtime prerequisites. See [Tauri Windows distribution](https://v2.tauri.app/distribute/windows-installer/) and [Microsoft WebView2 distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution) for runtime/installer requirements. Installer creation, signing, and distribution are outside this M0 task.

## First-time prerequisites and errors

The three development/validation/build launchers require Node.js 24+ with npm, the repository-pinned Rust toolchain with rustfmt and clippy, MSVC Desktop development with C++ and Windows SDK, and WebView2. The current checkout already has the needed environment, including its optional workspace-local Rust installation. Another machine may need a one-time tool installation described in [development](DEVELOPMENT.md) and [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

The launchers work from any current working directory and with spaces in the repository path. They never alter the system PATH. Node dependency installation uses the Windows system CA store while keeping TLS certificate verification enabled. A missing/incompatible tool, occupied development port, failed check, or blocked executable is reported through visible output and a nonzero exit code. Do not change Windows security settings to force a blocked executable to run; report the exact error.

The standard human workflow uses the default Cargo target directory and native host target because the existing desktop smoke test uses target/debug. Custom CARGO_TARGET_DIR or CARGO_BUILD_TARGET values cause a clear prerequisite error. Release staging is currently validated for Windows x64. No launcher downloads or bundles WebView2 automatically.

Human visual testing complements automated tests; it does not replace them. Keep findings with the application/build context and versions shown in Diagnostics, the test date, and the checklist item that failed.

## Permanent engineering workflow

Future milestones must maintain all four root launchers, their shared helpers, and the canonical validation commands. Extend [HUMAN_TEST_CHECKLIST.md](HUMAN_TEST_CHECKLIST.md) for newly approved behavior rather than replacing the M0 checks. Keep generated binaries/manifests in ignored releases/current/; keep launcher/helper/documentation source in Git.

For automated harnesses only, PLC_LAB_NO_PAUSE=1 suppresses the final keypress wait. It does not skip any dependency check, validation, build, or error handling. Normal double-click use leaves this variable unset.

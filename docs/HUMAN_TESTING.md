# Human testing on Windows

Routine testing needs no typed commands. Keep the terminal window of a BAT launcher open
while testing; it explains what runs and preserves errors.

## Start the release

Double-click **Launcher.exe** in the repository root and press **Start PLC Lab**
(**PLC Lab.exe** and **RUN_PLC_LAB.bat** start the release directly). All of them start
`releases/current/PLC Lab Desktop.exe`, the last successfully staged release, which may be
older than the current source. If no release exists:

```text
No local PLC Lab release exists. Run BUILD_PLC_LAB_RELEASE.bat first.
```

## Build the release

Double-click **BUILD_PLC_LAB_RELEASE.bat**. It runs the complete validation, builds the
optimized Tauri application, builds the root launcher and stages `releases/current/`
(application, knowledge pack, manifest with hashes). A failure never reports a new
release and leaves the previous one untouched. Close running PLC Lab windows first.

## Development application

Double-click **RUN_PLC_LAB_DEV.bat**: the Tauri development application from the current
source tree with hot reload. Dependencies are installed with `npm ci` only when they
differ from the lockfile.

## Validate

Double-click **RUN_ALL_CHECKS.bat** (`npm run validate`: formatting, TypeScript, ESLint,
clippy, frontend, architecture, Rust tests, production build and the native smoke test).
It ends with **ALL CHECKS PASSED** only when every command succeeds; **CHECKS FAILED**
otherwise.

## Prerequisites and errors

Node.js 24 with npm, the pinned Rust toolchain, Visual Studio C++ build tools with the
Windows SDK, and WebView2. Launchers work from any working directory and with spaces in
paths and never change the system PATH. Never change Windows security settings to force a
blocked executable to run; report the exact error (see
[development](DEVELOPMENT.md#windows-smart-app-control)).

For automated harnesses only, `PLC_LAB_NO_PAUSE=1` suppresses the final keypress wait.

Use the [checklist](HUMAN_TEST_CHECKLIST.md) for a manual session.

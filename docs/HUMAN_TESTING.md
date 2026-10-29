# Human testing on Windows

Routine testing needs no typed commands. Keep the terminal window of a BAT helper open
while testing; it explains what runs and preserves errors.

## Start the release

Double-click **PLC Lab.exe** in the repository root. It is the application itself (the
last successfully staged release, which may be older than the current source); it reads
its knowledge pack from `resources\` next to it. **RUN_PLC_LAB.bat** is only a fallback
that starts `PLC Lab.exe`. If no release exists, the BAT reports:

```text
PLC Lab.exe was not found next to this file. Run BUILD_PLC_LAB_RELEASE.bat first.
```

## Build the release

Double-click **BUILD_PLC_LAB_RELEASE.bat**. It runs the complete validation, builds the
optimized Tauri application and stages it as `PLC Lab.exe` in the repository root with
its `resources\` folder (knowledge pack, `product.json` with hashes), then checks that the
distribution contains exactly one application executable. A failure never reports a new
release. Close running PLC Lab windows first: `PLC Lab.exe` cannot be replaced while it
runs.

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
Windows SDK, and WebView2. `PLC Lab.exe` and the BAT helpers work from any working
directory and with spaces in paths and never change the system PATH. Never change Windows security settings to force a
blocked executable to run; report the exact error (see
[development](DEVELOPMENT.md#windows-smart-app-control)).

For automated harnesses only, `PLC_LAB_NO_PAUSE=1` suppresses the final keypress wait.

Use the [checklist](HUMAN_TEST_CHECKLIST.md) for a manual session.

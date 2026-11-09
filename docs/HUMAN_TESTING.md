# Human testing on Windows

Routine testing needs no typed commands. Keep the terminal window of a BAT helper open
while testing; it explains what runs and preserves errors.

**To test PLC Lab, double-click TEST_PLC_LAB.bat.** It starts the complete native
application from this checkout: the Tauri desktop window with the real Rust backend (PLC
runtime, ST compiler, hardware, process and network simulation, alarms, forces,
diagnostics, restarts). No browser and no signing certificate are involved. See
[native development test](#native-development-test-test_plc_labbat).

## Which BAT file?

| File                          | What it does                                                                                                                                                                                                                     |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **TEST_PLC_LAB.bat**          | Runs the latest native development application built from this checkout, uncommitted changes included (debug build, real Rust backend).                                                                                          |
| **UPDATE_PLC_LAB.bat**        | Builds the current commit and installs it as the root `PLC Lab.exe` (unsigned development build): backup, atomic install, hash check and a native check before it reports success. Refuses uncommitted work and signed releases. |
| **RUN_PLC_LAB.bat**           | Starts the existing root `PLC Lab.exe`, whatever version it is.                                                                                                                                                                  |
| **RUN_PLC_LAB_DEV.bat**       | The same as TEST_PLC_LAB.bat without the log file.                                                                                                                                                                               |
| **BUILD_PLC_LAB_RELEASE.bat** | Builds a signed production release (needs signing credentials).                                                                                                                                                                  |

Not sure which version is running? **About** shows the source commit, the build time,
the distribution (development run, unsigned development build, signed release) and the
code-signing state, read from the build and the staged `resources\product.json` (trusted
only when it names the same commit as the running executable).

## Update the root PLC Lab.exe (UPDATE_PLC_LAB.bat)

Commit your changes, close PLC Lab, then double-click **UPDATE_PLC_LAB.bat**. It checks
Node.js, the dependencies and the pinned Rust toolchain, compares the installed
`resources\product.json` with the repository HEAD (nothing to do when they match and the
files verify; `UPDATE_PLC_LAB.bat --force` rebuilds anyway), verifies the knowledge pack
archive and builds the release application. Before anything is replaced it starts the
new executable once to see whether Windows lets it run; a Code Integrity (Smart App
Control) block stops the update with the event and leaves the root unchanged. It then
copies the installed distribution to `artifacts\plc-lab-backups\` (the newest three are
kept), installs the new one with the existing staging pipeline (`stage-release.mjs
--development`, atomic with rollback), checks every hash and the source commit, and runs
the native smoke test on the installed `PLC Lab.exe`. Only then does it report the
installed commit and offer to start PLC Lab; if the native check fails, the verified
previous version is restored. A signed production release is never overwritten.

## Start the release

Double-click **PLC Lab.exe** in the repository root. It is the application itself (the
last successfully staged release, which may be older than the current source); it reads
its knowledge pack from `resources\` next to it. **RUN_PLC_LAB.bat** is only a fallback
that starts `PLC Lab.exe`. If no release exists, the BAT reports:

```text
PLC Lab.exe was not found next to this file. Run BUILD_PLC_LAB_RELEASE.bat first.
```

## Build the release

Configure trusted signing first using [Windows code signing](WINDOWS_CODE_SIGNING.md).
Double-click **BUILD_PLC_LAB_RELEASE.bat**. It runs signing preflight, the complete validation, builds the
optimized Tauri application and stages it as `PLC Lab.exe` in the repository root with
its `resources\` folder (knowledge pack, `product.json` with hashes), then checks that the
distribution contains exactly one application executable, verifies trusted executable
and manifest-catalog signatures, and installs the complete candidate with rollback. A failure never reports a new
release. Close running PLC Lab windows first: `PLC Lab.exe` cannot be replaced while it
runs.

Without publisher credentials the production build fails with `SIGNING NOT CONFIGURED`.
Use `npm run build:release:unsigned` for an explicitly labeled development build in a
separate checkout; it cannot replace a signed root release and may be blocked by Smart
App Control. To obtain a production executable from protected CI, extract its
`SIGNED-production` artifact and run `npm run release:install -- <extracted-directory>`.
This verifies and stages the signed application at the normal root path.

## Native development test (TEST_PLC_LAB.bat)

Double-click **TEST_PLC_LAB.bat**. It finds the repository from its own folder (spaces in
paths are fine), checks Node.js and the pinned Rust toolchain, installs JavaScript
dependencies with `npm ci` only when they differ from the lockfile, and runs the normal
Tauri development workflow (`npm run dev`). Cargo builds the development application
incrementally; when it is ready the PLC Lab window opens. The page inside the window is
served by Tauri's development server, but it is the native window with the Rust backend,
not a browser. Close the window to finish: the development server stops with it.

- Only one PLC Lab runs at a time. If `plc-lab-desktop.exe` or `PLC Lab.exe` is already
  open, the BAT says so and does nothing: both share one single-instance identity, so a
  second start would only hand over to the running window.
- The whole output of the last run is kept in `artifacts\test-plc-lab\last-run.log`, and
  the window stays open after a failure until you press a key.
- After a failed start the launcher reads the Windows Code Integrity log: it either names
  the exact file Windows refused to run (see [Smart App Control](#smart-app-control)) or
  states that Windows recorded no block, so the problem is a build or application error.

**RUN_PLC_LAB_DEV.bat** runs the same development application without the log file.

`node tests/e2e/dev-session-check.mjs` verifies a whole session on this machine: it starts
`TEST_PLC_LAB.bat` from another folder, attaches to the native window and checks the Rust
IPC, the Polish interface, a Free Lab ST program with `VAR RETAIN` and `FIRST_SCAN` in RUN,
real Enter/Backspace/undo editing during RUN, backend values, the alarm view, warm and
cold restarts with their diagnostics, switching between the light and dark themes with the
program unchanged, no uncaught JavaScript error during the session, and a clean close. Its
screenshots stay in `artifacts\dev-session-check`; a passing run removes its browser profile
and data, a failing run keeps them for diagnosis. With
`--exe target\debug\plc-lab-desktop.exe` it runs the same checks against an already built
development application instead of starting `TEST_PLC_LAB.bat`.

`npm run test:editor` types in the real Monaco editor of the native application
(Enter, Backspace, Delete, undo/redo, clipboard paste, 200 rapid edits) in English and
Polish and fails on any uncaught JavaScript error or a lost line
([P0 record](validation/P0_EDITOR_BLACK_SCREEN.md)).

`node tests/e2e/sequence-session.mjs` runs the sequence lab natively through
`TEST_PLC_LAB.bat` (with `--exe target\debug\plc-lab-desktop.exe` against a built
application, as `npm run test:desktop` and the Windows CI lane do): the inspector's initial
step, the reference ST program, a start refused by a permissive, the tank filling,
HOLD and RESUME, completion and history, the stuck-valve lab's step timeout with its
alarm, Polish, both themes, ST editing with Enter and Backspace and a clean close.

`npm run test:pou` (`node tests/e2e/pou-session.mjs`, part of `npm run test:desktop`)
runs the function block exercise natively: the required blocks in the task brief, the
starter told which function is missing, the pasted reference program with its own
FUNCTION_BLOCK and FUNCTION compiling and running, signatures and examples in the Library
tab, two instances of one block lighting two lamps independently and the function lighting
the third, instance memory (`Toggle1.On`) in the variable list, grading, the blocks moved
into a new project library document (renamed, built, graded and found again after
reopening the project), Polish, both themes, ST editing and a clean close. Screenshots stay in `artifacts\pou-session`.

`npm run test:screenshots` captures the real application in both themes at 1366x768,
1920x1080 and 2560x1440 (Home, program, process, hardware, commissioning; Polish) into
`artifacts\theme-screenshots` and fails when a page overflows or a label is clipped.

## Smart App Control

Smart App Control decides per executable file. The root `PLC Lab.exe` (a staged, unsigned
release build) and the development application `target\debug\plc-lab-desktop.exe` are
different files and can be treated differently; on the development machine the release
build is refused while the development build runs. A BAT file cannot lift a block of the
executable it starts, and PLC Lab never changes security settings. If Windows blocks the
development application too, the launcher prints the file and the Code Integrity event
(3033 or 3077). The legitimate option is then a signed production release (see
[Windows code signing](WINDOWS_CODE_SIGNING.md)) or another Windows machine whose policy
allows the development build.

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

## Rust toolchain preflight

The build requires the exact version in `rust-toolchain.toml` (currently 1.99.0,
Windows x64 host `x86_64-pc-windows-msvc`, minimal profile with rustfmt and clippy).
`scripts/toolchain.mjs` checks installations in this order:

1. Repository `.tools/cargo` and `.tools/rustup`.
2. Existing configured `CARGO_HOME`/`RUSTUP_HOME` (standard user homes for unset values).
3. Standard user `.cargo` and `.rustup` homes.

A `cargo.exe` proxy alone is insufficient. The selector checks the installed toolchain
files, then runs the actual compiler, Cargo, rustfmt, clippy-driver and rustdoc with a
60-second timeout per command (the first start after a reboot can be slow while security
software scans the toolchain; a timeout is reported as such). It verifies the compiler release/host, Cargo and rustdoc
release, and the formatter/clippy compiler commit. Cargo's formatter/clippy entry points
must also exist. It never calls rustup proxies to test availability. The selected
environment places the actual toolchain binaries first, sets explicit compiler/doc paths,
pins `RUSTUP_TOOLCHAIN` and disables automatic installation with `RUSTUP_AUTO_INSTALL=0`
([rustup documentation](https://rust-lang.github.io/rustup/environment-variables.html)).

If no healthy exact pin exists, the BAT stops before dependency installation or
compilation with **PLC LAB BUILD BLOCKED**, the pin and missing/unusable paths. A broken
local installation does not prevent use of a healthy user installation. No toolchain is
downloaded or repaired by preflight. CI retains its separate, explicit pinned Rust setup
and uses the same parser and health selection afterward.

On a machine without a security block, install the declared pin/components through the
normal Rust setup before retrying. If Windows reports access denied or a virus/PUA error
during Rust installation, stop repeated attempts and retain the source/destination and
Windows error. Check the active antivirus's detection history for that exact file; do
not disable protection, change ACLs or exclude the repository. See the
[local recovery evidence](validation/LOCAL_BUILD_RECOVERY.md) for this machine's block.

Successful release builds verify the staged distribution and print **PLC LAB RELEASE
BUILD: SUCCESS**, the absolute root `PLC Lab.exe` path and the double-click instruction.

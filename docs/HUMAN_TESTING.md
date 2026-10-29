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

## Rust toolchain preflight

The build requires the exact version in `rust-toolchain.toml` (currently 1.99.0,
Windows x64 host `x86_64-pc-windows-msvc`, minimal profile with rustfmt and clippy).
`scripts/toolchain.mjs` checks installations in this order:

1. Repository `.tools/cargo` and `.tools/rustup`.
2. Existing configured `CARGO_HOME`/`RUSTUP_HOME` (standard user homes for unset values).
3. Standard user `.cargo` and `.rustup` homes.

A `cargo.exe` proxy alone is insufficient. The selector checks the installed toolchain
files, then runs the actual compiler, Cargo, rustfmt, clippy-driver and rustdoc with a
five-second timeout per command. It verifies the compiler release/host, Cargo and rustdoc
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

# ADR-0026: Launcher window and typed project hand-over

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-28
- Amends: ADR-0025 (root launcher)

## Context

`PLC Lab.exe` only started the release. Users need one obvious entry point that shows
whether the installation is usable (application, knowledge pack, WebView2, data folder),
opens projects (also by double-clicking a `.plclab` file with `Launcher.exe "<file>"`),
explains problems and offers diagnostics, without a terminal, development tools, network
access or administrator rights. Opening a project in an already running PLC Lab must not
start a second instance, and command-line text must never become a command.

## Decision

- `Launcher.exe` in the installation root is the primary entry point: a native Win32
  window (`winsafe` safe wrappers, no `unsafe` in PLC Lab code, no web view) in the
  `apps/launcher` crate. It shows the version, an overall status, the recent project,
  the knowledge pack status and four installation checks, and offers Start PLC Lab,
  Open Recent, Open Project, Manage Knowledge Packs (opens the user knowledge folder),
  Diagnostics (read-only report with Copy) and Open Data Folder. It closes after it
  started the application. Every path is resolved from its own executable folder and the
  data folder (`PLC_LAB_DATA_DIR` or `%APPDATA%\dev.plclab.desktop`); never from the
  working directory. Command-line modes: `--start`, `--diagnostics` (report on stdout)
  and one project file; anything else is refused with a usage message.
- The launcher reads only trusted data written by the application: `recent.json` in the
  data folder (project id, name, time) and the release manifest. It validates a project
  file (extension, size limit, project parser) before handing it over.
- Hand-over: the launcher starts `releases/current/PLC Lab Desktop.exe` with exactly
  `--open-project <canonical path>` as separate process arguments (no shell). The
  application parses arguments only with `startup::open_request` (that option followed by
  a `.plclab` path). A second instance forwards its arguments through the existing
  single-instance plugin to the running one, which applies the same parser. The request
  is resolved by the desktop use case `DesktopApp::request_open`: a file in the projects
  folder opens in place, any other file is validated and imported as a copy; failures
  become an explanation in the window. The web view pulls the result with the versioned
  `take_open_request` command (IPC contract 8, additive) after an `lab-open-request`
  event. No path or argument reaches the web view, and nothing is executed.
- `PLC Lab.exe` stays as a compatibility alias that starts the release directly. Error
  messages of both use the native message box through `winsafe` (replacing the system
  PowerShell message box of ADR-0025). `PLC_LAB_LAUNCHER_NO_DIALOG` sends them only to
  standard error for automated tests.
- Both launchers embed an application manifest: `asInvoker`, Common Controls 6, system
  DPI awareness.
- Optional ZIP installation of knowledge packs from the launcher is not implemented;
  "Manage Knowledge Packs" opens the trusted user folder, where the application already
  discovers and validates packs (data only).
- The product version has one source: the Cargo workspace version (`0.2.0-dev`), shared
  by all crates, the npm packages and, through Cargo, the Tauri bundle.

## Alternatives considered

A Tauri/React launcher (rejected: a second web view and runtime only to start the first);
forwarding raw arguments to the application (rejected: untyped and unbounded); detecting
a running instance by process name (rejected: the single-instance plugin already owns
that, and the launcher only shows a hint based on its window); writing projects to a
shared folder for the running application to poll (rejected: unnecessary file protocol).

## Consequences

The launcher depends on `winsafe` (no transitive dependencies) on Windows only. The
"already running" hint relies on the single-instance plugin's window names; if they
change, the hint disappears but starting still works. Hosted Windows CI drives the real
window with window messages (`tests/e2e/launcher-smoke.mjs`), because this repository's
development machine blocks freshly built binaries (Smart App Control) for full release
builds.

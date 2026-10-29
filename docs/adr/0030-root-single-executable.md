# ADR-0030: The root `PLC Lab.exe` is the application (single-executable distribution)

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29
- Supersedes: ADR-0025 (root launcher and `releases/current/`), the launcher window of
  ADR-0026. The typed project hand-over of ADR-0026 is kept and moves into the application.

## Context

Users start PLC Lab from the product (repository) folder. The earlier distribution had a
root `Launcher.exe` window and a `PLC Lab.exe` alias that both started a second
executable, `releases/current/PLC Lab Desktop.exe`, which read its knowledge pack from
`releases/current/knowledge-packs/`. Users had to choose between several executables and
BAT files, and the application could only be reached through a wrapper.

## Decision

- **One application executable.** The Tauri release binary (`plc-lab-desktop`) is staged
  as `PLC Lab.exe` in the product root: the product name with its space. It is the
  application itself and starts no other executable. The distribution is `PLC Lab.exe`,
  `resources/` and the optional `RUN_PLC_LAB.bat` (which only starts
  `"%~dp0PLC Lab.exe"`). A staged distribution must contain exactly one application
  executable; `scripts/distribution.mjs` checks this and every file hash.
- **Data beside the executable.** The application reads shipped data from `resources/`
  next to its own executable (`std::env::current_exe()`), never from the working
  directory: `resources/knowledge-packs/` (the verified content of the knowledge pack
  archive) and `resources/product.json` (version, DLL audit, SHA-256 of the executable
  and of every file). User data stays in the application data folder. Knowledge files are
  not embedded in the binary.
- **Launch and hand-over in the application.** Single instance stays (the Tauri plugin).
  `PLC Lab.exe "<file.plclab>"` (and `--open-project <file>`) is parsed by the typed
  parser only; a second instance hands its arguments to the running one, which opens the
  project. Nothing from the command line is executed.
- **Startup problems are the application's own.** Before the window exists the
  application shows a native message box (safe Win32 bindings, `winsafe`): its knowledge
  pack is missing from `resources/` (release builds), the data folder is unusable, or the
  window cannot be created (with a WebView2 hint when the runtime is not installed; a
  panic before the window exists is reported the same way). Problems with a requested
  project file are reported in the window.
- **Staging** installs the executable atomically into the root (it fails clearly while
  PLC Lab runs), swaps `resources/` in atomically, removes earlier entry points
  (`Launcher.exe`, `releases/current/`) and verifies the distribution.
- **Validation.** The canonical Windows smoke starts the root `PLC Lab.exe` from another
  working directory, then from a copy of the distribution in a folder whose path contains
  spaces, then through the BAT; it checks the distribution first, that the knowledge pack
  comes from `resources/`, single instance, the project-file hand-over and a clean close.
- **Retired.** `apps/launcher` (the launcher window and the alias), its status window,
  its smoke tests and the launcher CI stages. Opening Explorer folders from the launcher
  is not carried over: the application has no shell access by design; its Knowledge page
  shows the folders it searches.

## Alternatives considered

Keeping a wrapper `PLC Lab.exe` that starts the application (rejected: two executables,
errors shown by a wrapper); embedding the knowledge pack in the binary (rejected: no
benefit, larger binary, packs are data); staging to `resources/knowledge/` from the source
tree instead of the verified archive (rejected: the release ships the verified content).

## Consequences

A copied `PLC Lab.exe` without its `resources/` folder refuses to start with a clear
message instead of opening an empty application. The local build overwrites
`PLC Lab.exe` in place, so the build fails while PLC Lab runs. Single-instance hand-over
failures inside the plugin are not observable by the application.

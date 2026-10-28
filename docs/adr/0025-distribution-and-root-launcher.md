# ADR-0025: Portable release staging and root launcher

- Status: Accepted for the product rebuild
- Date: 2026-10-27

## Context

A normal Windows user must start PLC Lab from the repository or distribution folder
without tools, terminals or knowledge of the build layout. `target/` is a build
workspace, not a distribution.

## Decision

- The canonical staged release is `releases/current/` containing `PLC Lab Desktop.exe`
  (the optimized Tauri build with embedded UI), audited local DLLs, the verified knowledge
  pack under `knowledge-packs/`, `release-manifest.json` (versions and SHA-256 of every
  file) and `RUNTIME_REQUIREMENTS.txt`. Staging is atomic and fails on any missing file.
- The root `PLC Lab.exe` is a separate tiny native launcher (`apps/launcher`): it resolves
  `releases/current/PLC Lab Desktop.exe` relative to its own path, starts it with that
  folder as working directory, opens no console, needs no administrator rights or
  network, contains no product logic and reports a missing release in a native message
  box. It uses no `unsafe` code (the message box is shown by the system PowerShell with
  the text passed through an environment variable).
- `RUN_PLC_LAB.bat` is the batch fallback. Development stays on `RUN_PLC_LAB_DEV.bat`.
- Generated binaries are not committed; the knowledge archive is committed because it is
  a deterministic, verified release artifact.
- Hosted Windows CI builds, stages and smoke-tests the release and the launcher and
  publishes them as a workflow artifact. Binaries are unsigned; signing is a release
  pipeline concern documented in `docs/BUILD_AND_RUN.md`.

## Alternatives considered

Copying the Tauri executable to the root (rejected: knowledge packs and DLLs would have
to live in the root); an installer (deferred); a shortcut file (not portable when the
folder moves).

## Consequences

Machines that block unsigned freshly built executables (Smart App Control) cannot build
or run local releases without signing; validation then relies on hosted CI.

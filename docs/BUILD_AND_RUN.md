# Build and run

## For users

| Entry point                 | What it does                                                        |
| --------------------------- | ------------------------------------------------------------------- |
| `Launcher.exe` (root)       | Launcher window: status, recent project, diagnostics, Start PLC Lab |
| `PLC Lab.exe` (root)        | Compatibility alias: starts `releases/current/PLC Lab Desktop.exe`  |
| `RUN_PLC_LAB.bat` (root)    | Same, as a batch file fallback                                      |
| `BUILD_PLC_LAB_RELEASE.bat` | Validates, builds the optimized release and both launchers, stages  |
| `RUN_PLC_LAB_DEV.bat`       | Development application with hot reload (`npm run dev`)             |
| `RUN_ALL_CHECKS.bat`        | Complete validation (`npm run validate`)                            |

`Launcher.exe` (`apps/launcher`, ADR-0026) is a native window, not a web view. It shows
the version, an overall status, the most recent project, the knowledge pack (version,
file count, validation) and four installation checks: application, knowledge pack,
WebView2 Runtime and project folder. Its buttons:

- **Start PLC Lab** starts the release and closes the launcher;
- **Open Recent** / **Open Project...** start PLC Lab with a project (a running PLC Lab
  opens it instead); a project outside the projects folder is imported as a copy;
- **Manage Knowledge Packs** opens `%APPDATA%\dev.plclab.desktop\knowledge-packs`, where
  additional packs are discovered and validated by the application;
- **Diagnostics** shows a plain-text report (paths, versions, checks) with **Copy
  Diagnostics**; nothing is sent anywhere;
- **Open Data Folder** opens `%APPDATA%\dev.plclab.desktop`.

Command line: `Launcher.exe "<file.plclab>"` opens a project, `Launcher.exe --start`
starts PLC Lab, `Launcher.exe --diagnostics` prints the report. The launcher resolves
everything relative to its own location (the folder can be moved), opens no console,
needs no administrator rights and no network and never starts a development server. Each
problem (release not built, knowledge pack missing or invalid, corrupt release manifest,
WebView2 missing, unusable data folder, missing or invalid project file) is shown in the
window or a message box. If PLC Lab is already running, starting it again (through either
launcher, the BAT file or the application itself) brings the open window to the front
instead of starting a second instance.

Requirements to run: Windows 10/11 x64 and the Microsoft Edge WebView2 Runtime
(preinstalled on current Windows, not bundled). Data (projects, progress, additional
knowledge packs) is stored in `%APPDATA%\dev.plclab.desktop`.

## Staged release layout

```
Launcher.exe                         launcher window (generated, not in Git)
PLC Lab.exe                          compatibility alias (generated, not in Git)
RUN_PLC_LAB.bat
releases/
  current/                           generated, not in Git
    PLC Lab Desktop.exe              optimized Tauri application with embedded UI
    knowledge-packs/PLC_Lab_Knowledge_Pack_v1/
    release-manifest.json            versions, DLL audit, SHA-256 of every file
    RUNTIME_REQUIREMENTS.txt
  knowledge/
    PLC_Lab_Knowledge_Pack_v1.zip    committed release archive (+ .sha256)
```

Staging (`scripts/stage-release.mjs`) audits DLL imports with `dumpbin`, verifies that the
knowledge archive matches the pack, copies everything into a temporary folder, hashes and
verifies every file, checks required files, swaps the folder in atomically and installs
both root launchers (their hashes are recorded in the manifest). A failed step leaves the previous release untouched and exits nonzero.

## Commands

```
npm ci                       # locked dependencies
npm run dev                  # development application
npm run validate             # all checks + native debug smoke
npm run build:release        # validate + release build + staging
npm run build:release:native # optimized Tauri build only
npm run stage:release        # build launchers + stage (after build:release:native)
npm run test:desktop:release # native smoke of the staged release
npm run test:launcher:gui    # Launcher.exe window, modes and project hand-over
npm run test:launcher        # native smoke through the root PLC Lab.exe
npm run knowledge:verify     # knowledge archive matches the pack
```

## Code signing

Release executables are not code-signed. Windows SmartScreen or Smart App Control may
block or warn about unsigned binaries built locally. Do not disable Windows security to
run them; sign them with an organization certificate (for example with `signtool` in a
release pipeline) or use builds whose reputation your policy accepts. On the development
machine used for the rebuild, Smart App Control blocked freshly compiled release build
scripts (os error 4551); the release is therefore built, staged and smoke-tested on hosted
Windows CI and published as the `plc-lab-windows-release` workflow artifact.

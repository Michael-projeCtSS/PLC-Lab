# Build and run

## For users

| Entry point                 | What it does                                                     |
| --------------------------- | ---------------------------------------------------------------- |
| `PLC Lab.exe` (root)        | Starts the staged release `releases/current/PLC Lab Desktop.exe` |
| `RUN_PLC_LAB.bat` (root)    | Same, as a batch file fallback                                   |
| `BUILD_PLC_LAB_RELEASE.bat` | Validates, builds the optimized release and stages it            |
| `RUN_PLC_LAB_DEV.bat`       | Development application with hot reload (`npm run dev`)          |
| `RUN_ALL_CHECKS.bat`        | Complete validation (`npm run validate`)                         |

`PLC Lab.exe` is a small native launcher (`apps/launcher`). It resolves the release
relative to its own location (the folder can be moved), opens no console, needs no
administrator rights and no network, and shows a message if the release has not been
built. It never starts a development server.

Requirements to run: Windows 10/11 x64 and the Microsoft Edge WebView2 Runtime
(preinstalled on current Windows, not bundled). Data (projects, progress, additional
knowledge packs) is stored in `%APPDATA%\dev.plclab.desktop`.

## Staged release layout

```
PLC Lab.exe                          root launcher (generated, not in Git)
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
the root launcher. A failed step leaves the previous release untouched and exits nonzero.

## Commands

```
npm ci                       # locked dependencies
npm run dev                  # development application
npm run validate             # all checks + native debug smoke
npm run build:release        # validate + release build + staging
npm run build:release:native # optimized Tauri build only
npm run stage:release        # build launcher + stage (after build:release:native)
npm run test:desktop:release # native smoke of the staged release
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

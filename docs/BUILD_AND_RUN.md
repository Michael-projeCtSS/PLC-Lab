# Build and run

## For users

Double-click **`PLC Lab.exe`** in the product (repository) root. That is the whole
startup: `PLC Lab.exe` is the PLC Lab desktop application itself (the optimized Tauri
build), not a launcher, and it starts no other executable (ADR-0030).

| File                        | What it does                                                        |
| --------------------------- | ------------------------------------------------------------------- |
| `PLC Lab.exe` (root)        | The application. `PLC Lab.exe "<file.plclab>"` opens a project      |
| `resources/` (root)         | Data the application reads: knowledge pack and `product.json`       |
| `RUN_PLC_LAB.bat` (root)    | Optional fallback that only starts `PLC Lab.exe` next to it         |
| `BUILD_PLC_LAB_RELEASE.bat` | Validates, builds the optimized release and stages it into the root |
| `RUN_PLC_LAB_DEV.bat`       | Development application with hot reload (`npm run dev`)             |
| `RUN_ALL_CHECKS.bat`        | Complete validation (`npm run validate`)                            |

`PLC Lab.exe` resolves everything relative to its own location, so it works when started
from Explorer, CMD or PowerShell, from any working directory, from a folder whose path
contains spaces, and after the folder (`PLC Lab.exe` together with `resources/`) is
copied or moved. It opens no console, needs no administrator rights and no network and
never starts a development server.

Only one PLC Lab window runs at a time. Starting `PLC Lab.exe` again brings the open
window to the front; starting it with a project file (`PLC Lab.exe "<file.plclab>"`,
also `--open-project "<file.plclab>"`) hands the file to the running window, which opens
it. The file is only read and validated; nothing from the command line is executed. A
project outside the projects folder is imported as a copy; a missing, invalid or too-new
project file is reported in the window.

Problems before the window can exist are shown in a native message box: the knowledge
pack missing from `resources\knowledge-packs\` (the executable was copied without its
`resources` folder), an unusable data folder, or an application window that cannot be
created (with a hint when the Microsoft Edge WebView2 Runtime is not installed).

Requirements to run: Windows 10/11 x64 and the Microsoft Edge WebView2 Runtime
(preinstalled on current Windows, not bundled). Data (projects, progress, additional
knowledge packs) is stored in `%APPDATA%\dev.plclab.desktop`; the Knowledge page shows the
folders it searches for packs.

## Distribution layout

```
PLC Lab.exe                          the application (generated, not in Git)
RUN_PLC_LAB.bat                      optional fallback
resources/                           generated, not in Git
  knowledge-packs/PLC_Lab_Knowledge_Pack_v1/
  product.json                       version, DLL audit, SHA-256 of the executable and every file
  RUNTIME_REQUIREMENTS.txt
releases/
  knowledge/
    PLC_Lab_Knowledge_Pack_v1.zip    committed release archive (+ .sha256)
```

The distribution contains exactly one application executable, `PLC Lab.exe`. Any DLL the
build produces next to the executable would be staged beside it and listed in
`product.json` (the current build imports only system DLLs). `releases/` holds archives
and CI artifacts only; it is not a runtime location.

Staging (`scripts/stage-release.mjs`) audits DLL imports with `dumpbin`, verifies that the
knowledge archive matches the pack, installs `target/release/plc-lab-desktop.exe` as the
root `PLC Lab.exe` (copy to a temporary file, hash check, atomic rename; it fails clearly
while PLC Lab is running), stages `resources/` in a temporary folder with hashes and swaps
it in atomically, removes entry points of earlier distributions (`Launcher.exe`,
`releases/current/`), and finally checks the distribution (`scripts/distribution.mjs`):
one application executable, every file matching `product.json`, a knowledge pack present.

## Commands

```
npm ci                         # locked dependencies
npm run dev                    # development application
npm run validate               # all checks + native debug smoke
npm run build:release          # validate + release build + staging into the root
npm run build:release:native   # optimized Tauri build only
npm run stage:release          # stage PLC Lab.exe and resources/ (after build:release:native)
npm run check:distribution     # one application executable, files match product.json
npm run test:desktop:release   # native smoke of the root PLC Lab.exe from another directory
npm run test:release:relocated # the same from a copied folder whose path contains spaces
npm run test:release:bat       # the same through RUN_PLC_LAB.bat
npm run knowledge:verify       # knowledge archive matches the pack
```

## Code signing

Release executables are not code-signed. Windows SmartScreen or Smart App Control may
block or warn about unsigned binaries built locally. Do not disable Windows security to
run them; sign them with an organization certificate (for example with `signtool` in a
release pipeline) or use builds whose reputation your policy accepts. On the development
machine used for the rebuild, Smart App Control blocked freshly compiled release build
scripts (os error 4551); the release is therefore built, staged and smoke-tested on hosted
Windows CI and published as the `plc-lab-windows-release` workflow artifact.

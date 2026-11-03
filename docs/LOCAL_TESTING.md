# Local testing without a publisher certificate

## Browser UI preview

Double-click **TEST_PLC_LAB_WEB.bat** in the repository root. Keep its terminal open.
It checks Node.js 24+ and npm, checks the installed dependency tree against the
lockfile, runs `npm ci` when needed, starts Vite, then opens the default browser at
**http://127.0.0.1:1420**. It never builds or launches a native PLC Lab executable.
The first dependency installation needs Internet access. Subsequent valid installs
are reused. The root path may contain spaces; the current working directory does
not matter. `npm run dev:web` uses the same entrypoint.

Edit UI source to see Vite updates. Knowledge JSON changes reload the public content.
Press **Q then Enter** or **Ctrl+C** in the original terminal to stop and release port 1420. Closing its console ends the in-process server. A retry reuses this checkout's
existing session without opening another browser tab. An unrelated listener on
1420 causes a readable error; the helper never stops it or switches ports silently.
`--no-open` is available for automated tests only.

The permanent **TRYB PODGLĄDU INTERFEJSU / UI PREVIEW** banner identifies this mode.
The preview reads the real repository knowledge pack through a development-only,
read-only presentation endpoint. It shows 25 lessons, 49 exercise summaries and 95
articles. Content localization uses the existing PL/EN architecture. No educational
source files are copied into fixtures or rewritten.

| Capability                                                           | Browser preview                      |
| -------------------------------------------------------------------- | ------------------------------------ |
| Home, Learn, Practice and Knowledge catalogues                       | Available                            |
| Lessons, articles, search and static self-check questions            | Available; no saved progress         |
| Settings, PL/EN, theme, density, refresh                             | Available; local browser preferences |
| Basic/Advanced navigation and Monaco ST editor                       | Available; editor text is unsaved    |
| Projects, files, compile/load, PLC scans, process/network simulation | Native backend required              |
| Hardware tools, online state, traces, grading and progress           | Native backend required              |

Native actions are disabled or explain the missing backend. There is no simulated
runtime, plant state, grading result, or native filesystem adapter in the browser.
This server is for a local developer checkout, not deployment or public hosting.

## Unsigned native development build in an isolated Windows guest

The real Tauri/Rust application is required for PLC programming and simulation.
Use a supported Windows Sandbox or a dedicated Windows VM whose normal guest policy
permits unsigned developer applications. Keep host Smart App Control and other
security settings unchanged. If the guest blocks the executable, record **BLOCKED**
and retain the warning. Do not change policy, rename the executable, or use a loader.

1. Run `npm run test:native:status`. It reads Windows edition, hypervisor presence,
   Sandbox executable and optional-feature status. Feature inspection may need an
   administrator; an unknown state is reported explicitly. The script changes nothing.
   Windows Sandbox supports appropriate Pro, Enterprise and Education editions;
   installation and prerequisites are described in the
   [Microsoft Sandbox overview](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/).
2. Retrieve a **successful Canonical validation** run from the trusted development
   branch: `npm run test:native:fetch -- <run-id>`. Requires authenticated GitHub CLI.
   This verifies run/commit identity, the GitHub artifact archive SHA-256, unsigned
   classification, and every distribution file against its manifest. It extracts into
   a fresh directory under `artifacts/native-test-downloads/` and prints that path
   plus a receipt. Nothing is installed over the root application or executed.
   Alternatively build with `npm run build:release:unsigned` in a **separate checkout**
   on a Windows development/build environment where normal policy permits the build
   tools and native tests, then use its complete verified distribution. A local build
   does not by itself establish successful CI provenance.
3. Run `npm run test:native:prepare -- "<distribution-directory>"`. Integrity is
   checked before and after copying. It prints a dedicated session path containing
   `input/product`, `output`, `preparation.json`, and **PLC Lab native test.wsb**.
4. If Sandbox is available, double-click that WSB. It maps only this session's input
   read-only and output writable. Networking, clipboard, microphone, camera and printer
   redirection are disabled. It opens Explorer; it does **not** automatically execute
   the product or a PowerShell script. The mapping follows Microsoft's
   [Sandbox configuration schema](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-configure-using-wsb-file).
5. Inside the guest, open `C:\PLC-Lab-Test\Input\product\PLC Lab.exe` where guest
   policy permits it. The guest needs Microsoft Edge WebView2 Runtime; host-installed
   software is not inherited. If missing, use a prepared VM with the official
   [Microsoft WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
   installed normally. The generated Sandbox configuration deliberately has no network.
6. Test Polish Home/Learn/Practice/Knowledge, Settings and PL/EN; open the tank exercise,
   compile/load its ST program, run/pause/single-scan, observe process animation and
   trace, then grade. Check Basic/Advanced hardware and diagnostics navigation. Record
   actual outcomes and runtime faults. Never infer a native PASS from browser results.
7. Where the guest's normal script policy permits, run
   `powershell.exe -NoProfile -File C:\PLC-Lab-Test\Input\collect-native-guest.ps1 -ManualResult 'PASS'`
   (or `BLOCKED` / `NOT TESTED`). The guest-only collector verifies manifest file
   hashes, saves a desktop screenshot, relevant Code Integrity events and a JSON
   report to `C:\PLC-Lab-Test\Results`. It does not launch the application, run a
   simulation or grant trust. If script policy blocks it, save screenshots, observed
   results and relevant Event Viewer logs manually; do not bypass execution policy.
8. Keep results in the mapped output folder before closing the guest. Closing Sandbox
   discards its internal state. Inspect the host session's `output` afterward.

If Sandbox is absent, do not enable Windows features automatically. Use an existing
or separately provisioned Windows 11 x64 VM in Hyper-V, VMware or VirtualBox. Copy the
prepared **input** folder into the guest (read-only sharing or an ISO is preferable),
and use a dedicated results folder. Adjust the collector's `-Product` and `-Output`
paths if needed. Install WebView2 through the guest's normal administrator workflow,
take a clean snapshot, test the same unchanged distribution under guest policy,
export evidence, shut down and restore the snapshot. The host's hypervisor being
present is not evidence that a configured VM exists.

## Signed production release

**BUILD_PLC_LAB_RELEASE.bat** and protected CI still require a trusted publisher
certificate and verify executable/catalog signatures. Browser preview and unsigned
guest testing grant no production trust. See [Windows code signing](WINDOWS_CODE_SIGNING.md).
`PLC Lab.exe` remains the single native application; `RUN_PLC_LAB.bat` remains its
optional fallback and `RUN_PLC_LAB_DEV.bat` remains native development startup.

## Automated evidence

`npm run test:web` runs the real BAT lifecycle and a fresh headless browser against
the real Vite server. It checks PL/EN, readers, Settings, responsive layout, refresh,
Basic/Advanced, CSS hot updates, retries, shutdown, occupied port, missing Node/npm
and a checkout path containing spaces. It checks host native-process/block events,
root EXE hash and Smart App Control before/after. It refuses to stop an existing
preview; close that session first. Use `PLC_LAB_BROWSER_CHANNEL=chrome` if Edge is
unavailable. Reports/screenshots are in `artifacts/web-preview-launcher/` and
`artifacts/web-preview-smoke/`. Windows CI retains this evidence with native checks.
An automated BAT invocation verifies the command path; a literal Explorer mouse
double-click is a separate manual acceptance check.

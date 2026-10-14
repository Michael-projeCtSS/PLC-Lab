# Development

## M2B session development

Compile != Load != Execute. Compile edits/validates source only. After current successful Compile, use Load Program, stage Input values (numeric fields use Apply/Enter; TIME is integer microseconds) and press Single Scan. Each successful click advances the Rust SimulationClock by exactly 100 ms without waiting; click speed never defines elapsed PLC time. Reset Runtime clears staged inputs, all memory/TON state, fault/count/time. Editing marks the loaded association stale and requires Compile + Load again. Unload preserves source/compile metadata. Run/Pause/Stop remain disabled.

Focused application checks: node scripts/rust.mjs test --locked -p app-contract -p desktop-session. Release: add --release. Existing npm run validate covers the full local canonical/native path; ci:windows and ci:core retain prior checks and now select the portable contract/session packages in both profiles. Complete snapshots are returned once per mutation; no polling loop exists.

## M2A compiler/editor development

The UI package bundles pinned Monaco 0.57.0 and its editor worker locally; no CDN, parsing dependency or ST evaluator is used. IPC is now v3; regenerate declarations with npm run ipc:generate after Rust DTO edits. Compile runs bounded ST compilation only; controls remain disabled. No persistence or hidden runtime is created by compilation.

Focused command: `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core -p st-frontend -p app-contract`. Optimized source/runtime proof: `node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core -p st-frontend`. Existing npm run validate remains canonical, including actual Windows Tauri/WebView2 compilation/IPC/Monaco smoke. CI runs source tests on both OS lanes and retains every M1 regression requirement. See [Alpha ST](spec/STRUCTURED_TEXT_ALPHA.md), [ADR-0012](adr/0012-alpha-st-compiler.md) and the extended human checklist. M1 validation documents retain their original exact code SHA.

## Prerequisites

Use Node.js 24+, npm, and the pinned Rust 1.99.0 toolchain with rustfmt and clippy. Windows requires MSVC C++ build tools, a Windows SDK, and WebView2. macOS and Linux need their native Tauri prerequisites. See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for the platform-specific installation steps. Sprint 0 native validation targets Windows x86_64; other platforms are not yet validated.

The initial machine had Node, MSVC, and WebView2, but no Rust. Rust was installed locally under ignored .tools/cargo and .tools/rustup, without modifying system PATH. Repository scripts use that local toolchain if present, otherwise Rust on PATH. A normal developer can install Rust through rustup and run `rustup show` in the repository to obtain the pinned toolchain. No .tools files are committed or required on another machine.

## Setup

```powershell
npm ci
npm run ipc:generate
```

If Node reports UNABLE_TO_VERIFY_LEAF_SIGNATURE on a machine with a trusted enterprise CA installed in Windows, Node 24 supports the system CA store:

```powershell
$env:NODE_OPTIONS = '--use-system-ca'
npm ci
```

This session used that setting; certificate verification remains enabled. No npm registry or machine trust settings were changed.

## Repository commands

The project owner's maintained double-click workflow is documented in [human testing](HUMAN_TESTING.md). Its four root BAT launchers share scripts/human-launcher.bat and scripts/human-test.mjs, call canonical commands below, and preserve exit codes. Future milestones must maintain the launchers and extend the checklist.

The Tauri beforeDevCommand starts the installed Vite CLI directly in apps/desktop with the same vite.config.ts used by dev:web. This avoids nested npm lifecycle failure messages when Tauri stops the development server after normal application closure; genuine startup failures remain visible and nonzero.

Run these from the repository root. They work through npm on Windows; use npm.cmd if PowerShell execution policy blocks npm.ps1.

| Command                     | Purpose                                                                     |
| --------------------------- | --------------------------------------------------------------------------- |
| `npm run dev`               | Start Vite and the native Tauri desktop shell                               |
| `npm run dev:web`           | Browser-only shell preview; reports native core unavailable                 |
| `npm run build`             | Typecheck and build frontend assets                                         |
| `npm run build:desktop`     | Build a native debug executable with embedded frontend assets, no installer |
| `npm run format`            | Format source/docs and Rust                                                 |
| `npm run format:check`      | Check Prettier and cargo fmt                                                |
| `npm run typecheck`         | Strict TypeScript checks                                                    |
| `npm run lint`              | ESLint and workspace clippy with warnings denied                            |
| `npm run test:frontend`     | React and IPC adapter unit tests                                            |
| `npm run test:rust`         | All Rust workspace tests, including integration and DTO drift checks        |
| `npm run test:architecture` | Cargo dependency boundaries and Sprint 0 host/renderer constraints          |
| `npm test`                  | Architecture, frontend, and Rust tests                                      |
| `npm run test:desktop`      | Build and exercise the actual Windows Tauri/WebView2 application and IPC    |
| `npm run ipc:generate`      | Regenerate checked-in TypeScript DTOs from Rust                             |
| `npm run check`             | Formatting, typechecking, lint/clippy, all non-native tests, frontend build |

For direct Rust commands with the optional local toolchain, use `node scripts/rust.mjs <cargo arguments>`. Lockfiles are committed; use npm ci and --locked Cargo commands for repeatability. The core contract generator may need a first `cargo run` without --locked when intentionally changing dependencies.

The build embeds apps/desktop/dist. If running cargo directly before a frontend build, the desktop compile can fail because embedded assets are absent. Use `npm run build:desktop` for the supported native build path. Installer packaging is disabled for this foundation sprint.

`npm run validate` runs check and the native smoke test. `npm run build:release` runs validate, builds real Tauri release assets/executable with the locked Cargo dependencies, then stages a verified runtime artifact set to releases/current. Release staging reads Cargo metadata and MSVC dumpbin imports and does not modify the executable. RUN_PLC_LAB_RELEASE.bat launches that existing artifact without requiring development tools or performing a build.

getCoreInfo now uses IPC contract version 2 because buildContext is a required response field. Rust derives development/release from its actual debug_assertions configuration. The same generated contract and existing Diagnostics detail line carry this information; application versions still come from Cargo package metadata. Update callers together and regenerate DTOs when changing this contract.

## Contract changes

Edit Rust DTOs in crates/app-contract, run ipc:generate, and commit both sources and packages/ipc/src/generated.ts. The Rust drift test compares exact generated text. Generated.ts is excluded from Prettier because it belongs to the Rust generator. AGENTS.md is also excluded to preserve the supplied engineering contract verbatim. Do not hand-edit generated declarations.

IPC requests and responses are runtime validated. A browser preview has no substitute Rust service or fake successful response. Unit tests isolate transport with mocks; native acceptance uses the actual executable and no mocked backend.

## Native smoke test

The Windows smoke harness launches target/debug/plc-lab-desktop.exe with a fresh WebView2 data folder and a dynamically assigned debugging port, connects playwright-core to that WebView2 instance, verifies panels and disabled controls, checks real successful and rejected IPC requests, and closes its own application process. Remote debugging is enabled only in the spawned test environment, not application configuration. No separately downloaded browser is required.

Ignored artifacts are written to artifacts/desktop-smoke: desktop.png, result.json, and native.log. The debug executable remains in target/debug for manual review. The smoke harness currently assumes the default Cargo target directory; a custom CARGO_TARGET_DIR must be unset for this test.

## Contribution boundaries

Read AGENTS.md, the [ADRs](adr/README.md), [architecture](architecture/ARCHITECTURE.md), and [test strategy](spec/TEST_STRATEGY.md) before extending the product. M1A semantics are accepted; M1B adds headless owned execution and a deterministic clock. Trace/network/grading/language crates remain reservations. Introduce features only in approved milestones. Keep production code and technical documentation in English.

For focused M1A checks, run `node scripts/rust.mjs test --locked -p plc-types -p plc-ir` and `node scripts/rust.mjs clippy --locked -p plc-types -p plc-ir --all-targets -- -D warnings`. These include type/value, identity, compatibility, symbol, diagnostic, source, IR golden, and validator tests. The normal workspace test commands include them automatically. `npm run validate` remains the full regression gate, including actual native IPC; no fake UI or runtime command is needed for headless semantic work.

For M1B/M1C, use `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core` and the equivalent clippy command with `--all-targets -- -D warnings`. Direct-IR execution tests are in plc-runtime/tests and reuse the test-only semantic assembler. No dev server, OS clock, or sleeps are required for semantic tests. Runtime APIs remain headless and are not exposed through desktop IPC. M1C implementation review is accepted; validation remains blocked by the owner's Windows host policy. Review [runtime semantics](spec/PLC_RUNTIME_SEMANTICS.md) before adding executable features. M2 is not authorized.

## Independent CI validation

See [CI_VALIDATION.md](validation/CI_VALIDATION.md) for the prepared Windows canonical and Linux core lanes, exact commands, artifacts and gate policy. `.github/workflows/validation.yml` owns automatic push/PR validation and manual dispatch; the older quality workflow delegates manually to the same lane definition.

`npm run test:ci` checks workflow YAML/configuration and CI command failure/logging behavior without running blocked PLC test binaries. It is also included by `npm run test:architecture`. `npm run ci:status` prints the documented local-host limitation without changing the application. `npm run ci:windows` runs the existing canonical check and native test commands plus required focused debug/optimized core tests; `npm run ci:core` is Linux-only. Neither is a compile-only substitute for acceptance. Hosted Windows prerequisite installation is guarded against local use.

Preparation alone performs no remote, push or hosted execution. The later owner-authorized M1C-CI-RUN published private PLC-Lab and passed both lanes/native Windows acceptance in run 38533787674 for code SHA a4a22f19c7bcdbd1c1d78a38804c29e15da32828. See the [gate record](validation/M1C_INDEPENDENT_VALIDATION.md) and [remote report](validation/M1C_REMOTE_VALIDATION_REPORT.md). Local Smart App Control remains unchanged. Technical-lead review and explicit M2A authorization are still required; gate recording starts no frontend work.

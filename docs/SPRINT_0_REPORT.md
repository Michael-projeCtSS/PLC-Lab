# Sprint 0 — Project Foundation completion report

Date: 2026-10-13 (Europe/Warsaw). Scope: FND-001 through FND-010. Status: complete; stopped at M0 for architectural review. No M1 implementation was started.

## Completed

| Task    | Result | Delivered                                                                                                                                                         |
| ------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| FND-001 | PASS   | Eight ADRs for the required stack, shared IR, deterministic clock, sandbox, process/rendering separation, trace, virtual network, and local versioned persistence |
| FND-002 | PASS   | Alpha 0.1/MVP/future feature matrix; Alpha remains ST, BOOL/INT/REAL/TIME, assignments, basic expressions, IF, TON                                                |
| FND-003 | PASS   | Git repository on main; Cargo and npm workspaces; all requested module, content, and test boundaries                                                              |
| FND-004 | PASS   | Working Tauri 2/React/TypeScript desktop shell with seven panels and disabled Run/Pause/Stop/Single Scan                                                          |
| FND-005 | PASS   | Rust-generated DTOs/constants, typed IPC adapter, runtime response validation, real native round trip, explicit version/transport errors                          |
| FND-006 | PASS   | Formatting, lint, strict typing, Rust fmt/clippy, frontend/Rust/architecture/native tests, root commands, Windows quality workflow and development guide          |
| FND-007 | PASS   | Architecture map, canonical execution chain, dependency graph, and ownership boundaries                                                                           |
| FND-008 | PASS   | Diagnostics specification and Rust category/severity/source range definitions with boundary/error tests                                                           |
| FND-009 | PASS   | Test strategy covering all requested layers and the deterministic trace golden rule                                                                               |
| FND-010 | PASS   | M0–M9 roadmap with DEMO-001 before graphical language editors                                                                                                     |

AGENTS.md remains byte-identical to the supplied contract (SHA-256: 24AB582DDC05C57EB90D3DBA91B329492C2AE4F4754FEDCAFA70FF3AD198DF47). Git attributes and formatter exclusions preserve it.

## Files

- [ADRs](adr/README.md), [feature matrix](spec/PLC_FEATURE_MATRIX.md), [architecture](architecture/ARCHITECTURE.md), [IPC boundary](architecture/FRONTEND_BACKEND_BOUNDARY.md).
- [Development](DEVELOPMENT.md), [diagnostics](spec/DIAGNOSTICS.md), [test strategy](spec/TEST_STRATEGY.md), [roadmap](ROADMAP.md).
- [Cargo workspace](../Cargo.toml), [npm workspace/commands](../package.json), [pinned Rust toolchain](../rust-toolchain.toml), [quality workflow](../.github/workflows/quality.yml).
- [Native host](../apps/desktop/src-tauri/src/lib.rs), [command adapter](../apps/desktop/src-tauri/src/commands.rs), [React shell](../packages/ui/src/LabShell.tsx).
- [Rust IPC contract](../crates/app-contract/src/lib.rs), [generated TypeScript](../packages/ipc/src/generated.ts), [typed IPC client](../packages/ipc/src/index.ts), [diagnostic types](../crates/plc-types/src/diagnostics.rs).
- [Architecture tests](../tests/architecture/boundaries.test.mjs), [Rust integration tests](../tests/conformance/tests/foundation.rs), [native smoke harness](../tests/e2e/desktop-smoke.mjs).

Local ignored validation artifacts: [desktop screenshot](../artifacts/desktop-smoke/desktop.png), [native result](../artifacts/desktop-smoke/result.json), and native.log. The embedded-frontend executable is target/debug/plc-lab-desktop.exe. These artifacts are rebuilt with npm run test:desktop and are not repository source.

## Architecture

The eight ADRs establish the approved foundation boundaries. Core crates are independent of React/Tauri and language frontends. ST is the first reserved frontend; LAD/FBD/STL remain directory reservations. All eventual language frontends lower to one vendor-neutral IR and one bounded runtime.

Only diagnostics vocabulary and application information are substantive core/contract implementations in this sprint. Runtime, IR, clock, process/network models, trace recording, and grading remain reserved boundaries. VERSION constants are package metadata, not functioning simulation components.

The desktop exposes one read-only get_core_info command. Request/response types and command/version constants are generated from Rust. Incoming requests and outgoing responses are validated; unsupported contract versions produce APP-IPC-001. React displays only the returned package information and explicit connection states. No renderer logic executes student programs or advances simulation state.

SimulationClock, source-aware canonical trace, sandbox limits, network adapters, and versioned local JSON persistence are documented decisions only. Detailed IR semantics, fault/partial-scan policies, clock scheduling, and the persistence schema require their next scoped specifications before implementation.

## Tests

Final verification completed successfully:

| Command/check                          | Result                                                                                                                                                                                                               |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| npm run format                         | PASS: Prettier and cargo fmt applied                                                                                                                                                                                 |
| npm run check                          | PASS: formatting, strict TypeScript, ESLint, workspace clippy with warnings denied, all non-native tests, frontend build                                                                                             |
| npm run test:architecture              | PASS: 3 tests covering dependency closure, harmless IPC surface, and renderer evaluator/scheduler guards                                                                                                             |
| npm run test:frontend                  | PASS: 14 tests for seven panels, disabled controls, connection/error states, validation, and unmount behavior                                                                                                        |
| npm run test:rust                      | PASS: 10 substantive tests (4 contract, 4 diagnostics, 2 conformance), workspace targets and doc tests successful                                                                                                    |
| npm run test:desktop                   | PASS: real Windows Tauri/WebView2 app, embedded React assets, real IPC success/rejection, all panels, disabled controls, no run command, no page errors, no horizontal overflow, Diagnostics fits the default window |
| npm run dev + live WebView2 inspection | PASS: Vite starts, native application starts at the development URL, real core information arrives, Run remains disabled                                                                                             |
| Documentation inspection               | PASS: all 15 required ADR/specification/architecture/development documents exist; local Markdown links resolve                                                                                                       |
| Contract SHA-256 comparison            | PASS: exact match to supplied engineering contract                                                                                                                                                                   |

The desktop main entry point contains no unit tests and does not generate an empty test executable; actual host startup and its command adapter are covered by native acceptance. Contract/diagnostics/integration tests remain enabled. Unit transport mocks are restricted to test files. Native validation uses no mock Rust backend or fake simulation.

## Acceptance criteria

| Requested criterion                      | Result | Evidence                                                                                                 |
| ---------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------- |
| Tauri application starts                 | PASS   | Built executable and native development application both started                                         |
| React frontend works                     | PASS   | Seven panels rendered in actual WebView2; frontend tests and build pass                                  |
| Rust backend works                       | PASS   | Registered Rust command returned real Cargo package versions                                             |
| Typed IPC works                          | PASS   | Generated request/response types, checked adapter, Rust drift test, native success and version rejection |
| ADRs exist                               | PASS   | Eight records plus index in docs/adr                                                                     |
| Feature matrix exists                    | PASS   | Alpha 0.1, MVP, future support documented across all requested categories                                |
| Architecture documentation exists        | PASS   | Canonical execution chain, dependency map and frontend/backend boundary                                  |
| Development documentation exists         | PASS   | Platform prerequisites, local toolchain assumptions and root commands                                    |
| Format checks pass                       | PASS   | Prettier check and cargo fmt --check                                                                     |
| Lint/clippy checks pass                  | PASS   | ESLint and workspace/all-targets clippy with -D warnings                                                 |
| Tests pass                               | PASS   | 27 non-native substantive tests plus actual native acceptance                                            |
| No fake PLC execution exists in frontend | PASS   | Disabled controls, no execution IPC command, no evaluator/scheduler in shell, native rejection of run    |

Scope constraints also pass: no ST parser, PLC VM, timers, LAD/FBD/STL editor, process/network simulation, or protocol was implemented.

## Assumptions

- Start with npm/Cargo workspaces, application version 0.1.0, and the neutral desktop identifier dev.plclab.desktop. Node 24 and Rust 1.99.0 are the validated toolchains. TypeScript 6.0.3 matches the linter's supported range.
- The initial host lacked Rust, so the development toolchain was installed in ignored workspace-local .tools directories; system PATH was not modified. Node package installation used the Windows system CA store while retaining TLS verification.
- A single entry Program with explicit named input/output/internal variables is the minimal eventual Alpha basis for the requested ST features. This is a support plan only.
- Integer microsecond virtual ticks and a versioned local .plclab JSON document are the simplest planned interpretations of the clock/persistence requirements. Their detailed semantics/schema remain subject to the next architectural review and implementation tasks.

## Known limitations

- Native acceptance was performed on Windows x86_64/WebView2. macOS/Linux native startup has not been verified; the CI workflow is configured but has no remote run in this local repository.
- Delivery includes a debug desktop executable with embedded assets. Installer packaging is disabled.
- Execution and all simulation remain unavailable by Sprint 0 scope. Core version information identifies a reserved Cargo crate, not an implemented VM.

## Next dependency

Architectural review of M0. After review, the next safely scoped task is M1 PLC Runtime Foundation: specify IR/type/scan semantics, deterministic clock/scheduling, bounded execution, source mappings, trace ordering, and fault behavior before implementation. No M1 work has been performed.

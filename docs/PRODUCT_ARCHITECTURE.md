# Product architecture

This document describes the architecture that is implemented today. Decisions are
recorded in the [ADRs](adr/README.md); ADR-0018 to ADR-0025 cover the product rebuild.

## Stack (kept)

Tauri 2 desktop host, Rust core, React + TypeScript user interface, Monaco for Structured
Text. The stack was re-evaluated during the rebuild and kept: the Rust core already held
the PLC semantics, Tauri keeps the web view sandboxed with a minimal capability set, and
nothing in the product requires another runtime (ADR-0001).

## Execution chain

```
ST source ─> st-frontend ─> canonical IR (plc-ir) ─> plc-runtime (one runtime)
                                                          │
lab-engine: world scheduler ── PLC participant ── station hardware ── process scene ── virtual network
     │                                   (hardware-model)      (process-simulation)   (network-core)
     ├─> causal cycle observations ─> compact trace (trace-core) ─> projections / waveforms
     └─> grading evidence ─> grading-core (behavioral rules)
desktop-session (application adapter, typed errors) ─> app-contract DTOs (IPC v8)
Tauri host (commands, pacing, trusted paths) ─> packages/ipc (strict validators) ─> packages/ui
```

## Crates and their responsibility

| Crate                | Responsibility                                                                 |
| -------------------- | ------------------------------------------------------------------------------ |
| `plc-types`          | PLC types and values, diagnostics, identities, standard block signatures       |
| `plc-ir`             | Canonical PLC IR and its validator                                             |
| `st-frontend`        | Structured Text lexer, parser, AST and compiler to IR                          |
| `plc-runtime`        | The only PLC runtime: transactional scans, budgets, standard blocks, clock     |
| `simulation-core`    | `WorldTime`, deterministic event scheduler, `SimulationClock`                  |
| `trace-core`         | Byte-bounded keyframe/delta signal trace and bounded event log                 |
| `lab-model`          | Versioned data: scenes, components, hardware, profiles, faults, exercises      |
| `process-simulation` | Exact fixed-point physics of scene components and process events               |
| `hardware-model`     | Configured vs installed modules, wiring, channel conversion, LEDs, diagnostics |
| `network-core`       | Deterministic virtual Ethernet segment and behavioral Modbus TCP client/server |
| `grading-core`       | Behavioral rule evaluation over grading evidence; troubleshooting scoring      |
| `lab-engine`         | Headless lab session: one world, cycle execution, observations, assessment     |
| `knowledge-core`     | Knowledge pack discovery, validation, cross-references and search index        |
| `project-model`      | `.plclab` project format, migrations, atomic store, learner progress           |
| `app-contract`       | IPC v8 transport DTOs only (serde, ts-rs); generates TypeScript declarations   |
| `desktop-session`    | Application adapter: use cases over engine, library and projects               |
| `plc-lab-desktop`    | Tauri host: lifecycle, trusted paths, command adapters, pacing loop            |
| `plc-lab-launcher`   | Root `Launcher.exe` window and the `PLC Lab.exe` alias (ADR-0026)              |

Dependency rules are enforced by `tests/architecture/boundaries.test.mjs` against the real
`cargo metadata` graph: semantic crates never depend on the contract, adapter, host or
Tauri; there is exactly one runtime crate; the engine owns no knowledge, persistence or
transport; grading never depends on the simulation; the contract is DTO-only.

## Time model (ADR-0018)

- **Host wall time** only paces the display loop (`execution_driver.rs`).
- **World time** (`WorldTime`, µs) is the authoritative chronology. The scheduler orders
  events by (time, phase, participant, sequence) with phases Stimulus < Fault < Network
  < Device.
- **PLC task time** (`SimulationClock`) advances only with committed scans.

## One cycle (ADR-0018, ADR-0019)

1. Scan with the input image staged at the end of the previous cycle.
2. Drive outputs at the cycle start (RUN: committed outputs plus forces; STOP/FAULT:
   configured stop behavior).
3. Advance the world by one cycle period, interleaving scheduled actions, network events
   and exact physics (skipped when the scan faulted).
4. Sample all inputs at the cycle end and stage them.
5. Record one **causal observation**: `scan_inputs`, `committed`, `applied`,
   `process_after`, `staged_next`, header and fault.

The invariant `output = f(scan_inputs)` holds inside each observation and is tested.

PLC modes are RUN, STOP and FAULT with a mode switch. A runtime fault puts the CPU in
FAULT until a memory reset. The world keeps running in STOP and FAULT. **Stop
Simulation** restores the plant's initial state and keeps the program, hardware
configuration and switch; it is not PLC STOP.

## Trace (ADR-0019)

Signals have stable `SignalId`s in a registry. The trace stores a keyframe every 64
records and per-layer deltas, under a byte budget (16 MiB default); the oldest records
are dropped first and reported. The event log is bounded (20 000 entries, 4 MiB).
Trends are change-point series decimated per time bucket (first, min, max, last).

## Application adapter and IPC (ADR-0020, ADR-0024)

`desktop-session` exposes one method per use case. It owns the single open workspace
(project + `LabSession` + simulation state + investigation), knowledge library, project
store and progress. Typed `AppError` values are mapped to `IpcError` codes only at its
public boundary. Continuous simulation uses a `RunLease`; the host calls `tick(lease)`
and receives a typed `TickOutcome` (stale, or advanced with the next period), so the host
never interprets error strings. A PLC fault pauses the simulation; an internal engine
error fails the workspace closed.

IPC v8 DTOs use `deny_unknown_fields`, carry the contract version and transport 64-bit
values as canonical decimal strings. `packages/ipc` validates every response with
combinators typed against the generated declarations (exact key sets, ranges, unions);
a drift between Rust and TypeScript fails compilation and the generated-file drift test.

## Host

The Tauri host resolves trusted locations: knowledge packs next to the executable
(`knowledge-packs/`), in the user's application data folder, and (development builds only)
the repository's `knowledge/` folder; projects and progress in the application data
folder (overridable with `PLC_LAB_DATA_DIR` for tests and portable use). The web view
has only event listen/unlisten permissions; there is no shell, filesystem, HTTP or
dialog plugin. CSP is `script-src 'self'` without `unsafe-eval`.

## User interface

`packages/ui` contains the shell (Home, Learn, Practice, Troubleshoot, Free Lab,
Knowledge, Progress, About) and the engineering workspace: project tree, Program /
Hardware / Process / Network / Diagnostics tabs, inspector (task, investigation, cause &
effect, properties) and dock (variables, watch, waveforms, events, messages). The UI
contains no PLC, physics or grading logic and never drives cycles (no timers drive the
simulation); it refreshes on backend wake events with at most one read in flight.

## Security

- `unsafe_code = "forbid"` for the whole workspace.
- Student code runs only in the bounded PLC VM (operation budget per scan, transactional
  rollback on faults); it never reaches the filesystem, network or shell.
- Knowledge pack loading never follows symbolic links and limits depth, file count and
  file size; a malformed pack is reported, not fatal.
- Markdown is rendered by a small safe renderer that creates React elements only (no
  HTML injection, no navigation).

## Distribution

`BUILD_PLC_LAB_RELEASE.bat` runs validation, builds the Tauri release, builds the
launcher and stages `releases/current/` (application, DLL audit, knowledge pack,
manifest with SHA-256 hashes) before installing the root `PLC Lab.exe`. See
[build and run](BUILD_AND_RUN.md) and ADR-0025.

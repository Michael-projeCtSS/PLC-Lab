# PLC Lab

M3A adds the manual Tank Filling process slice: Use Example (confirm) -> Compile -> Load -> Attach -> Single Scan. Rust owns the integer plant and sensors; React/SVG renders combined snapshots. Eight successful 100 ms scans reach 80%, and the next scan closes the inlet through PLC feedback. Runtime/Process reset are distinct; Run/Pause/Stop remain disabled. See [process specification](docs/spec/PROCESS_SIMULATION.md) and the updated human checklist. M3A stops for technical-lead source review.

Educational desktop workspace for PLC programming and industrial automation. **M1C/M2A accepted; M2B adds manual desktop runtime sessions.** Real ST source compiles through a Rust lexer, dedicated AST, semantic compiler and existing canonical IR validator. Explicit Load independently compiles and lowers it into the accepted transactional runtime. Manual Single Scan exposes real committed variables, TON state and source-backed faults without process/network simulation, trace recording or grading.

The working application uses Tauri 2, React, TypeScript, Rust and a locally bundled Monaco ST editor. It retains Project, Program, Variables, Process, Network, Timeline and Diagnostics. Compile remains pure; IPC v4 adds explicit Load, staged input editing, Single Scan, Reset Runtime and Unload. Run/Pause/Stop remain disabled. Values, count and simulation time come from Rust snapshots; click speed never controls PLC time. See [Alpha ST](docs/spec/STRUCTURED_TEXT_ALPHA.md), [session rules](docs/spec/DESKTOP_RUNTIME_SESSION.md) and [M2B report](docs/M2B_REPORT.md).

```powershell
npm ci
npm run dev
```

See [development](docs/DEVELOPMENT.md) for native prerequisites, build/test commands, the optional workspace-local Rust setup, and browser preview limitations.

For one-click Windows testing, use the four root BAT launchers described in [human testing](docs/HUMAN_TESTING.md) and its [checklist](docs/HUMAN_TEST_CHECKLIST.md).

See the [Sprint 0 completion report](docs/SPRINT_0_REPORT.md) for task acceptance and validation evidence.

## Engineering references

- [Persistent contract](AGENTS.md)
- [Architecture decision records](docs/adr/README.md)
- [PLC feature matrix](docs/spec/PLC_FEATURE_MATRIX.md)
- [Architecture and boundaries](docs/architecture/ARCHITECTURE.md)
- [Frontend/backend IPC](docs/architecture/FRONTEND_BACKEND_BOUNDARY.md)
- [Diagnostics](docs/spec/DIAGNOSTICS.md)
- [PLC type system](docs/spec/TYPE_SYSTEM.md)
- [Symbol model](docs/spec/SYMBOL_MODEL.md)
- [Source mapping](docs/spec/SOURCE_MAPPING.md)
- [Canonical semantic IR](docs/spec/CANONICAL_PLC_IR.md)
- [Runtime semantics](docs/spec/PLC_RUNTIME_SEMANTICS.md)
- [Test strategy](docs/spec/TEST_STRATEGY.md)
- [Roadmap](docs/ROADMAP.md)

M1B requires technical-lead code review before M1C. Desktop execution controls remain unavailable.

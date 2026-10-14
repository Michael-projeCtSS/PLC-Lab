# PLC Lab

Educational desktop workspace for PLC programming and industrial automation. **M1C independently validated; M2A Alpha ST compiler/editor implemented for review.** Real ST source compiles through a Rust lexer, dedicated AST, semantic compiler and existing canonical IR validator. Headless tests execute it through the accepted transactional runtime and TON. Desktop Compile returns diagnostics and declarations only; no desktop PLC execution, process/network simulation, trace recording or grading is implemented.

The working application uses Tauri 2, React, TypeScript, Rust and a locally bundled Monaco ST editor. It retains Project, Program, Variables, Process, Network, Timeline and Diagnostics. Compile uses real typed IPC v3; Run, Pause, Stop and Single Scan stay disabled. Variables shows declarations with unavailable runtime values. See [Alpha ST](docs/spec/STRUCTURED_TEXT_ALPHA.md) and [M2A report](docs/M2A_REPORT.md).

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

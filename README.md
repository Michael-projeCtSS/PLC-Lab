# PLC Lab

Educational desktop workspace for PLC programming and industrial automation. **M0/M1A approved; M1B headless runtime implemented for review.** Validated direct IR now lowers to owned plans and executes transactional, budgeted single scans with deterministic time. No language parser, timers, process/network simulation, trace recording, grading, or UI execution is implemented.

The working application uses Tauri 2, React, strict TypeScript, and Rust. It reserves Project, Program, Variables, Process, Network, Timeline, and Diagnostics areas. Run, Pause, Stop, and Single Scan are disabled. Diagnostics displays real Rust package information through a typed IPC contract.

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

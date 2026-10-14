# Frontend/backend boundary

## Ownership

React owns layout, accessibility, editor interactions, and rendering of supplied state. Rust owns all authoritative PLC semantics, timing, simulation, protocol behavior, traces, and grading. Tauri adapts narrow application operations to IPC; core crates do not import Tauri or React.

Sprint 0 exposes exactly one harmless read-only command. There is no execution, persistence, file, process, or network command. The main window has no plugin permissions and no remote-page capability. Custom registered application commands use Tauri's command mechanism; empty plugin permissions do not implement a PLC sandbox. See [Tauri commands](https://v2.tauri.app/develop/calling-rust/) and ADR-0004.

## Implemented typed exchange

```text
React LabShell
  → @plc-lab/ipc getCoreInfo()
  → invoke('get_core_info', { request: CoreInfoRequest })
  → Tauri command adapter
  → app-contract get_core_info()
  → Result<CoreInfo, IpcError>
  → serde JSON
  → TypeScript runtime validation
  → CoreInfo rendered in Diagnostics/status bar
```

Rust is the source of DTO definitions. [ts-rs](https://docs.rs/ts-rs/latest/ts_rs/) generates declarations and the command/version constants into packages/ipc/src/generated.ts. A drift test fails if that file differs from Rust output. TypeScript's static types cannot validate data arriving over IPC, so the adapter treats responses as unknown and checks their structure and contract version before exposing them to React.

| DTO             | Fields                                                                                                                                             |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| CoreInfoRequest | contractVersion: number, must be 2                                                                                                                 |
| CoreInfo        | applicationName, appVersion, coreVersion: nonempty strings; contractVersion: 2; execution: 'unavailable'; buildContext: 'development' or 'release' |
| IpcError        | code: string; message: string                                                                                                                      |

The command obtains the desktop version from its Cargo package and the core version from plc-runtime's Cargo package. The latter identifies the reserved core crate version and does not claim a VM is implemented. Serde rejects malformed requests and unknown fields. An unsupported well-formed contractVersion returns APP-IPC-001. No time, source evaluation, or hidden mutable state is involved.

buildContext is derived from the actual Rust debug_assertions build configuration, not UI settings or manually duplicated version values. Debug/native development builds report development; the standard optimized release profile reports release. The required new field advances this transport contract to version 2; old version 1 requests are explicitly rejected. No Canonical PLC IR semantics are involved. Human launchers reuse this command and expose context in the existing Diagnostics detail line.

## Failure behavior

| Code             | Meaning                                       | UI behavior                      |
| ---------------- | --------------------------------------------- | -------------------------------- |
| APP-IPC-001      | Rust rejects an unsupported contract version  | Show the Rust code and message   |
| APP-IPC-002      | Response shape/version does not match the DTO | Show incompatible-response error |
| APP-IPC-003      | Browser preview lacks native IPC              | Show native core unavailable     |
| APP-IPC-004      | Transport invocation fails                    | Show explicit transport failure  |
| APP-INTERNAL-001 | Unexpected application-side error             | Show an application error        |

Connecting, connected, and unavailable are explicit UI states. They never unlock execution controls in Sprint 0. Unmount guards prevent delayed replies from updating a destroyed shell. Unit mocks exist only in tests. The native smoke test verifies the compiled Rust handler through the actual Tauri IPC mechanism, including a rejected version and the absence of a run command.

## Future boundary rules

Later snapshots and trace batches must remain typed, versioned, and bounded. Transport must preserve authoritative scan/timestamp ordering. Large u64 timestamps use strings rather than lossy JS numbers. Cancellation, event subscriptions, backpressure, and diagnostics transport need explicit specifications before runtime exposure. Do not transport editor objects into the runtime or let renderer frame callbacks advance simulation semantics.

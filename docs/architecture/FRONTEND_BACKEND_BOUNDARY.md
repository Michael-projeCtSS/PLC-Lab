# Frontend/backend boundary

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [PRODUCT_ARCHITECTURE.md](../PRODUCT_ARCHITECTURE.md). Kept for engineering history.

## M4B / IPC v7

start_execution, pause_execution and stop_execution accept only strict contractVersion/sessionId and return coherent RuntimeSnapshotDto. executionMode (STOPPED/RUNNING/PAUSED) is separate from state (READY/FAULTED); executionRunId is read-only correlation metadata. TraceEntryDto carries the same control observation. No lease/epoch/tick can be submitted over IPC. execution-state-changed has only version/sessionId/runId; React must query get_runtime_snapshot and get_trace_page. Strict v7 validators reject older contracts, unknown fields, invalid modes/identities and FAULTED with an active mode. Main-window permissions add only local event listen/unlisten; no browser emit, remote access or OS plugin capability is granted. Rust remains authoritative for values/time/process/modes/history.

## M4A IPC v6 trace query

get_trace_page(version, sessionId, afterSequence, limit) dispatches through the existing blocking-worker adapter. Rust returns bounded ordered TraceEntryDto pages with exact decimal-string sequence/count/time, variable/TON/process/fault observations and retention/saturation metadata. Internal typed history is separate from DTOs. React validates, incrementally merges and displays historical observations only. Request/source/session guards discard obsolete pages. No polling, replay, state synthesis or execution authority crosses this boundary.

## M3A IPC v5 and authoritative Tank snapshots

app-contract v5 adds ProcessSceneKind TANK_FILLING, TankProcessSnapshotDto/ProcessSnapshotDto, InputAuthority USER/PROCESS and AttachProcessSceneRequest. RuntimeSnapshotDto includes required process:null-or-Tank and each variable includes inputAuthority (null for Output/Local). All existing load/input/scan/reset/observation responses and three new targeted scene mutations return one coherent combined snapshot. TIME/count/IDs/budget encoding and PlcReal remain unchanged.

attach_process_scene requires version/sessionId/sceneKind; detach_process_scene and reset_process_scene require version/sessionId. Thin Tauri blocking adapters call desktop-session; no physical engine lives in IPC or React. Rust validates bindings/authority before mutation. TypeScript strictly validates tags, fields, scalar ranges and ownership/scene metadata, while never deriving sensor states or future levels. UI rejects obsolete source/request/session replies through the existing generation checks.

Use Example only changes editor text after explicit replacement confirmation; Compile, Load, Attach and Single Scan remain independent. Attached Input sensors display PROCESS with no edit controls; Rust still rejects direct writes. SVG maps levelPermille to geometry and supplied booleans to labels. No timers or frame-based interpolation run the plant. Core/runtime/process diagnostics retain their distinct categories/families. See [process specification](../spec/PROCESS_SIMULATION.md) for the one-scan feedback delay, reset and failure policies. Earlier v4/v3 sections describe history.

## M2B IPC v4 and manual runtime

The generated contract advances to v4. Core information reports manual-single-scan capability; this does not create or load a program. Compile still returns metadata/diagnostics only. load_st_program accepts source/version, recompiles at the trusted boundary and returns loaded snapshot or compiler diagnostics; task configuration is fixed once in Rust and returned as metadata. set_runtime_input, single_scan, reset_runtime, unload_program and get_runtime_snapshot require the opaque session generation. Input writes additionally require the opaque variable ID and discriminated Alpha value.

TIME, IDs, count and budget use checked canonical decimal strings; REAL uses the existing finite binary32 wrapper. packages/ipc strictly validates fields, tags, ranges, IDs, source spans and coherent snapshot relationships. IpcTransportError is a compatible alias for CoreInfoError; compiler/runtime diagnostics remain data. Runtime mutation and snapshot construction happen in one application mutex observation. React does not increment time/count, evaluate ST, predict outputs or derive TON. Numeric drafts require Apply/Enter. Editing immediately invalidates the executable association; late replies cannot restore it.

## M2A compilation contract v3

Core information remains read-only with the same version-rejection behavior, now version 3. compile_st accepts contractVersion/source and runs the bounded Rust ST compiler on Tauri's blocking pool. Success contains success=true, programName, declaration symbols (name/typeName/role), diagnostics; failure contains success=false and diagnostics. Diagnostic spans retain one-based Unicode scalar start/end coordinates. No IR, ExecutionPlan, PLC memory or hidden runtime is transported/created. Normal student failures are successful IPC responses containing compiler data; transport/version/worker failures remain explicit application errors.

app-contract derives DTOs and generates TypeScript; packages/ipc validates responses before React consumes them. The frontend's source-spans converter adapts scalar columns to Monaco UTF-16. A monotonic editor revision/request identity prevents older responses from replacing newer markers/status. Only appearance, text editing, source navigation and declaration metadata belong in React; runtime controls stay disabled. Earlier foundation-only details below describe the preceding contract.

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

# Manual Tank Filling process

M3A provides one explicit educational scene, independent of PLC memory, UI and runtime semantics. The process-simulation crate owns the private level and applied actuator state; it depends only on plc-types. It contains no SymbolId bindings, clocks, Tauri, React, protocol or runtime calls.

## Model and arithmetic

Level is integer permille, initially 0, clamped to 0..1000. Fixed Rust configuration: Low <=200, High >=800, inlet 1000 permille/s, drain 800 permille/s. LevelPercent is finite binary32 level/10.0 through PlcReal; it is feedback only. Snapshots expose level, sensors, last applied actuator flags and read-only rates/thresholds. No private remainder or activation state crosses IPC.

For dt = cycle microseconds, independently compute each enabled rate * dt / 1,000,000 with checked i128 arithmetic, then add inlet, subtract drain and clamp. Integer division truncates each nonnegative contribution; fractional permille is discarded per step without accumulation. Even u64::MAX microseconds at these fixed rates fits i128. A zero-duration headless step changes no level. Default 100 ms gives +100, -80, +20 or 0 for inlet-only, drain-only, both or neither. No wall time, sleeps, floating integrated state or rendering cadence participates.

## Binding and authority

| Name (ASCII case-insensitive) | Role   | Type | Attached input authority |
| ----------------------------- | ------ | ---- | ------------------------ |
| LevelLow                      | Input  | BOOL | PROCESS                  |
| LevelHigh                     | Input  | BOOL | PROCESS                  |
| LevelPercent                  | Input  | REAL | PROCESS                  |
| InletValve                    | Output | BOOL | none                     |
| DrainPump                     | Output | BOOL | none                     |

Preserve original spelling. Extra variables are permitted. Never infer bindings by type/address/ID number. Resolve immutable descriptors originating in the accepted compiler/lowered memory layout, retain actual IDs privately and validate all roles/types and sensor staging preconditions before any attachment mutation. Inputs outside these bindings remain USER. Outputs/locals carry null inputAuthority. Direct manual writes to owned inputs are rejected before Runtime::set_input. Detach restores USER and retains the last staged values.

## Explicit lifecycle

attach_process_scene(version, sessionId, TANK_FILLING) requires Ready and all bindings, creates an empty plant, initializes applied actuator flags from actual committed outputs and stages initial sensors. Reattachment replaces a plant only after validation. Failed attachment preserves the current scene and complete runtime. reset_process_scene reinitializes tank/sensors while retaining last applied actuator flags and all PLC state/count/time/fault. detach_process_scene discards only the plant and ownership. Reset Runtime resets PLC state/count/time/fault using the existing core API, preserves plant and applied flags, and restages current physical sensors. Successful Load creates no scene; failed/stale Load preserves the current scene. Unload removes both.

## Authoritative cycle boundary

Sensors(t) already staged -> PLC scan at t -> committed Outputs(t), runtime count/time advanced -> physical evolution for exactly cyclePeriod -> Process(t+dt) -> derived/staged Sensors(t+dt) -> next scan. The application mutex covers all steps and the returned snapshot, so no observer sees partial staging. Failed PLC scans preserve the complete plant snapshot, applied actuators and sensors. Student faults and accepted internal runtime diagnostics remain their original snapshot data.

| Boundary | Level          | High staged | Inlet output / last applied |
| -------- | -------------- | ----------- | --------------------------- |
| Attach   | 0              | false       | false (default outputs)     |
| Scan 1   | 100            | false       | true                        |
| Scan 2   | 200, Low true  | false       | true                        |
| Scan 3   | 300, Low false | false       | true                        |
| Scan 7   | 700            | false       | true                        |
| Scan 8   | 800            | true        | true                        |
| Scan 9   | 800            | true        | false                       |

Scan 8 reads the preceding High=false. Its process interval creates High=true, which scan 9 reads. This one-scan feedback delay is intentional. Process labels report the most recently applied physical interval; Runtime reset changes PLC outputs but does not apply them to the plant until the next successful scan.

## Errors and IPC

Binding/ownership/state errors use a documented PLC-PROCESS application diagnostic family, not RuntimeFault. Impossible process/staging invariants use APP-PROCESS-INTERNAL and fail the application session closed until explicit replacement/unload. Checked model errors cannot occur for valid fixed configuration/durations. No ordinary simulated arithmetic fault or broad recovery framework is invented. Accepted ScanError::Internal transport and its future design note remain unchanged.

IPC v5 adds process: null or a TANK_FILLING snapshot, InputAuthority USER/PROCESS (null for non-inputs), and targeted attach/detach/reset process commands. Strict Rust request decoding and TypeScript payload/tag/range validators reject malformed/unknown fields. Snapshots contain no physical implementation internals. Runtime time/count/values are still authoritative core observations.

## Presentation and validation

The Process panel offers an explicit Tank Filling example and confirmation before replacing different editor text. Insertion only edits source and invalidates its existing compile/load association. Compile, Load, Attach and Single Scan remain separate. React maps supplied levelPermille to SVG height/percentage and supplied booleans to labels/classes; it never derives sensors or predicts future values. No animation or continuous loop exists. Old-source plants remain labeled STALE with execution/process mutation disabled.

Headless model boundary/determinism/maximum-duration tests, real-source binding/cycle/fault/reset/authority/lifecycle tests, strict contract tests, fixed DTO UI tests and actual native Windows 0->10->...->80%/next-scan closure are required. Both independent CI lanes retain all existing REAL/TON/compiler/load-intent/rollback checks. Stop after M3A for technical-lead source review.

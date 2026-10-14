# Human test checklist

## M4B continuous execution review

- [ ] Run starts automatic scans without manual clicks
- [ ] Run does not synchronously execute a cycle
- [ ] Pause freezes count, time, TON and process across several host periods
- [ ] Resume continues the preserved state with a fresh lease
- [ ] Stop freezes count, time, TON and process
- [ ] Stop does not reset PLC memory or staged inputs
- [ ] Stop does not reset the plant or sensors
- [ ] Single Scan is available after Stop and advances exactly one cycle
- [ ] Single Scan is disabled during Run and Pause
- [ ] Editor is read-only during Run and Pause
- [ ] Use Example, Compile and Load require Stop
- [ ] Attach, Detach and Reset Process require Stop
- [ ] Reset Runtime and Unload require Stop
- [ ] USER input staging remains available during Run and Pause
- [ ] PROCESS sensor editing is rejected in all modes
- [ ] Timeline refreshes from Rust pages during Run
- [ ] Historical selection remains pinned while live scans continue
- [ ] Automatic Tank scan 8 is 800 ms / 80% / High ACTIVE / Inlet OPEN
- [ ] Automatic Tank scan 9 is 900 ms / 80% / High ACTIVE / Inlet CLOSED
- [ ] Runtime fault automatically stops once and explicit Reset Runtime recovers
- [ ] Run/Pause/Stop use host time only for pacing; visible time remains SimulationClock

Earlier manual-only checklists below are historical; M4B supersedes their disabled Run/Pause/Stop expectations.

## M4A manual trace review

- [ ] Load creates the first Timeline event.
- [ ] Staged manual Input appears in Timeline.
- [ ] Attach Tank is a separate event.
- [ ] Each Single Scan adds exactly one cycle event.
- [ ] Sequence IDs increase monotonically, independent of scan count.
- [ ] Time is simulation time.
- [ ] Tank scan 8 shows 80%, High ACTIVE, Inlet OPEN.
- [ ] Tank scan 9 shows 80%, High ACTIVE, Inlet CLOSED.
- [ ] Selecting old history leaves the live tank unchanged.
- [ ] A runtime fault has code, source provenance, scan and time in history.
- [ ] Runtime Reset is a distinct reset event.
- [ ] Process Reset is distinct and preserves PLC time/count.
- [ ] Source edit marks old history STALE and disables historical source navigation.
- [ ] New Load starts a new trace.
- [ ] Unload clears Timeline.
- [ ] Dropped history and internal saturation are visibly identified.
- [ ] Run/Pause/Stop remain disabled.

## M3A manual Tank Filling

- [ ] Tank Filling Demo is discoverable in the Process panel.
- [ ] Example source replaces editor text only after explicit action/confirmation.
- [ ] Compile, Load Program and Attach Tank Scene remain separate actions.
- [ ] Attached tank starts empty, with Low active and High inactive.
- [ ] LevelLow/LevelHigh/LevelPercent visibly show PROCESS ownership.
- [ ] Process-owned sensors cannot be edited; unrelated Inputs remain editable.
- [ ] Single Scan opens the valve according to the real PLC output.
- [ ] Tank rises by exactly one deterministic step per successful scan.
- [ ] LevelPercent follows authoritative tank state.
- [ ] Low is active through 20% and becomes inactive above it.
- [ ] High activates at 80% on scan eight.
- [ ] PLC closes the inlet on the next scan; tank stays at 80%.
- [ ] PLC faults leave physical state and last applied actuator flags unchanged.
- [ ] Reset Runtime preserves the physical tank and restages its sensors.
- [ ] Reset Process empties the tank without resetting PLC time/count/outputs/locals/TON.
- [ ] Detach restores manual sensor ownership and preserves the loaded PLC.
- [ ] Run/Pause/Stop remain disabled.
- [ ] Tank, actuator/sensor labels and controls remain readable at normal desktop resolution.

The checklist is for manual review; automated native evidence is recorded separately. M3A stops for technical-lead source review before continuous execution.

## M2A Structured Text compilation

- [ ] ST editor opens and the editable default PROGRAM source is visible.
- [ ] Keywords, types, literals, comments and operators have syntax highlighting.
- [ ] Valid source compiles through the native backend and shows COMPILE SUCCESS.
- [ ] Variables shows Name/Type/Role declarations with —/Not running, never FALSE/0 values.
- [ ] Run/Pause/Stop/Single Scan remain disabled before and after compilation.
- [ ] A syntax error produces a backend code/message/location and correct Monaco marker.
- [ ] Clicking a diagnostic focuses its mapped source range.
- [ ] Editing/fixing source clears obsolete diagnostics; recompilation updates declarations.
- [ ] Mixed-case keywords compile and mixed-case variable/TON names resolve.
- [ ] A comment containing ą, ż, – and 😀 before an error does not shift its highlight.
- [ ] Local TON declaration and reordered named inputs/outputs compile.
- [ ] Empty/malformed/oversized source shows failure without freezing the application.
- [ ] Rapid edits during compilation do not display stale results.
- [ ] Compile never creates runtime values, scan counts or process/network animation.

No runtime RUN test is authorized for M2A.

Run automated validation first. Perform this checklist in DEV and then in RELEASE when testing a newly built release. Human visual testing complements automated checks and does not replace them. Future milestones must extend this checklist while retaining these foundation checks.

- Test date:
- Tester:
- Context shown in Diagnostics: DEVELOPMENT / RELEASE
- Application version:
- Core version:
- IPC contract version:
- Automated validation result:

## Startup

- [ ] The application opens through the selected root launcher.
- [ ] There is no immediate crash or unexpected disappearance.
- [ ] The window renders correctly at its default size.
- [ ] There is no blank page or indefinitely uninitialized application state.
- [ ] The terminal identifies DEV or RELEASE and displays any failure clearly.

## Main shell

- [ ] Project is visible.
- [ ] Program is visible.
- [ ] Variables is visible.
- [ ] Process is visible.
- [ ] Network is visible.
- [ ] Timeline is visible.
- [ ] Diagnostics is visible.

## Execution controls

- [ ] Run exists.
- [ ] Pause exists.
- [ ] Stop exists.
- [ ] Single Scan exists.
- [ ] All four controls remain disabled because the PLC Runtime is not implemented yet.
- [ ] No fake program execution, variable values, process behavior, or network activity is presented.

## Backend and build identity

- [ ] Diagnostics reaches CORE CONNECTED rather than staying CONNECTING or showing an IPC error.
- [ ] Application/core information is displayed after the real Rust IPC response arrives.
- [ ] Application/core versions and IPC contract version are visible.
- [ ] The context matches the launcher: DEVELOPMENT for DEV, RELEASE for the locally built release.
- [ ] The release can start without the Vite development server running.
- [ ] If no release exists, the release launcher asks me to run BUILD_PLC_LAB_RELEASE.bat and does not build automatically.

CORE CONNECTED and its version/context information are rendered from getCoreInfo supplied by Rust, not from handwritten UI version constants. Automated native acceptance verifies that real IPC boundary; this checklist verifies its visible outcome.

## Visual integrity

- [ ] Major panels do not overlap.
- [ ] Primary text is not obviously clipped.
- [ ] The layout is intact at the default application window size.
- [ ] Diagnostics and Timeline are readable.
- [ ] Resizing within the supported window limits keeps the workspace usable.

## Findings

- [ ] Any failure includes the mode, displayed versions, checklist item, and useful terminal error text.
- [ ] Issues discovered during human testing are resolved and relevant automated checks rerun before acceptance.

Notes:

```text
Record observations here. Empty Process/Network/Timeline states and disabled
execution controls are expected at M0; they are not missing runtime behavior.
```

## M2B manual runtime

- [ ] Compile default ST; no runtime values appear yet.
- [ ] Load Program explicitly; PROGRAM LOADED / READY, Scan 0 / time 0 appear.
- [ ] Single Scan is enabled; Run/Pause/Stop remain disabled.
- [ ] Change BOOL input and verify staging does not predict outputs.
- [ ] Apply INT/REAL/TIME numeric drafts explicitly; TIME uses integer microseconds.
- [ ] Single Scan shows actual committed Output/Local values and Rust time/count.
- [ ] Edit source after load: STALE / RELOAD REQUIRED; Single Scan/input editing disabled.
- [ ] Recompile/reload restores exact source association.
- [ ] TON 100 ms/300 ms four-scan sequence gives ET 0/100/200/300 ms and Q false/false/false/true.
- [ ] Division by zero shows PLC-RUNTIME-003 / FAULTED and marks the correct expression.
- [ ] Failed scan preserves committed output/time/count.
- [ ] Reset Runtime restores READY/defaults, including staged inputs, TON, count/time.
- [ ] Unload removes live values, preserves editor source and compile metadata, disables Single Scan.
- [ ] No process animation, background execution, trace/timeline or network state appears.

Human testing complements real native automated acceptance; no continuous Run test exists yet.

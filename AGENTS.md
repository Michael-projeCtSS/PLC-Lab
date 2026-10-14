# PLC LAB — CODEX EXECUTION CONTRACT

You are the primary implementation engineer for PLC Lab.

PLC Lab is an educational desktop application for learning industrial automation, PLC programming, process control, diagnostics, and industrial communication protocols.

The application is intended for both beginners and experienced automation engineers.

You are responsible for implementation, tests, technical documentation, repository maintenance, and validation.

You are NOT responsible for changing product architecture without explicit approval.

---

# 1. CORE PRODUCT PRINCIPLE

The most important product requirement is:

> Every meaningful PLC programming exercise must produce a visible and understandable result.

Visualization is a core architectural requirement, not an optional UI feature.

The canonical execution chain is:

PLC Source
→ Language Frontend
→ Canonical PLC IR
→ PLC Runtime
→ Process / Network Simulation
→ Trace
→ Visualization
→ Grading

Do not introduce architectural shortcuts that bypass this chain.

---

# 2. TARGET ARCHITECTURE

Main technology stack:

Desktop:
- Tauri 2

Frontend:
- React
- TypeScript

Backend / simulation core:
- Rust

Text editor:
- Monaco Editor

Graph-based editor:
- React Flow where appropriate

LAD:
- dedicated rung/grid-oriented editor model
- do NOT model LAD as generic React Flow nodes internally

The frontend must never contain the authoritative PLC execution logic.

The Rust core is authoritative for:
- PLC semantics
- execution
- timing
- simulation
- traces
- protocol behavior
- grading

---

# 3. NON-NEGOTIABLE ARCHITECTURAL RULES

## 3.1 One PLC runtime

There must NOT be separate runtimes for:

- ST
- LAD
- FBD
- STL

All supported languages must compile into one Canonical PLC Intermediate Representation.

Conceptually:

ST ─┐
LAD ├─> Canonical PLC IR → PLC Runtime
FBD ┤
STL ┘

Equivalent programs written in different languages must produce equivalent runtime behavior.

---

## 3.2 Vendor-neutral core

The core PLC model must be vendor-neutral.

Do not introduce Siemens-, Rockwell-, Schneider-, Beckhoff-, CODESYS-, or other vendor-specific behavior into the base runtime.

Future architecture may support profiles such as:

- Generic IEC
- Siemens-like
- CODESYS-like
- Rockwell-like

Vendor behavior must be implemented through explicit profiles or adapters.

---

## 3.3 Deterministic simulation

Simulation must be deterministic.

The same:
- program,
- initial state,
- inputs,
- simulation seed,
- timing configuration

must produce the same result.

Do not use wall-clock timing as the authoritative source for PLC semantics.

Create and use a SimulationClock abstraction.

Required modes:

- paused
- single scan
- real-time
- accelerated simulation

The UI rendering frame rate must never affect simulation results.

---

## 3.4 Student code must be sandboxed

PLC programs written by users must NOT be:

- executed as JavaScript,
- converted to executable native code and launched,
- allowed to access the filesystem,
- allowed to create arbitrary sockets,
- allowed to execute processes,
- allowed to call operating-system APIs.

PLC code must run only inside the PLC virtual machine/runtime.

The runtime must provide protection against:

- infinite loops,
- excessive recursion,
- excessive memory use,
- excessive execution per scan,
- invalid memory access.

At minimum implement execution budgets.

A faulty student program must be capable of stopping the simulated PLC task without freezing the desktop application.

---

# 4. LANGUAGE ARCHITECTURE

Supported target languages:

1. Structured Text — ST
2. Ladder Diagram — LAD
3. Function Block Diagram — FBD
4. Statement List / Instruction List educational profile — STL

Implementation order:

1. ST
2. LAD
3. FBD
4. STL

Do not implement all four simultaneously.

Each language frontend must eventually follow:

Source/Document Model
→ Parse/Validate
→ AST or logical representation
→ Semantic analysis
→ Canonical PLC IR

The runtime must never directly interpret UI objects.

---

# 5. PLC CORE

Design the runtime so that it can eventually support:

Data types:
- BOOL
- BYTE
- WORD
- DWORD
- SINT
- INT
- DINT
- USINT
- UINT
- UDINT
- REAL
- LREAL
- TIME
- ARRAY
- STRUCT

Program organization:
- Programs
- Functions
- Function Blocks
- FB instances

Memory categories:
- inputs
- outputs
- internal/global variables
- function/block local state
- instance state
- future retained state

Initial standard blocks:

- TON
- TOF
- TP
- R_TRIG
- F_TRIG
- CTU
- CTD
- CTUD

Do not implement features before they are required by an approved task.

---

# 6. PLC SCAN MODEL

The architecture must represent a PLC-like cyclic execution model.

Conceptually:

1. update/read process inputs
2. execute PLC task
3. update output process image
4. update simulated process/network
5. record trace/events
6. advance virtual time
7. repeat

The exact semantics must be documented and tested.

---

# 7. TRACE IS A CORE FEATURE

Tracing is mandatory infrastructure.

Do not add it later as a debugging convenience.

The runtime should eventually be capable of recording:

- simulation timestamp
- scan number
- input changes
- output changes
- variable changes
- executed source locations
- runtime faults
- process events
- network events

Trace data will support:

- online monitoring
- timeline
- replay
- grading
- diagnostics
- educational explanations

Source-to-IR mapping must therefore be preserved.

---

# 8. PROCESS SIMULATION

The process simulation must be independent from rendering.

Required separation:

PLC Runtime
→ Process Model
→ Renderer

Never make React components authoritative process models.

The first process component abstraction should support concepts such as:

- command inputs
- process state
- feedback signals
- faults

Initial simulated devices are expected to include:

- button
- switch
- lamp
- motor
- pump
- valve
- tank
- digital level sensor
- analog level sensor
- conveyor
- photoelectric sensor

Models should be educational and deterministic rather than unnecessarily physically complex.

---

# 9. NETWORK SIMULATION

Industrial communication must be based on a reusable virtual network kernel.

Conceptual architecture:

VirtualNetwork
- Node
- Interface
- Link
- Message/Frame
- Event scheduler
- Fault model

Network links should eventually support:

- latency
- jitter
- packet/message loss
- disconnect
- reconnect
- corruption where appropriate

Protocol implementations must use an adapter/plugin abstraction.

Never couple a protocol directly to visualization components.

Initial protocol implementation order:

1. Modbus TCP
2. Modbus RTU
3. OPC UA
4. CANopen
5. IO-Link
6. more advanced industrial protocols later

PROFINET, PROFIBUS, EtherNet/IP and EtherCAT must not be prematurely implemented as full industrial stacks.

Educational behavioral simulation and real-device interoperability are separate goals.

---

# 10. EDUCATIONAL ARCHITECTURE

Exercises must be data-driven.

An exercise should eventually describe:

- ID
- title
- difficulty
- learning objectives
- prerequisites
- allowed languages
- starter project
- process scene
- network scene
- stimuli
- expected behavior
- timing constraints
- hidden tests
- hints
- scoring configuration

Do not hard-code individual lessons into React views.

---

# 11. GRADING

Grading must evaluate behavior, not only final variable values.

Example:

incorrect:
Q0.0 == TRUE at the end

preferred:
START event at t=0
→ pump must start before deadline
→ process reaches target state
→ pump must stop
→ emergency condition must override normal control

Grading should eventually support:

- behavioral correctness
- timing
- safety behavior
- fault handling
- optional code-quality checks

Tests and grading must be deterministic.

---

# 12. VISUALIZATION

The architecture must support at least four visualization layers:

## Program view

Runtime state visible directly on:
- ST
- LAD
- FBD
- STL

## Process view

Animated process state:
- devices
- movement
- tank level
- flow
- actuator position
- faults

## Network view

Visual communication topology:
- nodes
- links
- active messages
- diagnostics
- protocol state

## Timeline

Time-oriented view of:
- PLC scans
- variable transitions
- process events
- network events
- faults

The UI must display authoritative state supplied by the runtime.

It must NOT independently re-evaluate PLC logic.

---

# 13. CODE QUALITY

Production code and technical documentation must be written in English.

User-facing localization architecture may support multiple languages later.

Requirements:

- strong typing
- explicit errors
- deterministic tests
- minimal hidden global state
- modular crates/packages
- clear public interfaces
- no unnecessary dependencies
- no giant god objects
- no silent error swallowing

Avoid TODO placeholders in completed tasks unless the TODO represents an explicitly documented future feature.

Do not use mocks where the task explicitly requires end-to-end functionality.

---

# 14. TESTING REQUIREMENTS

Every relevant module must include tests.

Use:

- unit tests
- boundary tests
- negative/error tests
- integration tests
- determinism tests where appropriate

Language features eventually require semantic equivalence tests.

Example:

Equivalent ST, LAD, FBD and STL logic supplied with the same input trace must produce the same output trace.

A feature is not complete merely because the UI demonstrates it once.

---

# 15. ARCHITECTURE DECISIONS

Major architectural decisions must be documented as ADRs.

Examples:

- runtime representation
- project serialization
- virtual clock
- simulation scheduling
- IR changes
- protocol adapter design
- persistence
- plugin architecture

If a requested implementation requires changing a previously approved fundamental decision:

1. do not silently change it,
2. create a proposed ADR,
3. document motivation,
4. document alternatives,
5. document migration impact.

---

# 16. DO NOT DO WITHOUT EXPLICIT APPROVAL

Do NOT independently:

- replace Tauri with Electron
- replace Rust runtime with TypeScript
- execute PLC logic in React
- introduce separate runtimes for PLC languages
- change Canonical PLC IR semantics
- introduce vendor-specific behavior into generic core
- introduce real-time OS dependencies
- add a 3D engine
- implement cloud services
- add authentication
- add multiplayer/collaboration
- implement AI-generated solutions
- implement physical-device connectivity
- implement full PROFINET/EtherCAT/EtherNet/IP stacks
- introduce a database server

These may become valid later but are not currently approved.

---

# 17. IMPLEMENTATION WORKFLOW

For every assigned task:

1. inspect existing repository state,
2. inspect relevant ADRs/specifications,
3. identify task dependencies,
4. implement only the requested scope,
5. add/update tests,
6. run formatting,
7. run lint/static analysis,
8. run relevant tests,
9. run broader regression tests when practical,
10. update technical documentation,
11. provide a completion report.

Do not ask questions for minor implementation choices.

Choose the simplest solution consistent with this contract.

If information is genuinely missing:
- make the smallest reasonable assumption,
- document the assumption,
- keep the implementation easy to change later.

Do not expand task scope merely because additional features seem useful.

---

# 18. COMPLETION REPORT FORMAT

After every task or task batch report:

## Completed
What was implemented.

## Files
Important files added/changed.

## Architecture
Any architectural decisions made.

## Tests
Commands executed and results.

## Acceptance criteria
For every requested criterion:
- PASS
- FAIL
- PARTIAL

with a short explanation.

## Assumptions
Any assumptions made.

## Known limitations
Only real limitations, not speculative future features.

## Next dependency
What task can safely start next.

Never report success if tests are failing.

---

# 19. PROJECT DELIVERY STRATEGY

Do not build horizontally.

Prefer vertical slices.

The first major proof of architecture must eventually demonstrate:

Structured Text
→ compiler
→ Canonical PLC IR
→ PLC VM
→ process model
→ tank/valve/pump simulation
→ visible process animation
→ runtime trace
→ automatic grading

This milestone is called:

DEMO-001 — Tank Filling Vertical Slice

Do not implement LAD/FBD/STL production editors before this vertical slice proves that the underlying runtime architecture works correctly.

---

# 20. PRIMARY PRIORITIES

When decisions conflict, use this order:

1. semantic correctness
2. determinism
3. safety/isolation
4. architecture maintainability
5. educational clarity
6. diagnostics
7. testability
8. UX
9. performance optimization
10. visual polish

Visual results are mandatory, but visual polish must never hide incorrect runtime behavior.

This document is the persistent engineering contract for PLC Lab.
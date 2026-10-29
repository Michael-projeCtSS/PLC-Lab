# Simulation fidelity

PLC Lab models behavior at the level needed to learn and to practice fault finding. Each
area below states what is exact, what is a **simulation approximation** and what is not
modeled. Platform profiles and protocol articles carry the same statement in the
knowledge pack (`fidelity` field).

## Fidelity levels

| Level                     | Meaning                                                             |
| ------------------------- | ------------------------------------------------------------------- |
| `EXACT_BEHAVIORAL`        | Behavior follows the stated rule exactly, deterministically         |
| `APPROXIMATED_BEHAVIORAL` | Same observable behavior class, simplified timing/values/details    |
| `CONCEPTUAL`              | Explained in the knowledge base; presentation only in the simulator |
| `NOT_IMPLEMENTED`         | Not simulated                                                       |

## PLC

- **Exact**: cyclic scan with input image, transactional commit (a faulting scan commits
  nothing), BOOL/INT (16-bit)/REAL (finite f32)/TIME (u64 µs) semantics, ST features
  (IF/ELSIF/ELSE, CASE, AND/OR/XOR/NOT, MOD, comparisons, conversions), standard blocks
  TON, TOF, TP, R_TRIG, F_TRIG, CTU, CTD on simulation time, operation budget per scan.
- **Approximated**: fixed cycle time (default 100 ms), one task, no retentive memory.
- **Generic profile choices**: REAL_TO_INT rounds half to even; F_TRIG reports no edge in
  the first scan; R_TRIG reports an edge if CLK is TRUE in the first scan; MOD is
  truncated and faults on division by zero.

## Process

Components: push button, selector switch, pilot lamp, motor starter (contactor,
auxiliary contact, thermal overload), on/off valve with travel time and limit switches,
pump, tank, level switch, level transmitter, presence sensor, variable frequency drive.
Volumes and flows use exact fixed-point integers (mL, mL/s), so results do not depend on
the cycle length. Process events: overflow, dry run, high-high, low-low, motor overload.
Real fluid dynamics, valve characteristics and motor torque are not modeled.

The **variable frequency drive** is `APPROXIMATED_BEHAVIORAL`, a generic behavioral model:
a run terminal, optional run enable, fault acknowledge and preset speed terminals, relay
outputs ready, running, remote and fault, a local (keypad) control state that ignores the
run terminal (fault `driveLocalControl`), a latched trip (`driveTrip`), a keypad speed
reference (`driveKeypadReference`), a mains supply (`driveNoSupply`), a linear ramp to
the speed reference, an output frequency shown on a 50 Hz basis and an approximate motor
current (20 % of rated current at standstill rising linearly to 100 %). Parameters,
fault codes, fieldbus control words, braking and safety functions such as Safe Torque Off
are not modeled. Switching from Local back to Remote while the run terminal
is ON starts the drive at once (real drives may require a new run edge, depending on their
parameters); deceleration is immediate.

## I/O hardware

- **Exact (model rules)**: configured vs installed modules per slot, missing/mismatched
  modules, missing field power, wire break detection on live-zero inputs, channel modes,
  output stop behavior, sinking/sourcing compatibility, terminal measurements.
- **Approximated**: analog conversion bands per profile (generic 0..10000; S7-style
  0..27648 with overrange to 32511 and overflow 32767; full-scale 0..32767; REAL
  engineering units), NE 43 transmitter saturation at 3.8/20.5 mA, fixed digital
  thresholds, diagnostic texts.
- **Not modeled**: module parameter sets of real products, hot swap, firmware versions.
- **Current loop power (generic model)**: a level transmitter is 2-wire (loop-powered) or
  4-wire (separately powered). A current input channel either supplies the loop
  (`loopSupply`, default) or is passive. A 2-wire transmitter conducts only on a supplying
  input and a 4-wire transmitter only on a passive one; the wrong combination carries
  0 mA. Real modules differ (separate 2-/4-wire terminals, external loop supplies, and
  possible damage when an active source meets a supplied input); the knowledge pack labels
  this as generic and points to the module manuals.

## Drives (generic VFD behavioral model)

Not a specific product. Rules (also in `process-simulation` `Drive`):

- `ready = mains supplied AND NOT faulted AND (run enable present OR no enable terminal)`.
- The drive accepts the run command (runs) only while `run command AND remote AND ready`;
  any change that breaks this condition stops it at once. A PLC output that is ON is
  therefore not a running motor: compare the PLC output, the drive's received command
  (terminal) and its running relay.
- Local control ignores the run terminal; returning to remote with the run command
  present starts the drive at once (real drives may require a new run edge).
- A trip latches. A rising edge of the reset terminal acknowledges it only when the cause
  is gone; a reset held while the trip occurs acknowledges nothing.
- Speed reference in per mille of rated speed (0..=1000, validated): keypad value when the
  keypad is the reference source, else the preset value while the preset terminal is
  energized, else the terminal value. The actual speed ramps in world time at 1000 per
  mille per ramp time from the current speed, drops at once to a lower reference and is 0
  while stopped or faulted. The display shows it on a 50 Hz basis.
- Without mains supply the display is dark and every relay (ready, running, remote, fault)
  is open; supply returns without an initialization delay.
- Not modeled: product parameters and fault codes, motor data and control modes, braking,
  safe torque off as a safety function, and fieldbus control with its communication
  monitoring (`CommunicationHealthy`), which belongs to the network diagnostics milestone.

## Commissioning checklist

- **Exact (model rules)**: every check is derived from observable simulated state
  (module status and LEDs, field supplies, diagnostics, input readings and quality,
  outputs, terminal voltages, device indications, network status, PLC mode) compared
  with the scene's documented wiring; it never reads injected faults (ADR-0027).
- **Approximated**: an input test passes when the drawn input is determined by the
  device (one value at rest, the other actuated); missing feedback is reported after an
  actuator was energized for 3 s; terminals are ideal 24 V or 0 V.
- **Not modeled**: safety-function validation, insulation and protective-earth tests,
  documented sign-off.

## Network

Modbus TCP is behavioral (`APPROXIMATED_BEHAVIORAL`): FC03/FC04 polling, transaction
identifiers, zero-based addressing, int16/float32 with word order, exception codes
01/02/0B, unit identifier handling, subnet and gateway rules, ARP resolution, duplicate
addresses, connection refused, timeouts, fixed latency. TCP segmentation, write function
codes, security and performance are not modeled. PROFINET, PROFIBUS DP, EtherNet/IP,
EtherCAT, OPC UA, CANopen, IO-Link and Modbus RTU are knowledge only (`NOT_IMPLEMENTED`).

## Vendor platforms

Profiles change addressing style, analog raw representation, product names and
terminology. The PLC is always PLC Lab's generic runtime; programs use PLC Lab ST, not the
vendor's dialect; vendor firmware, memory models and instruction sets are not emulated.
See [vendor profiles](VENDOR_PROFILES.md).

## Determinism

Same program, initial state, inputs, faults, seed and timing configuration produce the
same observations, events and grades. Analog noise faults use a seeded generator. Tests
in `crates/lab-engine/tests/engine.rs` compare complete observation sequences.

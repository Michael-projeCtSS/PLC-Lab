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
pump, tank, level switch, level transmitter, presence sensor. Volumes and flows use exact
fixed-point integers (mL, mL/s), so results do not depend on the cycle length. Process
events: overflow, dry run, high-high, low-low, motor overload. Real fluid dynamics,
valve characteristics and motor torque are not modeled.

## I/O hardware

- **Exact (model rules)**: configured vs installed modules per slot, missing/mismatched
  modules, missing field power, wire break detection on live-zero inputs, channel modes,
  output stop behavior, sinking/sourcing compatibility, terminal measurements.
- **Approximated**: analog conversion bands per profile (generic 0..10000; S7-style
  0..27648 with overrange to 32511 and overflow 32767; full-scale 0..32767; REAL
  engineering units), NE 43 transmitter saturation at 3.8/20.5 mA, fixed digital
  thresholds, diagnostic texts.
- **Not modeled**: module parameter sets of real products, hot swap, firmware versions.

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

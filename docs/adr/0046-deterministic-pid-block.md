# ADR-0046: A deterministic PID standard block

- Status: Accepted for 0.2.0-dev (CP9-B)
- Date: 2026-11-09

## Context

Closed-loop control is the next step after sequences and reusable blocks. IEC 61131-3 does
not define a PID block; vendors ship their own, with different names, parameters and
algorithms. PLC Lab needs one vendor-neutral, educational PID that runs in the one Rust
PLC runtime like TON or CTU, is called from the student's program, uses simulation time
only and gives the same result on every machine.

## Decision

`PID` is a standard function block of the runtime (`plc-types` signature, `plc-runtime`
state), called from Structured Text like the other standard blocks. It is a generic
educational approximation, not a model of any vendor's PID.

### Signature

| Id  | Parameter     | Direction | Type | Initial value | Meaning                                    |
| --- | ------------- | --------- | ---- | ------------- | ------------------------------------------ |
| 1   | SP            | input     | REAL | required      | Setpoint                                   |
| 2   | PV            | input     | REAL | required      | Process value                              |
| 3   | KP            | input     | REAL | 1.0           | Proportional gain (negative: reverse)      |
| 4   | TI            | input     | TIME | T#0s (no I)   | Integral (reset) time                      |
| 5   | TD            | input     | TIME | T#0s (no D)   | Derivative time                            |
| 6   | MV_MIN        | input     | REAL | 0.0           | Lower output limit                         |
| 7   | MV_MAX        | input     | REAL | 100.0         | Upper output limit                         |
| 8   | MANUAL        | input     | BOOL | FALSE         | Manual mode                                |
| 9   | MV_MAN        | input     | REAL | 0.0           | Manual output                              |
| 10  | ENABLE        | input     | BOOL | TRUE          | FALSE switches the controller off          |
| 11  | RESET         | input     | BOOL | FALSE         | Clears the integral and derivative history |
| 12  | TS            | input     | TIME | T#0s          | Sample time; T#0s samples at every call    |
| 13  | MV            | output    | REAL |               | Manipulated variable (controller output)   |
| 14  | E             | output    | REAL |               | Control error SP - PV of the last sample   |
| 15  | P, 16 I, 17 D | output    | REAL |               | The three parts of the output              |
| 18  | SAT           | output    | BOOL |               | The output is at a limit                   |

Optional inputs keep their last bound value, like every standard block.

### Algorithm (one call at the scan-start task time `now`)

1. `MV_MIN > MV_MAX` is a runtime fault (PLC-RUNTIME-007) and changes nothing.
2. `ENABLE = FALSE`: the output is 0 limited to the range, P, I and D are 0, SAT is
   FALSE, and the history is cleared, so enabling starts cleanly.
3. `MANUAL = TRUE`: `MV = limit(MV_MAN)` at every call. The integral tracks
   `MV - P - D`, so switching to automatic continues from the manual output without a
   bump (bumpless transfer).
4. Automatic: a sample is due at the first call, when `TS` is T#0s, or when `TS` has
   elapsed since the last sample. Between samples the outputs hold. At a sample, with `dt`
   the virtual time since the previous sample:
   - `e = SP - PV`, `P = KP * e`;
   - `D = -KP * TD / dt * (PV - PV_previous)`: derivative on the measurement, so a
     setpoint step gives no derivative kick; 0 at the first sample and with `TD` T#0s;
   - `RESET` clears the integral, the previous PV and the sample time before the
     sample, so while it is TRUE the integral stays 0 and every call is a first sample;
   - integral: `I = I + KP * dt / TI * e` when `TI` is not T#0s, unless the output is
     already beyond a limit and this change would drive it further (conditional
     integration: anti-windup). With `TI` T#0s, `I` is a constant bias (0, or what
     bumpless tracking left);
   - `u = P + I + D`, `MV = limit(u)`, `SAT = u` outside the limits.

### Determinism

All arithmetic is IEEE 754 binary32 (REAL) addition, subtraction, multiplication and
division in a fixed order, with no transcendental functions; Rust does not contract
operations into fused multiply-adds, so results are bit-identical on every supported
platform. Time comes only from the virtual task clock (`dt` and the sample schedule are
microsecond counts of simulation time). A non-finite intermediate value is the existing
REAL fault (PLC-RUNTIME-004). The block's state belongs to the scan transaction like
every standard block: a faulted scan rolls it back.

### Visibility

The outputs (MV, E, P, I, D, SAT) are block outputs like TON's Q and ET: they appear
online under the instance path (`Loop.MV`, `Loop.I`) and in trends, the timeline and
grading evidence without extra plumbing.

## Alternatives considered

- Velocity (incremental) form: anti-windup and bumpless transfer come for free, but the
  P, I and D parts that make the controller explainable are not visible.
- Back-calculation anti-windup: smoother, but needs a tracking gain students would have
  to tune as well; conditional integration is the common textbook choice.
- A derivative filter: real controllers filter D; the simulated measurement has no
  noise, so the filter is left out and documented as a limitation.
- A PID written in ST as a library POU: it would run in the same runtime, but
  REAL/TIME conversions are not available in PLC Lab ST yet, and a standard block can
  expose its parts and be validated once.

## Consequences

A student program controls a process with `Loop(SP := ..., PV := ..., KP := ...,
TI := T#20s, MV => Heater);`. Vendor PID behaviour (Siemens PID_Compact, Rockwell PIDE)
is not modelled; vendor profiles may map their names onto this block later.

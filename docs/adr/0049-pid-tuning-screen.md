# ADR-0049: Online write, control-loop query and the PID tuning screen

- Status: Accepted for 0.2.0-dev (CP9-B)
- Date: 2026-11-10

## Context

ADR-0046 gives PLC Lab a deterministic PID block, ADR-0047 a heated vessel it can
control. A student still had no way to tune a loop: the trend showed watched signals one
by one, gains were literals in the program, and changing one meant a new Build & Load,
which restarts the controller and the process. Commit 2d62e0c added the backend
groundwork (online write and a control-loop query) and cited this ADR before it was
written; it documents that groundwork and the screen built on it.

## Decision

### Observation (runtime)

The PID block reports, next to its outputs MV, E, P, I, D and SAT, the inputs a tuning
screen needs: SP, PV, KP, TI, TD, MV_MIN, MV_MAX, MANUAL and MV_MAN, as they were at the
block's last call. They appear online like block outputs (`Loop.KP`, `Loop.MV_MAX`) and
are recorded in the trace, so they are available to trends, the variable table and
grading evidence.

### Control-loop query

`control_loops` lists the PID instances of the program in the PLC: the instance path, the
signal of every observed parameter, and the inputs that are **bound to plain program
variables** (`KP := Kp`) with the signal of that variable. An input given as a literal or
an expression (`MANUAL := NOT AutoMode`) is observable but not writable.

### Online write

`modify` writes a variable or an output of the running PLC once, between scans: the next
scan starts from the written value, and a program that assigns the variable overwrites
it. Inputs are refused (the process image writes them; a force replaces an input). The
value must have the variable's type. Each write is logged as a coded `Modified` event in
the interface language. Writing the variable an input is bound to therefore changes the
controller's parameter from the next scan, without a download, a restart or a process
reset. A later Build & Load starts again from the value written in the program.

### Tuning screen

The workspace shows a **Tuning** view whenever the program in the PLC contains a PID
block. It is read again after every download.

- **Trend**: SP and PV on one scale, MV on a second scale between MV_MIN and MV_MAX, over
  60 s, 5 min or 15 min of simulation time, from the trace (`trend`).
- **Controller state**: SP, PV, E, MV, P, I, D and SAT, read with `watch` on every frame.
- **Parameters**: SP, KP, TI, TD, MV_MIN, MV_MAX, MV_MAN and MANUAL with their current
  values. A parameter bound to a variable has a Write field (TIME in seconds, a decimal
  comma accepted). A write is a command through the workspace lane (ADR-0048), so it is
  ordered with gestures and other commands. A literal or expression is shown as "set in
  the program".
- **Operator controls and disturbances**: the scene's selector switches (MAN/AUTO) and
  heated vessel doors are operated like any process gesture. Setpoint steps of ±5 and ±10
  write SP when it is bound.
- **Diagnostics**: sentences derived from the backend's values only. They cover manual
  mode with integral tracking (bumpless transfer), automatic mode with the current error,
  an output at MV_MAX or MV_MIN with the integral frozen (anti-windup), TI = 0 s (a P-only
  loop keeps an offset), TD = 0 s, and a simulation that is not running.

The view computes no control action. Every number on it comes from the Rust core, and
the controller arithmetic stays in the runtime.

### Exercise

`ex.heater-pid` (practice, advanced) starts with a P-only loop on the heated vessel that
settles at about 50.5 °C instead of 60 °C. The student finds a TI in the Tuning view, writes
it into the program, wires the MAN/AUTO selector to MANUAL and MV_MAN, and keeps the
output within its limits. Grading is behavioural, over three deterministic scenarios:

- heat-up in AUTO: scaling, reaching 58 °C by 120 s, staying within 58.5..61.5 °C from
  150 s, never above 65 °C, no PLC fault;
- the door opened at 180 s and left open: back within 58.5..61.5 °C from 330 s;
- 60 s in MAN, then AUTO: 20 % in MAN, and no output jump 0.5 s after the switch.

The conformance tests prove that each criterion measures what it claims. The reference
program passes. The starter fails exactly reaches, no-offset, recovers, manual and
bumpless. A program that switches the output with an IF instead of MANUAL fails only
bumpless. A swinging loop (TI 2 s) fails no-offset, and a 70 °C setpoint fails
no-overshoot.

## Alternatives considered

- **Writing block inputs directly.** It would need a second path into the block's state
  besides the program's call and would be overwritten at the next call anyway.
  Writing the bound variable uses the program's own data flow.
- **Tuning values in the project instead of the program.** It would hide the parameters
  from the program the student writes and grades. Writing them back into the program is
  part of the exercise.
- **Charting in the interface from sampled frames.** The frame rate would shape the
  curve. The trace records every cycle, so the trend is exact and does not depend on
  rendering.

## Consequences

- A loop can be tuned while it runs, and the effect of KP, TI, TD, the limits and manual
  mode is visible within seconds of simulation time.
- Only inputs bound to plain variables can be tuned online. Literals and expressions need
  a program change.
- There is no automatic tuning and no derivative filter (ADR-0046). Tuned values are not
  copied into the program automatically.

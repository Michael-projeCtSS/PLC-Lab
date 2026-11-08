# ADR-0043: Sequence execution and diagnostic model

- Status: Accepted for 0.2.0-dev
- Date: 2026-11-08

## Context

Students write step sequences in Structured Text (a step variable and `CASE`), but the
product could only show variables and the plant. Nothing told them which step was
active, what the step was waiting for, why a start was refused, which protective
condition applied, how long a step had been running or what happened before. Industrial
sequence training needs exactly that, without a second controller hiding mistakes in the
student's program.

## Decision

### Authority: the PLC program controls, the sequence monitor observes

- The student's PLC program is the only controller. It owns the step variable, the
  held state and every output command; physical actions travel only through the
  existing PLC, output, hardware and process chain.
- The engine's **sequence monitor** (crate `sequence-core`, driven by `lab-engine`)
  compares what the program does with a documented sequence definition. It reads the
  same cycle facts that grading reads and never writes PLC memory, inputs, outputs or
  the process. A timeout, a violated interlock or a hold therefore never changes the
  plant by itself: the program must react, and the monitor reports whether it did.
- The view only displays the monitor's state. It evaluates nothing.
- This is an educational diagnostic model. Interlocks here are not certified functional
  safety and do not stand for a safety PLC.

### Definition (data, in the exercise)

An exercise may carry one `sequence` (`lab_model::sequence::SequenceDefinition`):

- `id`, `name`; `stepSignal`: the evidence name of an `INT` program variable whose value
  is the active step number (read from the committed program memory, `outputs` layer);
  `heldSignal` (optional): the evidence name of a `BOOL` program variable that is TRUE
  while the program holds the sequence.
- `steps`: `id`, `number` (the step variable's value), `name`, `role` (`initial`
  exactly once, `final` at most once, otherwise `normal`), optional `timeoutMs` for
  normal steps.
- `transitions`: `id`, `from`, `to`, `name`, `condition`, optional `permissives`
  (`id`, `name`, `condition`). At most one transition per ordered pair of steps.
- `interlocks`: `id`, `name`, `condition` (TRUE = the protective condition is active),
  `reaction` (`blockStart`, `holdProgress`, `abortSequence`), `steps` (where it
  applies; `blockStart` applies to the initial step only) and, for `abortSequence`,
  `reactionMs`.
- Conditions are the exercise's typed condition model (`all`, `any`, `not`, `compare`
  of an evidence signal in a layer against a literal). No expressions, no strings to
  evaluate. Loading validates unique ids and numbers, every reference, that the step
  and held signals are evidence bound to program tags, and that every compared signal
  exists. One active step at a time; parallel branches and nested charts are not
  modeled.

### Evaluation: one snapshot per cycle

At the end of every cycle, after the scan, the output update, the process advance and
the input sample, and before alarm evaluation, the monitor observes the cycle facts:
`inputs` (the image the scan read), `outputs` (memory the scan committed), `process`
(the plant after the cycle) and `next` (the image the next scan reads). The step and
held values come from `outputs`; transition, permissive and interlock conditions are
evaluated on the same facts. A step change committed by scan N is judged against the
facts of cycle N: the inputs the program decided on. There is no further delay and no
dependence on the view's refresh rate. Events of one cycle are ordered: run start, step
change (with its transition, permissive and interlock checks), hold or resume, interlock
edges, start refusals, timeout; each event carries a strictly increasing sequence
number.

### States

Sequence: `UNBOUND` (no loaded program provides the step variable), `STOPPED` (the CPU
is not executing scans), `IDLE` (initial step), `RUNNING` (a normal step), `HELD` (a
normal step while the held signal is TRUE), `COMPLETED` (final step), `FAULTED` (a
latched monitor fault). Exactly one applies; precedence: `UNBOUND`, `STOPPED`,
`FAULTED`, `COMPLETED`, `IDLE`, `HELD`, `RUNNING`.

Step: `INACTIVE`, `ACTIVE`, `HELD` (the active step while held), `COMPLETED` (left
through a defined transition in this run), `FAULTED` (the active step when the fault
latched). Only the active step is `ACTIVE`, `HELD` or `FAULTED`.

### Step changes

- Through a defined transition: `transitionCompleted` with the observed condition and
  permissive values. Leaving the initial step is `sequenceStarted`; entering the final
  step is `sequenceCompleted`. If the transition's condition was FALSE or a permissive
  was unsatisfied in that cycle, the monitor records a deviation (`conditionNotMet`,
  `permissiveViolated`).
- Back to the initial step from a normal or held step without a defined transition: an
  abort, always allowed (Stop must always be possible): `sequenceAborted`, naming an
  active `abortSequence` interlock if one applies.
- Any other change: deviation `undefinedTransition`. A step variable value that matches
  no step: deviation `unknownStep`.

### Permissives and interlocks

- A permissive gates one transition. While the source step is active and the
  transition's condition holds but a permissive does not, the monitor records
  `startBlocked` once (when the refusal begins) with the unsatisfied permissives; the
  program is expected not to advance. Permissives are never faults.
- An interlock is monitored continuously in the steps where it applies;
  `interlockActivated` and `interlockCleared` are recorded on its edges.
  - `blockStart`: while active, the sequence must not leave the initial step.
  - `holdProgress`: while active in one of its steps, the sequence must not advance
    (an abort to the initial step is allowed).
  - `abortSequence`: while active in one of its steps, the program must return to the
    initial step within `reactionMs` of world time.
    A breach records deviation `interlockViolated`.

### Timeouts (world time)

A normal step's elapsed time is the world time of executed cycles that ended with the
step active and not held; STOP and HOLD pause it, and it restarts from zero on every
activation. At the end of the cycle in which elapsed time reaches `timeoutMs`, the
monitor records `stepTimeout` once and faults. Host time and view timers are never
used.

### Faults

Every deviation and every timeout latches `FAULTED` with its reason. While faulted, a
step change other than a return to the initial step records `progressAfterFault`: a
faulted sequence never continues silently. Returning to the initial step clears the
fault (`faultCleared`); the completed-step marks reset when the sequence starts again.

### HOLD and RESUME

- HOLD is the program's decision, read from the held signal. The monitor pauses the
  step timer, shows the step `HELD` and expects no progress: a step change other than an
  abort records `progressDuringHold`.
- Outstanding transition conditions are not latched: on RESUME every condition is
  evaluated again in the next snapshot. Resuming while an applicable `holdProgress` or
  `abortSequence` interlock is active records `interlockViolated`.
- What happens to the outputs during HOLD is the program's job (the exercise specifies
  it); the inspector labels that it observes and cannot hold the plant itself.
- HOLD is not PLC STOP: in STOP no scan runs and the sequence is `STOPPED`; in HOLD the
  program keeps scanning and keeps the process safe.

### PLC starts and resets

Every transition into RUN is a start (ADR-0038). The first executed scan after a start
begins a new monitor run (`runStarted` with the start kind): timers, interlock edges,
completed-step marks and faults are reset and the active step is taken from the
program memory, so a retained step variable resumes after a warm restart and a cold
restart, memory reset or download starts from the initial value. STOP and a CPU fault
show `STOPPED` and pause timers. A world reset creates a new session: a new monitor and
history. Forces act as they do for the program: a forced input is what the scan read.

### Alarms and history

The monitor feeds the existing alarm engine (ADR-0037), source `sequence.<id>`,
category `PROCESS`: `sequence.<id>.step-timeout` (ALARM; active from the timeout until
the step changes or the run restarts), `sequence.<id>.deviation` (ALARM; active while a
deviation fault is latched) and `sequence.<id>.interlock.<interlock>` (WARNING; active while the
interlock is active where it applies). Alarm lifecycles, acknowledgment and history are
unchanged; a held condition activates once. Every sequence event is also written to the
session event log. The monitor keeps the newest 200 events and counts all. Event codes,
step, transition, permissive and interlock ids are stable; texts are composed by the
view from codes and localized content.

### Persistence, IPC and grading

- Definitions are content (knowledge pack, localized by the source-guarded overlay).
  Monitor state and history are session state, never written to a project; no project
  format change.
- IPC contract 8, additive: the workspace frame carries the sequence (definition ids,
  state, active step, timers, evaluated conditions, interlocks, recent history) when the
  project's exercise defines one. No new command.
- Grading: `sequenceConforms` passes when the monitor recorded no deviation and no
  timeout in the scenario; `sequenceReached` passes when a given step was active by a
  given time. Both use the isolated assessment run, never clicks.

## Alternatives considered

A sequence supervisor that drives outputs (rejected: two owners of the plant, and it
would make a wrong student program look right); evaluating conditions in the view
(rejected: the backend owns semantics); string expressions in content (rejected:
untyped and unsafe); inferring steps from output patterns (rejected: ambiguous);
pausing the PLC for HOLD (rejected: HOLD and STOP are different operating concepts).

## Consequences

Sequences become observable, diagnosable and gradable without changing the runtime, the
IR or existing labs. Parallel branches, SFC editing, vendor sequence blocks (GRAPH,
SFC actions) and supervisor-driven sequences can be added later as explicit, separately
owned models.

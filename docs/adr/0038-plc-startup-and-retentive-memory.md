# ADR-0038: PLC startup, restart and retentive memory

- Status: Accepted for 0.2.0-dev
- Date: 2026-11-03

## Context

The engine already performed a warm restart on every STOP to RUN transition
(`PlcParticipant::warm_restart`), but nothing was retentive, so a warm restart and a
full initialization had the same effect, and `RETAIN` was only a reserved word. Students
could not observe the difference between a warm restart, a cold restart, a memory reset
and a download, nor write startup logic for the first scan.

## Decision

This is a generic educational model. Real PLC families differ (for example in what
STOP to RUN initializes, how retentive areas are configured and whether timers retain);
PLC Lab does not claim to reproduce any vendor.

- **Retention in the source.** Structured Text accepts `VAR RETAIN ... END_VAR` for
  program variables of the existing scalar types (BOOL, INT, REAL, TIME; no new types).
  Scalar declarations outside `VAR_INPUT` may carry a constant initializer `:= literal`
  of exactly their type and range; any other expression is refused. Function block
  instances are not retentive in this model (an educational fidelity boundary): declaring
  one in `VAR RETAIN` is refused, and so is `RETAIN` on inputs or outputs. Retentive
  instances may follow with the function block work of Checkpoint 9.
- **Canonical IR.** Every symbol carries its retention class (`RETAINED` or
  `NON_RETAINED`) and its initial value (the initializer, else the type default). The
  runtime builds every memory cell from that metadata; retention is never inferred from
  names or text.
- **Start kinds.** Every transition into RUN is a start, either warm or cold:
  - _STOP to RUN_ is a warm start (the existing behavior).
  - _Warm restart_ (operation): the CPU passes through STOP and starts warm.
  - _Cold restart_ (operation): the CPU passes through STOP and starts cold.
  - _Download_ of a program creates new memory; its first start is cold.
  - _Memory reset_ (existing contract): all program memory is initialized, the runtime
    fault and every force are cleared and the CPU leaves RUN; the next start is cold. With
    the mode switch in RUN that start follows at once, as before.

  - _Simulation reset_ (world reset) and _project reopen_ create a new session and
    download the program again: the first start is cold.

  One shared initialization runs exactly once per start. A warm start initializes
  non-retained variables and every function block instance, the program clock and the
  scan count, and keeps retained variables. A cold start initializes everything,
  retained variables included. The restart operations are offered only while the CPU is
  in RUN; in STOP, setting the switch to RUN is a warm start and a memory reset prepares a
  cold one. A CPU in FAULT cannot be restarted: a memory reset is required, as before. A
  restart uses the configuration and program already in the CPU and never downloads
  pending changes.

- **First scan.** A program may read the system variable `FIRST_SCAN` (BOOL, read-only,
  reserved: a declaration of that name is refused). It is TRUE during exactly the first
  executed scan after each start and FALSE in every other scan. Only an executed scan
  consumes it: STOP, a paused simulation and observation queries do not. The engine
  reports whether that scan follows a warm or a cold start.
- **Outputs.** Unchanged: in STOP and between a restart and the first scan, outputs follow
  the configured STOP behavior of each channel (off or hold last); the first scan's
  committed values drive them. A restart never changes the process or its feedback.
- **Forces** survive warm and cold restarts and STOP to RUN (the FORCES ACTIVE banner stays
  visible); a memory reset clears them (ADR-0033).
- **Diagnostics.** The CPU diagnostic buffer records, as single events, each start (warm
  or cold, with the number of retained values kept), each memory reset and each first
  scan, besides the existing mode changes and downloads. Nothing is recorded per scan.
- **Persistence.** Retention is part of the program source; no project format change.
  Retained values are session state: reopening a project starts a new session, cold.
- **Assessment.** Scenarios may schedule `warmRestart` and `coldRestart` actions, so labs
  are graded by isolated runs, never by clicks.

## Alternatives considered

Making STOP to RUN a resume (rejected: it would silently change existing labs); retaining
function block state (rejected for now: timer state over a restart has no clear generic
meaning; retentive instances belong to the FB instance work of Checkpoint 9); inferring
retention from variable names (rejected); persisting retained values in the project
(rejected: they belong to a running CPU, not to the project).

## Consequences

Startup logic can be taught and graded. Retentive function block instances, retentive
outputs and configurable retentive ranges can be added later through explicit metadata.

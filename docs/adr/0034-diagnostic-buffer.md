# ADR-0034: The CPU diagnostic buffer as an engineering tool

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29

## Context

The CPU diagnostic buffer recorded incoming and outgoing conditions of modules,
channels, network connections and the CPU (fault, force active), but not the CPU's
operating state changes or downloads, and the view showed a flat list: the engineer had
to work out which entries belong together, how severe they are and which module they
concern.

## Decision

- One bounded CPU buffer (200 entries, newest kept) in the station model. Besides
  incoming and outgoing conditions it records single events (`EVENT`) from the engine:
  the CPU operating state changes (RUN, STOP, FAULT, with the state before and after),
  program downloads and hardware configuration downloads. Module missing and restored,
  wrong module, field supply lost and restored, wire break, network lost and restored,
  device timeouts and Modbus exceptions keep their existing incoming/outgoing pairs. A
  module without electronics supply stays reported as not answering (missing): the CPU
  cannot tell the two apart.
- The backend derives, per entry: severity (incoming faults are errors; limits, forces,
  unconfigured modules and Modbus exceptions warnings; cleared conditions and events
  information), category (CPU, hardware, network), structured details (module id,
  channel id and address, configured and installed module, mode before and after,
  exception code), and for an incoming condition whether it is still active or when its
  outgoing entry cleared it.
- A typed `diagnostic_buffer` query returns the buffer newest first, filtered by
  severity, category and module id, in pages of at most 200, with the capacity and the
  number ever recorded. The view only chooses filters and pages.

## Alternatives considered

A second event log for the view (rejected: two truths); pairing entries in the view
(rejected: the backend owns the semantics); an unbounded history (rejected: memory and
IPC size).

## Consequences

Watchdog and scan-time diagnostics can be added as further kinds once the runtime models
them. Entries keep typed slot, channel and module ids, so future editors can link a
diagnostic to the program elements that use the module.

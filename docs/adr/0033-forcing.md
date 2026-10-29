# ADR-0033: Forcing the PLC input and output images

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29

## Context

Forcing is a commissioning tool with safety consequences: a forced value replaces what
the process or the program would supply. It must be visible at all times, behave the same
way in every session and only be offered where the runtime models it correctly.

## Decision

- Two force targets, named in the API and the view:
  - **PLC input image**: the program reads the forced value instead of the channel's or
    the network's for a PLC input tag;
  - **PLC output image**: the module receives the forced value instead of the program's
    for a PLC output tag.

  Internal variables (locals, block state) and the physical field state are not
  forceable: the runtime cannot hold such a value correctly against the program's own
  writes, and the field is changed by operating devices, not by forcing.

- Forces are typed: the value must have the tag's type (BOOL, INT, REAL), a REAL must be
  finite. The backend validates every request; the view never fakes a force.
- Lifetime, explicit and tested: forces survive STOP/RUN and program downloads that keep
  the symbol; a download that removes the symbol ends its force; a memory reset (PLC
  reset) removes every force; forces belong to the workspace session and a reopened
  workspace starts without any. Every removal is a logged force event.
- Visibility: while any force is active the workspace shows a persistent **FORCES
  ACTIVE** banner with the count, the list (signal, canonical path, what it replaces,
  value), Remove per force and **Remove all forces** (one request). The CPU FORCE LED, the
  toolbar count and the watch table's Forced column show the same backend state.

## Alternatives considered

Persisting forces in the project (rejected: a forgotten force would silently return on
the next opening); forcing locals (rejected: the program overwrites them in the same scan,
so the shown value would not be the one used).

## Consequences

The canonical signal ids used by watch tables (ADR-0032) are the force targets, so future
LAD, FBD, SFC, CFC and STL editors force the same signals without owning their identity.

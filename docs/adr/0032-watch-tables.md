# ADR-0032: Watch tables over canonical runtime signals

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29

## Context

The workspace had one flat watch list of signal paths that showed only a current value.
Engineering work needs several named tables, more detail per row (type, quality, where
the value comes from, the PLC's scan input, committed value and next input, an active
force) and rows that keep working when the program is written in another language or a
module moves.

## Decision

- Project format 5 replaces `watch` with `watchTables`: up to 16 named tables of up to 64
  rows. A row names a canonical signal path (`plc.<symbol>`, `plc.<instance>.<output>`,
  `process.<component>.<quantity>`, `io.<moduleId>.ch<n>`) and an integer display format
  (auto, decimal, hexadecimal, binary). Format 4 watch lists become the table "Watch 1".
  Rows never refer to editor text, so the same table serves ST and the future LAD, FBD,
  SFC, CFC and STL editors, which resolve their elements to the same signals.
- A typed `watch` query returns, for up to 64 signal ids, the runtime's view of each
  signal: canonical path, label, kind, value type, unit, active flag, value, scan input,
  committed value, next input, source (channel address or Modbus connection), quality and
  force. Unknown ids are left out. The view resolves rows to ids from the signal list and
  shows rows that do not resolve (for example after a rebuild removed a symbol) as not
  available.
- The Watch panel shows the tables with configurable columns (a per-viewer preference);
  table and row edits are project saves validated by the backend. The active table also
  feeds the live frame and the waveforms.

## Alternatives considered

Keeping one list (rejected: no named tables); storing rows by signal id (rejected: ids
are per session); computing row details in the view from several projections (rejected:
the backend owns values, quality and forces).

## Consequences

Forcing (CP7-D) and online variable detail (CP7-F) can use the same canonical rows and
the same query.

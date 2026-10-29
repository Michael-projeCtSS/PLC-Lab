# ADR-0029: Hardware configuration, stable module identity and derived findings

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29

## Context

Station building and hardware fault finding need three descriptions of one station kept
apart: the **project configuration** (`StationConfig`, what the engineer downloads), the
**installation** (the scene's modules and wiring plus injected faults, what is physically
there) and the **drawing** (the scene's documented devices, signals and wiring). Before
this decision modules were identified only by their slot, module electronics and field
supply were one condition, analog outputs had no channel model, and the commissioning
checklist compared configuration and installation with its own rules.

## Decision

- **Stable identity.** Every configured module has an `id` (an identifier, unique per
  station). Channels are identified as `<moduleId>.ch<n>`. Moving a module in the
  configuration (`StationConfig::move_module`, mirrored by the UI editor) keeps its id,
  its channel settings and the I/O mappings of its channels. Files without ids are given
  `<class>-r<rack>s<slot>` deterministically (`default_module_id`).
- **Project format 2.** `formatVersion` 2 adds module ids; version 1 files are migrated
  on load (`migrate_v1`), version 0 through version 1.
- **Canonical I/O identity, project format 3.** Five concepts stay separate: the module
  id (stable identity), the slot (current topology position), the process-image address
  (runtime mapping), the display address (profile rendering such as `%IX0.4`) and the
  signal id (runtime signal identity). I/O channel signals are registered under
  `io.<moduleId>.ch<n>`; the rack/slot position is only their label, so a moved module
  keeps its signal ids and every watch or force reference to them. Channel signals that
  no longer exist after a download are retired. Format 3 stores watch entries in that
  form; format 2 entries `io.R<r>/S<s>/CH<c>` are migrated through the project's own
  station, and an entry whose position has no configured channel is refused with the
  entry named instead of guessed (`migrate_v2`). Language editors (ST, LAD, FBD, SFC,
  CFC, STL) reference these canonical signals; none of them owns hardware identity. Progress and recent-project files
  keep their own version constants, so the project bump does not discard them.
- **Two supplies per module.** `InstalledModule.module_power` (electronics, default on)
  is separate from `field_power`. A module without electronics supply is present but does
  not answer (`NO_MODULE_POWER`, LEDs dark, reported to the CPU like a missing module);
  without field supply it answers and reports the missing load voltage (`NO_FIELD_POWER`).
  Fault `modulePowerMissing` injects the former.
- **Analog outputs.** AO channels bind to `INT`/`REAL` outputs, are driven only in RUN
  with an `OK` module and field supply, convert the raw command with the inverse of the
  profile's input bands and are measurable at the terminal.
- **Derived findings.** `StationRuntime::compare(scene)` derives typed findings
  (module missing, unexpected module, wrong type, wrong slot, module power missing, field
  power missing, channel mode and loop supply against the drawing, address overlap for
  byte-oriented address schemes, mapping to a missing/disabled/unconfigured channel or
  duplicated). Nothing is stored or injected as a finding; the commissioning checklist
  consumes the same findings instead of its own comparison rules.
- **Projection.** The hardware view (IPC contract 8, additive) carries module and channel
  ids, the findings and a signal map (PLC symbol, channel id, address, value and quality,
  drawn device and port, the device's own state). The UI shows Compare, Configured
  (project), Actual (detected) and Signals views and a module inspector (Overview,
  Configuration, Channels, Power, Diagnostics); module cards are buttons. Advanced mode
  adds, removes, moves and retypes modules; every edit is a project save that the
  backend validates, and the UI shows the returned project. The project's (offline)
  configuration reaches the PLC on download; until then the view says it differs and
  Compare shows the configuration loaded in the PLC.
- **Typed diagnostic identity.** Diagnostic buffer entries carry `slot`, `channel` and
  `moduleId` as structured data; the UI never recovers hardware identity from the
  display text in `source`.
- **Generic indicators.** CPU and module LEDs follow PLC Lab rules and are labeled as
  generic diagnostic indicators, not a vendor's LED behavior.

## Alternatives considered

Identifying modules by slot only (rejected: moving a module would lose its settings and
mappings); storing mismatch diagnostics in the installation (rejected: findings must be
derived so they cannot drift from the state they describe and cannot leak injected
faults); vendor module catalogs (rejected: the core stays vendor-neutral; profiles only
name products).

## Consequences

Hardware labs (`scene.hardware-station`, labs `hw.*`) can contrast field supply, module
electronics, wrong type, wrong slot, wiring and channel mode faults with distinct
evidence. Wiring faults are not findings (the PLC cannot detect where a wire is landed);
they show only through testing. Module start-up times, hot swap and vendor module
parameters remain outside the model (SIMULATION_FIDELITY).

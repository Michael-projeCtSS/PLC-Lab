# Hardware model

`hardware-model` simulates the station between the PLC program and the field.

## Module catalog (generic types)

`cpu.compact`, `cpu.modular`, `ps.24vdc`, `di8.24vdc.sink`, `di16.24vdc.sink`,
`di8.24vdc.source`, `do8.24vdc`, `do16.24vdc`, `ai4.ui`, `ai8.ui`, `ao2.ui`. Profiles map
them to product names for presentation.

## Configured versus installed

The **station configuration** (`StationConfig`: CPU, modules with a stable id, start
address and channel configuration, I/O map, network) is what the project says. The
**installation** (from the scene plus injected faults) is what is physically there. Per
slot the runtime reports `OK`, `MISSING`, `MISMATCH`, `NO_FIELD_POWER`, `NO_MODULE_POWER`
or `UNCONFIGURED`; only `OK` modules exchange data (inputs of other modules hold their
substitute value 0 with quality `MODULE_FAULT`). Downloading a configuration whose CPU
differs from the installed CPU is rejected (ADR-0029).

Module ids are stable: `<class>-r<rack>s<slot>` by default, unique per station, kept when
a module is moved. Channels are identified as `<moduleId>.ch<n>`.

## Two supplies

Every I/O module has an **electronics supply** (`modulePower`, typically the backplane)
and a **field supply** (`fieldPower`). Without electronics supply the module is present
but silent: status `NO_MODULE_POWER`, LEDs dark, reported to the CPU like a missing
module. Without field supply it answers and reports missing load voltage: outputs cannot
switch their terminals and inputs cannot supply their sensors.

## Findings

`StationRuntime::compare(scene)` derives the differences between project, installation
and drawing as typed findings: `MODULE_MISSING`, `UNEXPECTED_MODULE`, `WRONG_MODULE_TYPE`,
`WRONG_SLOT` (a configured slot is empty and the same type sits in an unconfigured slot),
`MODULE_POWER_MISSING`, `FIELD_POWER_MISSING`, `CHANNEL_MODE_MISMATCH` and
`LOOP_SUPPLY_MISMATCH` (against the drawn device), `ADDRESS_OVERLAP` (byte-oriented
address schemes) and `SIGNAL_BINDING_MISMATCH` (mapping to a missing, disabled or
unconfigured channel, or duplicated). Findings are never stored or injected; the
commissioning checklist uses them too. A wire landed on another terminal is not a finding:
the PLC cannot see it, only testing can.

## Channels

- **Digital inputs**: the field signal (contact, PNP/NPN sensor) is evaluated against the
  input style (sinking/sourcing); incompatible combinations never switch. Channel LEDs
  show the electrical state.
- **Digital outputs**: switch only with field power and an `OK` module; in STOP/FAULT the
  configured stop behavior (off or hold last) applies. The chain PLC tag, module command,
  field supply, terminal, actuator and feedback is visible in the signal map.
- **Analog outputs**: `INT`/`REAL` commands are converted with the inverse of the profile's
  input bands to mA or V at the terminal, only in RUN with an `OK` module and field supply;
  the commanded value is read back with its quality.
- **Analog inputs**: modes `current4To20mA`, `current0To20mA`, `voltage0To10V`,
  `disabled`; conversion to the profile's representation with over/underrange bands;
  live-zero inputs with diagnostics report wire break; values carry a quality (`GOOD`,
  `WIRE_BREAK`, `OVERFLOW`, `UNDERFLOW`, `OVERRANGE`, `UNDERRANGE`, `MODULE_FAULT`, ...).
- **Measurement**: the multimeter returns terminal voltage (mV) or loop current (µA).

## Diagnostics

A bounded CPU diagnostic buffer records time-stamped incoming/outgoing events for
modules, channels, connections and the CPU (module missing or mismatched, no field power,
wire break, overflow, underflow, connection timeout/refused/no route, Modbus exceptions,
address conflict, link down, PLC fault, force active). CPU LEDs: RUN/STOP, ERROR, FORCE
and LINK.

## Addressing

Addresses are formatted per profile (`hardware_model::address::format`) from the module
type, its start address and the channel: IEC direct, byte.bit, module-defined tags, linked
terminals, octal devices or unit ports.

## Editing in the UI

The Hardware tab has four views: **Compare** (the detected station with its findings),
**Configured (project)**, **Actual (detected)** and **Signals** (PLC symbol, channel id,
address, value and quality, drawn device and its state). Selecting a module (module cards
are buttons) opens the inspector with Overview, Configuration, Channels (values, LEDs,
multimeter), Power (electronics and field supply) and Diagnostics (this module's findings
and buffer entries). CPU and module LEDs are generic PLC Lab indicators.

Advanced mode edits a draft of the station: add a module (type and free slot; a stable id
and a first-fit start address are proposed), change a module type, move a module to a free
slot (its id and mappings move along), remove a module (with its mappings), analog channel
modes; network interface and Modbus connections in the Network tab. The draft is
downloaded with the program through Build & Load; the backend validates it before
anything changes and reports any remaining overlap as a finding.

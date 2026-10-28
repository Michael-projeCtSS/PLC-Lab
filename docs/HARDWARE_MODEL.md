# Hardware model

`hardware-model` simulates the station between the PLC program and the field.

## Module catalog (generic types)

`cpu.compact`, `cpu.modular`, `ps.24vdc`, `di8.24vdc.sink`, `di16.24vdc.sink`,
`di8.24vdc.source`, `do8.24vdc`, `do16.24vdc`, `ai4.ui`, `ai8.ui`, `ao2.ui`. Profiles map
them to product names for presentation.

## Configured versus installed

The **station configuration** (`StationConfig`: CPU, modules with start address and
channel configuration, I/O map, network) is what the project says. The **installation**
(from the scene plus injected faults) is what is physically there. Per slot the runtime
reports `OK`, `MISSING`, `MISMATCH`, `NO_FIELD_POWER` or `UNCONFIGURED`; only `OK` modules
exchange data. Downloading a configuration whose CPU differs from the installed CPU is
rejected.

## Channels

- **Digital inputs**: the field signal (contact, PNP/NPN sensor) is evaluated against the
  input style (sinking/sourcing); incompatible combinations never switch. Channel LEDs
  show the electrical state.
- **Digital outputs**: switch only with field power and an `OK` module; in STOP/FAULT the
  configured stop behavior (off or hold last) applies.
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

Advanced mode edits a draft of the station (configured module type, analog channel mode;
network interface and Modbus connections in the Network tab). The draft is downloaded
with the program through Build & Load; the backend validates it before anything changes.

# ADR-0028: Network diagnostics from the deterministic network model

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29

## Context

Commissioning and troubleshooting Modbus TCP needs the evidence an engineer uses: what
the PLC sends (function, address, quantity, unit ID), what comes back (data, exception,
nothing), when, and how the configuration compares with the device documentation. The
network kernel (`network-core`) already simulates this deterministically. The view must
not invent traffic, must stay bounded, and must not reveal a lab's injected fault.

## Decision

- No second network simulator. The workspace `network` projection (IPC contract 8,
  additive) is extended from authoritative state only:
  - each connection carries its request as downloaded (function and code, zero-based
    start address, quantity, port, period, timeout), its configured values decoded by the
    backend from the last good registers (INT, or REAL with its word order), and the times
    of the last request and response;
  - `traffic`: the newest 60 network events as structured records (connection,
    transaction, peer, kind, unit, function, address, quantity, round trip, exception
    code), built from the bounded engine event log;
  - each device reports whether its address lies in the PLC interface's subnet.
    The text-only list of recent messages is replaced by `traffic`.
- The Network view shows nodes, connections, an inspector for the selected connection
  (request, raw words in decimal and hexadecimal, decoded values) and the traffic table.
  Register numbers are also shown in the common 4xxxx / 3xxxx documentation convention,
  labeled as a convention that device manuals do not all follow. Exception codes are
  named with the standard Modbus meanings.
- Network labs are commissioning labs on `scene.network-commissioning`. Configuration
  faults (wrong server address, unit ID, timeout, word order, start address) ship as a
  wrong starter station that the student corrects and downloads; a switched-off device is
  an injected fault with a repair. The scene description carries the device
  documentation the student compares against.

## Alternatives considered

Rendering traffic from frame changes in the view (rejected: the UI must not reconstruct
protocol behavior); an unbounded traffic history (rejected: memory and IPC size); a
separate packet capture model (rejected: duplicates `network-core`).

## Consequences

Traffic shows only what the PLC's stack observes; a device's own state is visible only
through its node status, as on site. Writes, other function codes, TCP handshakes and
serial bus timing behind gateways remain outside the model (SIMULATION_FIDELITY).

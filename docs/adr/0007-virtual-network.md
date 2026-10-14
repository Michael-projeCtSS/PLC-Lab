# ADR-0007: Virtual industrial network

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-003, FND-007

## Context

Industrial protocol exercises need shared deterministic scheduling and faults instead of protocol logic embedded in network visualization.

## Decision

network-core will own a virtual network kernel with nodes, interfaces, links, frames/messages, an event scheduler, and fault models. Reuse SimulationClock ordering rather than creating protocol-specific clocks. Support latency, seeded jitter/loss, disconnect/reconnect, and corruption where applicable when requested.

Protocols use explicit Rust adapters over the kernel. Implementation order is Modbus TCP, Modbus RTU, OPC UA, CANopen, IO-Link, then later protocols. Network visualization consumes authoritative topology, state, and events. Virtual educational behavior and physical-device interoperability are separate goals; the latter is not approved.

## Alternatives

Real sockets for educational links introduce nondeterminism and unauthorized connectivity. Protocol-specific schedulers duplicate semantics. React-based adapters couple behavior to rendering. Reject these alternatives.

## Consequences and migration

Protocol adapters will require conformance, fault, timing, and determinism tests. Full PROFINET, PROFIBUS, EtherNet/IP, and EtherCAT stacks are outside current scope. No network implementation exists to migrate.

## Sprint boundary

Reserve network-core and Network UI only. No scheduler, simulated packets, or protocols.

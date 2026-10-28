# Network model

`network-core` is a deterministic virtual industrial network with a behavioral Modbus TCP
implementation. It is an educational model, not a conformance-tested stack.

## Topology

One switched Ethernet segment with a fixed one-way latency (scene `latencyUs`). The PLC
has an interface (IP, mask, optional gateway) from the station configuration; devices
come from the scene. Links of the PLC and of each device can be up or down; devices can
be offline or answer on a changed address (faults).

## Delivery rules

- A destination outside the PLC subnet without a gateway: **no route**.
- No device answers ARP for the address: **ARP unresolved**, then timeout.
- Several devices answer for one address: **address conflict**.
- A reachable device without a Modbus server: **connection refused**.
- No response within the configured timeout: **timeout**.

## Modbus TCP

The PLC is the client. Each configured scan polls a server periodically with FC03 (read
holding registers) or FC04 (read input registers), start address (zero-based) and
quantity (1..125), and maps response offsets to PLC input tags as int16 or float32 (high
or low word first). An optional status tag is TRUE while the scan succeeds. Servers map
register addresses to constants or scaled component quantities and answer with exception
01 (illegal function), 02 (illegal data address) or 0B (gateway target failed) where
appropriate; the unit identifier is honored or ignored per device. Values from a failed
scan are not refreshed and never reach the PLC as new data.

## Events and views

Every request, response, exception, timeout, no-route, ARP failure, refusal and address
conflict is an event in world time. The Network tab shows topology, connection counters,
last round trip, raw registers, recent messages and a ping tool; connection problems also
appear in the CPU diagnostic buffer.

## Other protocols

Modbus RTU, PROFINET, PROFIBUS DP, EtherNet/IP, EtherCAT, OPC UA, CANopen and IO-Link are
described in the knowledge base and marked `NOT_IMPLEMENTED`. New protocols are added as
adapters on the same event-driven kernel (ADR-0007), never coupled to the UI.

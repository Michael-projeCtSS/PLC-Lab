# ADR-0047: A heated vessel process and analog outputs into the process

- Status: Accepted for 0.2.0-dev (CP9-B)
- Date: 2026-11-09

## Context

The PID block (ADR-0046) needs a process it can control: an actuator that takes a
continuous command, a self-regulating process with lag and dead time, a disturbance and
a measurement. Until now analog signals flowed only into the PLC (transmitters into
analog inputs); the `ao2.ui` analog output module computed field currents that reached no
component.

## Decision

### Analog sinks

A new port kind `AnalogSink` consumes a current or voltage from an analog output channel.
Wiring validation allows an analog output terminal on an analog sink port. After the
analog outputs are computed in each cycle, the hardware model drives every wired analog
sink with its channel's field value; a sink whose channel drives nothing (no module, an
open circuit, the CPU not in RUN) receives 0 mA. A signal of the wrong kind (a voltage
into a current input) is no signal; values outside the range saturate at 0 or 100 %.

### Heated vessel

`heatedVessel` is a generic educational model of an electrically heated vessel with a
built-in temperature transmitter (ports `power` and `temperature`):

- the heater power command (per mille) reaches the heating element after a dead time;
- element and vessel are two first-order lags of the rise above ambient:
  `element' = (gain * power - element) / elementLag`,
  `vessel' = (element - vessel - doorLoss * vessel) / vesselLag`;
- the measured temperature is `ambient + vessel`; the transmitter maps
  `rangeMinC..rangeMaxC` to its signal with the same saturation and faults as the level
  transmitter;
- the door, opened and closed by the operator or a scenario (`setDoor`), adds a heat
  loss of `doorLossPermille` of the vessel's rise: a disturbance;
- the heater cannot cool: power is limited to 0..100 % (actuator saturation).

Quantities: `temperatureMilliC`, `elementMilliC`, `powerPermille`, `doorOpen`, `signal`
(`m°C` is shown as °C).

### Determinism

The process scene integrates continuous quantities exactly between transitions so that
results do not depend on how time is partitioned. A lag is exponential and cannot be
integrated exactly with integers, so the vessel advances on a fixed grid of world time
(every 10 ms at absolute multiples) with explicit Euler steps in integer
micro-degrees, rounded half away from zero. State changes only at grid instants, whatever
the caller's step sizes; commands apply from the next grid instant. The dead time is a
queue of grid samples.

## Alternatives considered

- Exact exponential integration in floating point: not bit-reproducible across libm
  implementations.
- Euler steps at the caller's step size: results would depend on the cycle time and on
  how the engine partitions an interval.
- A separate temperature transmitter component like the level transmitter: more general,
  but a heated vessel without its measurement has no use in the exercises yet.

## Consequences

Analog output modules now drive the process, so the hardware and commissioning
material can later cover analog output channels (wrong mode, open circuit). The
heated vessel scene `scene.heater-loop` (AI for TT-1, AO for the heater controller, a
MAN/AUTO selector and two lamps) is the plant of the PID exercises.

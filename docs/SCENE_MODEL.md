# Scene model

A scene (`lab-model::scene::SceneDefinition`, file format `plclab-scene`, schema 1) is
the complete physical side of a lab:

- **components**: physical devices with typed parameters (see below);
- **couplings**: process connections (valve fills tank, valve drains tank, pump drains tank);
- **installation**: the CPU and modules physically installed, with field power per module
  and the **wiring** from component ports to module channels;
- **referenceStation**: the configuration an engineer would create for the installation
  (modules, channel modes, I/O map, network);
- **network** (optional): devices on the Ethernet segment, e.g. a Modbus TCP server with a
  register map bound to component quantities;
- **interface**: the recommended PLC tags with type, direction and description;
- **layout**: HMI positions on a 1000 x 600 canvas (presentation only);
- **knowledgeRefs**.

## Components

| Type               | Ports                              | Quantities                                                                                  |
| ------------------ | ---------------------------------- | ------------------------------------------------------------------------------------------- |
| `pushButton`       | contact (NO/NC)                    | pressed                                                                                     |
| `selectorSwitch`   | position0..3                       | position                                                                                    |
| `pilotLamp`        | coil                               | lit, color                                                                                  |
| `motorStarter`     | coil, aux, overload                | command, running, speedPermille, currentMa, tripped                                         |
| `onOffValve`       | solenoid, openSwitch, closedSwitch | command, positionPermille, flowing                                                          |
| `pump`             | motor                              | command, running, dry, flowMlPerS                                                           |
| `tank`             | none                               | levelMl, levelPermille, capacityMl, spilledMl, overflowing, highHigh, lowLow, netFlowMlPerS |
| `levelSwitch`      | contact (NO/NC, PNP/NPN/dry)       | wet, conducting                                                                             |
| `levelTransmitter` | signal (4-20 mA, 0-20 mA, 0-10 V)  | signal (µA or mV)                                                                           |
| `presenceSensor`   | output (NO/NC, PNP/NPN)            | present, conducting                                                                         |

Operator actions: press/release buttons, set selector position, set presence target.

## Validation

`lab_model::validate_scene` checks identifiers, component references, wiring to existing
module channels and port kinds, station consistency and network references. Scenes in a
knowledge pack that fail validation are rejected with a reason.

## v1 scenes

`scene.io-basics` (buttons, selector, proximity sensor, lamps), `scene.motor-start-stop`
(start/stop, contactor with feedback and overload, run lamp), `scene.tank-filling` (inlet
valve, drain pump, tank, two level switches, 4-20 mA transmitter), `scene.tank-modbus`
(tank with a Modbus TCP level transmitter and a high-high switch).

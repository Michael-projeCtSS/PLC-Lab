# Troubleshooting model

Troubleshooting labs present a working installation with a **cause**, never a scripted
symptom. Symptoms emerge from the same simulation that runs every other exercise.

## Causes

- **Physical faults** (`lab-model::fault::FaultSpec`), injected into the installation and
  invisible in the UI except through their effects: sensor stuck, contact type mismatch
  (NO/NC), output style mismatch (PNP/NPN), analog drift/stuck/noise, loop break, valve
  stuck, feedback missing, motor overload trip, open circuit, short to supply, swapped
  wires, module missing, wrong module, module in wrong slot, field power missing, network
  link down, PLC link down, device offline, device address changed.
- **Configuration errors**, seeded into the starting project (`starterStation`) exactly as
  they occur in commissioning: wrong analog measuring range, wrong Modbus start address,
  wrong IP, wrong mapping. They are fixed by editing the configuration and downloading.

## Exercise data

`troubleshooting` in an exercise defines the symptom text, who reported it, hidden
faults, diagnosis options with the root causes, repair options (each repair clears listed
faults) and the criteria that verify the repaired system. The PLC is in RUN with the
plant's original program when the lab opens.

## Evidence

The learner works with the same tools as on site: process view, input/output LEDs and
values, terminal measurements (multimeter), variables with process images, watch and
waveforms, CPU diagnostic buffer, module status, network topology, connection status,
raw Modbus registers and ping. Opening the hardware, network and diagnostics views and
taking measurements or pings is recorded as evidence.

## Investigation record

Diagnoses, evidence and repairs are stored in the project (`exerciseState`) and survive
closing and reopening; repairs are reapplied when the lab reopens.

## Scoring (grading-core)

| Part         | Points | Rule                                                                                                   |
| ------------ | ------ | ------------------------------------------------------------------------------------------------------ |
| Diagnosis    | 30     | 30 for a correct first diagnosis, 15 after wrong attempts, 0 if none                                   |
| Repair       | 20     | all hidden faults cleared                                                                              |
| Verification | 50     | verification criteria, run in an isolated assessment with the faults still present in the installation |

Verification uses the program and configuration the learner provides; an unrepaired
fault therefore fails verification. The root cause is never shown before it is diagnosed.

## v1 labs

| Lab                        | Cause                                        | Fix           |
| -------------------------- | -------------------------------------------- | ------------- |
| `ts.motor-wont-start`      | NC Stop contact block replaced by NO         | repair        |
| `ts.outputs-dead`          | Output module without load supply            | repair        |
| `ts.part-sensor-inverted`  | Replacement sensor normally closed           | repair        |
| `ts.tank-level-reads-high` | Analog channel configured 0-20 mA            | configuration |
| `ts.remote-level-lost`     | Transmitter address in another subnet        | repair        |
| `ts.remote-level-wrong`    | Modbus start address shifted by one register | configuration |

A variable-frequency drive left in local control is described in the knowledge base;
drives are not simulated yet, so there is no lab for it.

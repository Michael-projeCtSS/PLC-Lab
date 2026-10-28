# Troubleshooting model

Troubleshooting labs present a working installation with a **cause**, never a scripted
symptom. Symptoms emerge from the same simulation that runs every other exercise.

## Causes

- **Physical faults** (`lab-model::fault::FaultSpec`), injected into the installation and
  invisible in the UI except through their effects: sensor stuck, contact type mismatch
  (NO/NC), output style mismatch (PNP/NPN), analog drift/stuck/noise, loop break, valve
  stuck, feedback missing, motor overload trip, drive left in local control, open
  circuit, short to supply, swapped wires, module missing, wrong module, module in wrong
  slot, field power missing, network link down, PLC link down, device offline, device
  address changed.
- **Configuration errors**, seeded into the starting project (`starterStation`) exactly as
  they occur in commissioning: wrong analog measuring range, wrong Modbus start address,
  wrong IP, wrong mapping. They are fixed by editing the configuration and downloading.

## Exercise data

`troubleshooting` in an exercise defines the symptom text, who reported it, hidden
faults, diagnosis options with the root causes, repair options (each repair clears listed
faults), the criteria that verify the repaired system and a **debrief**. The PLC is in
RUN with the plant's original program when the lab opens.

## Debrief

Every lab has a debrief (`troubleshooting.debrief`): the root cause, the evidence that
pointed to it, the correct repair and how to reason about such symptoms. The check use
case returns it only with a passed verification, so it never reveals the cause early.
Each evidence statement describes behavior the simulation actually produces for the
lab's fault (conformance test `troubleshooting_exercises_fail_until_repaired_and_pass_afterwards`
requires a complete debrief).

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

| Lab                        | Cause                                         | Fix           |
| -------------------------- | --------------------------------------------- | ------------- |
| `ts.motor-wont-start`      | NC Stop contact block replaced by NO          | repair        |
| `ts.outputs-dead`          | Output module without load supply             | repair        |
| `ts.part-sensor-inverted`  | Replacement sensor normally closed            | repair        |
| `ts.tank-level-reads-high` | Analog channel configured 0-20 mA             | configuration |
| `ts.vfd-local`             | Conveyor drive left in Local (keypad) control | repair        |
| `ts.remote-level-lost`     | Transmitter address in another subnet         | repair        |
| `ts.remote-level-wrong`    | Modbus start address shifted by one register  | configuration |

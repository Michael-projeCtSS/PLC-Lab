# Learning model

## Path

Learn (concepts) -> Practice (programming) -> Troubleshoot (diagnosis) -> Free Lab
(exploration). Every lesson links to a simulation or an exercise; every exercise links to
knowledge articles; troubleshooting guides link to the labs that simulate them.

## Lessons

A lesson (`lessons/*.json`) has a number, level, duration, objectives, sections and a
checkpoint. Section types: `concept`, `visual` (diagram), `example` (code), `mistake`,
`challenge` (question with explanation), `simulation` (opens a scene in a sandbox
project), `task` (starts an exercise) and `summary` (with a troubleshooting note).
Completing a lesson requires answering its checkpoint; the score is stored in progress.

| #   | Lesson                          | Exercise or scene                    |
| --- | ------------------------------- | ------------------------------------ |
| 1   | What is a PLC?                  | I/O panel                            |
| 2   | Inputs and outputs              | ex.lamp-pushbutton                   |
| 3   | The PLC scan cycle              | tank station (Step)                  |
| 4   | BOOL logic                      | I/O panel                            |
| 5   | The start/stop circuit          | motor station                        |
| 6   | Latching and seal-in            | ex.motor-start-stop                  |
| 7   | Interlocks                      | ex.tank-batch-sequence               |
| 8   | Timers                          | ex.delayed-start                     |
| 9   | Counters and edges              | ex.button-counter                    |
| 10  | Sensors                         | ex.part-counter                      |
| 11  | Actuators                       | tank station                         |
| 12  | Analog signals                  | tank station                         |
| 13  | The 4-20 mA loop                | ex.tank-analog-scaling               |
| 14  | Scaling                         | ex.tank-analog-scaling               |
| 15  | Hardware modules and racks      | tank station (Hardware tab)          |
| 16  | Addressing                      | motor station with platform profiles |
| 17  | Diagnostics                     | ts.outputs-dead                      |
| 18  | Industrial communication basics | ex.modbus-level                      |
| 19  | Commissioning workflow          | ts.tank-level-reads-high             |
| 20  | Troubleshooting methodology     | ts.motor-wont-start                  |

## Exercises

Data-driven (`exercises/*.json`): starter program, optional starter station, required
interface, instructions, hints (revealed one at a time), knowledge references, evidence
bindings, scenarios and behavioral criteria. The UI never hard-codes exercise ids.

## Basic and Advanced modes

Basic mode offers one **Build & Load to PLC** button (compile, link, download, PLC RUN,
simulation run). Advanced mode separates compile and download, exposes the PLC mode
switch, memory reset and forcing, and enables hardware and network configuration editing.

## Feedback design

Feedback explains _what happened_: the cause-and-effect panel shows the PLC read, logic
result, process change and next input for each cycle; check results name the failing
criterion with concrete timing; troubleshooting feedback never reveals the root cause.

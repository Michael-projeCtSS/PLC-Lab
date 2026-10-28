# Human test checklist

Use for a manual session on the development app (`RUN_PLC_LAB_DEV.bat`) and again on the
release (`PLC Lab.exe`). Record date, build (About page) and any failed item.

## Shell

- [ ] The window opens with HOME, LEARN, PRACTICE, TROUBLESHOOT, FREE LAB, KNOWLEDGE, PROGRESS.
- [ ] No internal terms (milestones, IPC versions, crate names) outside the About page.
- [ ] About shows the build (development or release), versions and the knowledge pack as LOADED.

## Learn

- [ ] Lesson 1 shows explanation, diagram, example, mistake and question; answering shows the explanation.
- [ ] "Open this machine in a sandbox" opens a workspace; "Start the exercise" opens the exercise.
- [ ] Completing a lesson marks it done in Learn, Home and Progress.

## Practice: motor start/stop

- [ ] Start the exercise; the Task inspector shows instructions, interface and checks.
- [ ] Type a wrong program: the editor marks the error; Build & Load reports it and loads nothing.
- [ ] Type the seal-in solution and press Build & Load: PLC RUN, simulation running.
- [ ] Hold Start S1 in the Process view: the motor runs and keeps running after release.
- [ ] Cause & effect shows PLC read, logic result, process change and next input.
- [ ] PLC STOP stops the outputs while the simulation keeps running; Stop (simulation) resets the plant.
- [ ] Check solution: all checks pass; Progress shows the score.

## Troubleshoot: conveyor will not start

- [ ] The lab opens with the PLC in RUN and a fault report, without naming the cause.
- [ ] Hardware: the Stop input LED is off at rest; Measure shows 0 V on that terminal.
- [ ] A wrong diagnosis is accepted as an attempt; the right one is confirmed.
- [ ] The correct repair makes the conveyor start; Verify repair passes; the score reflects attempts.

## Free Lab and hardware/network

- [ ] Create a free lab on the tank station with the Siemens S7-1200 profile: addresses show `%I0.0` style and analog raw values in the 0..27648 band.
- [ ] Advanced mode: change an analog channel to 0-20 mA and download; the value changes accordingly.
- [ ] Tank Modbus station: the Network tab shows requests/responses; ping 192.168.0.20 replies.
- [ ] Watch a tag and a process value; Waveforms plots them; Events lists operator actions.
- [ ] Save as, export and import a project; reopen it and find the program unchanged.

## Knowledge

- [ ] Search "4-20" finds the 4-20 mA article first; vendor and troubleshooting filters work.
- [ ] A vendor family article shows the simulation fidelity and sources with verification status.

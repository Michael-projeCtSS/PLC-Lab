# Product vision

PLC Lab teaches industrial automation through visible cause and effect. A learner with
no PLC knowledge must be able to start the application, follow a lesson, write a first
program, load it into a simulated PLC and see a machine react, and later diagnose a
faulty installation using the same evidence an engineer would use on site.

## Audiences

- **Beginners** (vocational training, career changers): guided lessons, Basic mode with
  one **Build & Load to PLC** button, explanations next to every value.
- **Students and junior engineers**: exercises graded on behavior, hardware and network
  configuration, analog scaling, sequences.
- **Experienced engineers**: Free Lab, Advanced mode (separate compile, download, PLC
  mode switch, memory reset, forcing), troubleshooting labs, vendor-style addressing.

## Principles

1. **Every exercise produces a visible result**: process animation, LEDs, values, waveforms.
2. **The core is authoritative**: PLC semantics, timing, physics, network behavior and
   grading run in Rust; the UI never recomputes them.
3. **Causality is explicit**: each cycle records what the PLC read, what it committed,
   what the actuators did, how the process changed and what will be read next.
4. **Determinism**: the same program, inputs, faults and timing always produce the same
   observations and the same grade. Wall-clock time only paces the display.
5. **Honesty about fidelity**: simplifications are labeled _simulation approximation_,
   unverified facts are labeled _unverified_, unimplemented protocols are labeled as such.
6. **Troubleshooting is first-class**: faults are physical or configuration causes in the
   simulated installation, never scripted symptoms; root causes are not revealed.
7. **Data-driven content**: scenes, exercises, lessons, platform profiles and knowledge
   are versioned pack data, not code.

## What success looks like

- A beginner completes lessons 1-6 and the motor start/stop exercise in one session.
- A learner diagnoses the "conveyor will not start" lab with a correct first diagnosis
  by comparing the Stop input image with a terminal measurement.
- An engineer configures a Modbus TCP connection, sees the request/response traffic and
  finds an off-by-one register error from raw register values.

## Not in scope today

Real-device connectivity, vendor firmware emulation, full PROFINET/EtherCAT/EtherNet/IP
stacks, LAD/FBD/STL editors, multi-user features, cloud services and AI-generated
solutions. See the [roadmap](ROADMAP.md).

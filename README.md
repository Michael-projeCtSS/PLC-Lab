# PLC Lab

PLC Lab is a desktop application for learning industrial automation by doing. You write
PLC programs in Structured Text, load them into a simulated PLC, operate a simulated
machine and watch every cause and effect: what the PLC read, what it decided, what the
machine did and what the PLC will read next. Troubleshooting labs train fault finding
with LEDs, a multimeter, diagnostic buffers and network status, the way commissioning
engineers work.

Everything that decides behavior runs in a deterministic Rust core: one PLC runtime, a
process simulation, I/O hardware, a virtual Modbus TCP network, traces and grading. The
React user interface only displays authoritative state and sends operator actions.

## Start PLC Lab

| What you want                       | Do this                                                              |
| ----------------------------------- | -------------------------------------------------------------------- |
| Use the application                 | Double-click **`PLC Lab.exe`** (or `RUN_PLC_LAB.bat`) in this folder |
| Build the release from source first | Double-click `BUILD_PLC_LAB_RELEASE.bat`, then start `PLC Lab.exe`   |
| Develop with hot reload             | `RUN_PLC_LAB_DEV.bat` or `npm run dev`                               |
| Run every check                     | `RUN_ALL_CHECKS.bat` or `npm run validate`                           |

`PLC Lab.exe` is a small launcher that starts the staged release in `releases/current/`.
It is created by the release build and is not stored in Git. Requirements: Windows 10/11
x64 with the Microsoft Edge WebView2 Runtime (preinstalled on current Windows). See
[build and run](docs/BUILD_AND_RUN.md).

## What is inside

- **Learn**: 20 lessons from "What is a PLC?" to troubleshooting methodology, each with
  explanation, diagram, example, common mistake, question, exercise link and checkpoint.
- **Practice**: data-driven exercises (pushbutton, motor seal-in, TON delays, counters,
  part counting, tank filling, analog scaling, a CASE batch sequence, Modbus TCP level).
  Programs are graded by running scripted scenarios in an isolated simulation and judging
  behavior over time.
- **Troubleshoot**: labs with hidden physical faults or commissioning errors (NC contact
  replaced by NO, missing field power, inverted sensor, wrong 4-20 mA range, device in
  the wrong subnet, Modbus register offset). The diagnostic path, repairs and the final
  verification are scored.
- **Free Lab**: an engineering workspace with program editor, hardware rack, HMI process
  view, network view, diagnostics, variables, watch, waveforms, events and cause & effect.
- **Knowledge**: a searchable, versioned knowledge pack with fundamentals, PLC platforms
  of eight vendors, nine industrial protocols and troubleshooting guides, with
  provenance records and explicit simulation-fidelity statements.
- **Progress**: completed lessons and exercise scores, stored locally.

The knowledge pack is also distributed as
[`releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip`](releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip).

## Honest scope

PLC Lab is an educational simulator. It does not emulate vendor firmware, does not
implement PROFINET, EtherCAT or EtherNet/IP stacks, does not connect to real devices and
claims no protocol conformance. Platform profiles present addresses, analog raw values
and product names in a vendor's style and declare their fidelity. See
[simulation fidelity](docs/SIMULATION_FIDELITY.md).

## Documentation

- [Product vision](docs/PRODUCT_VISION.md) and [product architecture](docs/PRODUCT_ARCHITECTURE.md)
- [Simulation fidelity](docs/SIMULATION_FIDELITY.md), [scene model](docs/SCENE_MODEL.md),
  [hardware model](docs/HARDWARE_MODEL.md), [network model](docs/NETWORK_MODEL.md)
- [Vendor profiles](docs/VENDOR_PROFILES.md), [knowledge pack](docs/KNOWLEDGE_PACK.md)
- [Learning model](docs/LEARNING_MODEL.md), [grading model](docs/GRADING_MODEL.md),
  [troubleshooting model](docs/TROUBLESHOOTING_MODEL.md)
- [Project format](docs/PROJECT_FORMAT.md)
- [Build and run](docs/BUILD_AND_RUN.md), [development](docs/DEVELOPMENT.md),
  [human testing](docs/HUMAN_TESTING.md)
- [Architecture decision records](docs/adr/README.md) and the
  [product rebuild report](docs/PRODUCT_REBUILD_REPORT.md)
- Engineering contract: [AGENTS.md](AGENTS.md)

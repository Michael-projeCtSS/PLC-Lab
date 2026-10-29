# PLC Lab

PLC Lab is a desktop application for learning industrial automation by doing. You write
PLC programs in Structured Text, load them into a simulated PLC, operate a simulated
machine and watch every cause and effect: what the PLC read, what it decided, what the
devices received, what the machine did and what the PLC will read next. Troubleshooting
labs train fault finding with LEDs, a multimeter, diagnostic buffers, drive displays and
network status, the way commissioning engineers work.

Everything that decides behavior runs in a deterministic Rust core: one PLC runtime, a
process simulation, I/O hardware, a virtual Modbus TCP network, traces and grading. The
React user interface only displays authoritative state and sends operator actions.

## Start PLC Lab

| What you want                       | Do this                                                                |
| ----------------------------------- | ---------------------------------------------------------------------- |
| Use the application                 | Double-click **`Launcher.exe`** in this folder, then **Start PLC Lab** |
| Open a project file                 | **Open Project...** in the launcher, or `Launcher.exe "<file.plclab>"` |
| Build the release from source first | Double-click `BUILD_PLC_LAB_RELEASE.bat`, then start `Launcher.exe`    |
| Develop with hot reload             | `RUN_PLC_LAB_DEV.bat` or `npm run dev`                                 |
| Run every check                     | `RUN_ALL_CHECKS.bat` or `npm run validate`                             |

`Launcher.exe` is a small native window that shows whether the installation is ready
(application, knowledge pack, WebView2, project folder), the most recent project and a
diagnostics report you can copy. It starts the staged release in `releases/current/` and
closes. It is created by the release build and is not stored in Git. It finds the release
next to itself, so the folder may be moved or copied anywhere, including paths with
spaces, as long as `Launcher.exe` and `releases/current/` stay together. `PLC Lab.exe`
and `RUN_PLC_LAB.bat` start the release directly. Only one PLC Lab window runs at a
time: starting it again, or opening a project, brings the open window to the front.
Requirements: Windows 10/11 x64 with the Microsoft Edge WebView2 Runtime (preinstalled on
current Windows). The executables are not code-signed; Windows Smart App Control or
SmartScreen may refuse to start a locally built copy. See
[build and run](docs/BUILD_AND_RUN.md).

## How to learn with it

1. **HOME** offers the next step: continue the course, practice, troubleshoot or open
   the Free Lab.
2. **LEARN** has 21 short lessons from "What is a PLC?" to troubleshooting methodology.
   Each lesson explains one idea, shows an example and a common mistake, asks a question
   and links an exercise.
3. **PRACTICE** exercises give you a machine and a task. Write the program, press
   **Build & Load to PLC**, operate the machine in the Process view and watch the
   Cause & effect panel. **Check solution** runs hidden test scenarios in an isolated
   simulation and grades behavior over time. The minimum path: pushbutton and lamp,
   motor jog, motor start/stop with seal-in, TON delayed start, button counter, tank
   filling; then analog scaling, part counting, a batch sequence and a Modbus level.
4. **COMMISSION** labs hand you a newly installed conveyor cell that does not start. The
   **Commissioning** tab of the workspace (available in every workspace) is a checklist
   from power to process that the simulation evaluates from what the installation shows
   and from the tests you perform: operate every button and sensor, switch every output
   and watch the checks turn into results. Labs: missing input field supply, guard switch
   with the wrong contact type, swapped Start/Stop wiring, swapped output wiring, missing
   contactor feedback, input module in the wrong slot. Analog labs on a buffer tank with
   a 2-wire 4-20 mA transmitter and a 0-10 V cross-check sensor: broken loop, missing
   loop supply, 0-10 V sensor on a current input, 0-20 versus 4-20 mA, zero offset. Drive
   labs on a conveyor with a generic VFD behavioral model (run, run enable, fault reset,
   preset speed; ready, running, remote and fault relays): local control, a latched trip,
   missing run enable, keypad speed reference, no mains supply.
5. **TROUBLESHOOT** labs start from a fault report. Reproduce the symptom, collect
   evidence (inputs, outputs, LEDs, measurements, diagnostics, ping, drive display),
   submit a diagnosis, repair and verify. Labs: Stop contact replaced by NO, inverted
   presence sensor, 4-20 mA channel set to 0-20 mA, missing output field supply, drive
   left in local control, Modbus register offset, transmitter in the wrong subnet. After
   a passed verification a debrief explains the root cause, the evidence that pointed to
   it, the correct repair and the reasoning.
6. **KNOWLEDGE** is the reference: fundamentals, instructions, eight PLC vendors and
   their families, nine industrial protocols and troubleshooting guides, all searchable.
7. **PROGRESS** shows completed lessons and best scores.

## Free Lab

**FREE LAB → New project** creates an engineering project on one of the machines
(I/O board, motor starter, conveyor with drive, tank, tank with Modbus transmitter) and a
PLC platform profile. The workspace has the program editor, hardware rack, process view,
network view, diagnostics, variables (scan input, committed value, next input), watch,
waveforms, events and cause & effect. In Advanced mode the hardware and network
configuration can be edited and downloaded. Projects can be saved, copied, exported and
imported as `.plclab` files.

## Where things are stored

| What                                 | Location                                                                                               |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| Projects (`.plclab`)                 | `%APPDATA%\dev.plclab.desktop\projects\`                                                               |
| Progress                             | `%APPDATA%\dev.plclab.desktop\progress.json`                                                           |
| Knowledge packs shipped with the app | `releases\current\knowledge-packs\`                                                                    |
| Additional knowledge packs you add   | `%APPDATA%\dev.plclab.desktop\knowledge-packs\` (one folder per pack)                                  |
| Knowledge pack archive               | [`releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip`](releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip) |

To install a pack, extract its ZIP into one of the knowledge-pack folders so that
`<folder>\<pack name>\pack.json` exists, then restart PLC Lab. The Knowledge section shows
each discovered pack with its status. See [knowledge pack](docs/KNOWLEDGE_PACK.md).

## Simulation fidelity

PLC Lab is an educational simulator. It does not emulate vendor firmware, does not
implement PROFINET, EtherCAT or EtherNet/IP stacks, does not connect to real devices and
claims no protocol or vendor conformance. The PLC runtime follows IEC 61131-3 semantics
for the implemented Structured Text subset; devices such as the motor starter, valve,
tank and drive are simplified, deterministic models. Platform profiles present addresses,
analog raw values and product names in a vendor's style and declare their fidelity
(exact, approximated, conceptual or not implemented). Knowledge pages mark facts that
could not be verified against a source. See [simulation fidelity](docs/SIMULATION_FIDELITY.md).

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

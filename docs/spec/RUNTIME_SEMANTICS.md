# Application execution semantics

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md). Kept for engineering history.

This is the M4B application-control companion to the accepted [PLC runtime semantics](PLC_RUNTIME_SEMANTICS.md), [TON semantics](TON_SEMANTICS.md) and [process model](PROCESS_SIMULATION.md). Their core behavior remains unchanged.

See [CONTINUOUS_EXECUTION.md](CONTINUOUS_EXECUTION.md) for normative mode transitions, checked lease authorization, quiescence and pacing. Manual Single Scan and continuous ticks both call one execute_one_cycle. Successful core commit precedes fixed-duration process evolution, next-scan sensor staging and coherent trace observation under one mutex. A fault invalidates Run before recording its rolled-back boundary. Host elapsed time never becomes PLC time. Stop/Pause preserve state; explicit resets retain their accepted, distinct meanings.

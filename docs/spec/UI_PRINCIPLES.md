# UI principles

These principles are mandatory for PLC Lab and follow [AGENTS.md](../../AGENTS.md), the [architecture](../architecture/ARCHITECTURE.md), [process/renderer separation](../adr/0005-process-renderer-separation.md), and [trace-first architecture](../adr/0006-trace-first.md). This specification adds no runtime functionality and does not require a redesign of the accepted M0 workspace shell.

## Results are a core product requirement

Every meaningful PLC programming exercise must produce a visible and understandable result. Results and runtime visualization are core product requirements, not optional UI enhancements.

When practical, a learner should be able to understand simultaneously:

- What PLC program is running.
- What important variables currently contain.
- What the simulated machine or process is doing.

Program, Variables, and Process views must support understanding these relationships together. Presentation should make the connection between code, state, and physical behavior understandable.

## Authoritative state drives visualization

Runtime visualization must always originate from authoritative Rust core/runtime state, including process and network state and source-aware trace evidence. The frontend renders that evidence; it must never independently calculate PLC logic to determine or display:

- Active LAD contacts.
- Active FBD paths.
- Executed ST or STL source.
- Motor state.
- Valve state.
- Tank level.
- Network communication state.

Program highlights must use authoritative execution/source mappings. Device and network views must use authoritative model state and events. UI rendering, animation timing, and frame rate must not determine PLC execution or simulation results. Display-only interpolation may smooth supplied state, but must not calculate process feedback or change authoritative values.

## Process visualization explains physical consequences

Process visualization represents the physical consequence of PLC logic and is not decorative. It must help learners understand how outputs affect actuators, how the process changes, and how resulting feedback influences subsequent execution. Visual movement or state changes must correspond to core-provided process state; they must not invent successful behavior.

## Presentation density can vary; the simulation engine cannot

The architecture must support different presentation density for beginner and advanced users without introducing different simulation engines. Beginner presentations may emphasize essential variables and process behavior; advanced presentations may expose more watch values, execution detail, timelines, and diagnostics.

Both presentations consume the same authoritative runtime, process/network state, and trace. Changing presentation density must not change PLC semantics, virtual timing, inputs, simulation state, or grading results. This is an architectural requirement for future presentation work, not an instruction to implement user modes in M0.

## Future diagnostics must explain causal chains

Future diagnostics should make causal chains understandable, for example:

```text
PLC output change
  → valve movement
  → process flow
  → tank level change
  → sensor transition
  → next PLC scan response
```

Explanations must originate from authoritative state, ordered events, source mappings, and trace evidence. The frontend must not reconstruct causality by re-running PLC logic or process/network models.

The causality system is a future capability. Do not implement causal inference, event correlation, explanatory runtime services, or fabricated causal traces during this M0 closeout. M1 is outside this task's scope.

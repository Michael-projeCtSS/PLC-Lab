# Grading model

Grading evaluates **behavior over time**, not final variable values, and never uses the
learner's live session.

## Isolated assessment

`lab_engine::assess` creates a fresh `LabSession` per scenario with the exercise scene,
the submitted station and program, the platform profile and (for troubleshooting) the
faults still present in the learner's installation. Scenario actions (press, release,
selector, presence, PLC mode, inject/clear fault) are scheduled at fixed world times. The
session runs to the scenario duration without pacing, wall clock or UI.

## Evidence

Each cycle becomes `CycleFacts` projected from the causal observation:

| Layer     | Source                                |
| --------- | ------------------------------------- |
| `inputs`  | the PLC scan input image              |
| `outputs` | values committed by the scan          |
| `process` | physical quantities after the cycle   |
| `next`    | inputs sampled for the following scan |

Evidence names map to PLC tags or component quantities. Process events (overflow, dry
run, high-high, low-low, motor overload) and PLC faults are recorded with world time.

## Rules

`always` (from a time), `never`, `eventually` (by a time), `respondsWithin` (each trigger
edge must be followed by the response within a time), `stateAt` (the cycle containing a
time), `noPlcFault`, `eventAbsent`. Conditions combine `compare` (signal, layer,
operator, literal) with `all`, `any`, `not`. Missing evidence fails a criterion with an
explanation instead of being guessed.

## Results

Each criterion reports points earned, pass/fail, the author's feedback on failure and a
concrete detail (for example the time at which a response was missing). Before grading,
the program must compile and declare the required interface with the right types and
directions; otherwise the outcome is `COMPILE_FAILED`, `INTERFACE_MISMATCH` or
`BINDING_FAILED` with the reasons. Troubleshooting scoring is described in the
[troubleshooting model](TROUBLESHOOTING_MODEL.md).

## Guarantees

- Deterministic: the same submission always gets the same report (fixed seed).
- Independent of UI and pacing.
- One simulator: assessment uses the same engine as the live session.
- Every shipped exercise is proven solvable and not solved by its starter
  (`tests/conformance/tests/knowledge_pack.rs`).

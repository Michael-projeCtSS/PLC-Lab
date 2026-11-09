# ADR-0044: User functions and function blocks by compile-time expansion

- Status: Accepted for 0.2.0-dev (CP9-A)
- Date: 2026-11-08

## Context

Students could only write one `PROGRAM`. Real PLC code is organized in reusable POUs:
functions that compute a value and function blocks whose instances keep their own state.
The canonical IR (ADR-0002) has one entry body of assignments, IF statements and standard
function block calls (ADR-0011); the runtime executes exactly that, transactionally and
within budgets. A second interpreter for user POUs is not acceptable.

## Decision

### Source

One ST document may contain any number of `FUNCTION` and `FUNCTION_BLOCK` definitions and
exactly one `PROGRAM` (the entry), in any order:

```
FUNCTION Scale : REAL
VAR_INPUT Raw : INT; Low : REAL; High : REAL; END_VAR
Scale := Low + (INT_TO_REAL(Raw) / 27648.0) * (High - Low);
END_FUNCTION

FUNCTION_BLOCK MotorControl
VAR_INPUT Start : BOOL; Stop : BOOL; END_VAR
VAR_OUTPUT Running : BOOL; END_VAR
VAR Delay : TON; END_VAR
Running := (Start OR Running) AND NOT Stop;
END_FUNCTION_BLOCK
```

- A function has `VAR_INPUT` and local `VAR`; its return value is the variable named like
  the function. It has no memory between calls, declares no function block instance and
  cannot write anything but its own variables.
- A function block has `VAR_INPUT`, `VAR_OUTPUT`, `VAR_IN_OUT` and `VAR` (scalars,
  standard and user function block instances).
- Calls: `F(a, b)` with positional arguments in declaration order inside expressions;
  `Inst(In := expr, InOut := variable, Out => variable)` as a statement. Outputs of an
  instance are read as `Inst.Out`. Unbound inputs keep their previous value (instance
  memory); every `VAR_IN_OUT` must be bound to a variable of exactly its type.
- Recursion (direct or indirect), unknown POUs, wrong argument counts or types, binding
  an output with `:=` or an input with `=>`, reading an instance input or local, a
  function declaring an instance, and writing a function input are compile errors with
  source locations.

### Lowering: expansion into the canonical IR

The compiler expands user POUs; the IR and runtime are unchanged in kind.

- **Instance memory.** Each declared user FB instance owns one IR symbol per variable of
  its definition, named by its path (`Pump1.Running`, nested `Pump1.Delay.Q` through the
  standard instance `Pump1.Delay`). Generated qualified names are dot-separated
  identifiers; source code cannot declare them (the lexer splits at dots) and hardware
  tags, which are plain identifiers, never bind to them. Instance variables are
  non-retained in this checkpoint.
- **FB call.** A call statement expands, in order, to: assignments of the bound inputs to
  the instance's input symbols; the definition's body compiled against the instance
  (its names resolve to the instance's symbols, `VAR_IN_OUT` names to the bound actual
  variable itself, so writes reach the caller's variable); assignments of the bound
  outputs from the instance's output symbols.
- **Function call.** Each call site gets its own temporary symbols (`Scale#3.Raw`,
  `Scale#3.Scale`). Before the statement that contains the call, the expansion assigns
  every local and the return variable its initial value, the arguments to the inputs, and
  then the function body; the expression reads the return temporary. Functions are pure,
  and the ST subset has no loops, so evaluating them before the statement is equivalent
  (inside a branch the hoisted code stays inside that branch).
- **Source mapping.** Expanded statements keep the source references of the POU's own
  lines, so runtime faults and diagnostics point into the function or function block.
- **Checking.** Every POU body is also compiled once in its own scope, so errors are
  reported once, at their definition, even for POUs nobody calls. Expansion is bounded:
  more than 64 nested calls or a program larger than the configured statement budget is
  a compile error, never a runtime surprise.

### Visibility

Compile results carry the POUs of the document (kind, name, return type, parameters with
direction, type and the comment after the declaration, the leading comment as the
description). The library browser lists them beside the standard blocks (TON, TOF, TP,
R_TRIG, F_TRIG, CTU, CTD) with signatures, descriptions and usage examples. Online,
instance variables appear under their paths (`Toggle1.On`), and the arguments and result
of the latest call of a function under its call path (`Both#1.A`).

Exercises may require user POUs (`requiredPous`: name and `function` or `functionBlock`).
They are checked before grading, like the required interface, so a program that behaves
correctly without the requested block is told what to write instead of being graded.

## Alternatives considered

A call stack and frames in the runtime (rejected for now: a second execution model beside
the transactional scan, more budget and fault cases, no gain for an educational subset
without recursion or loops); a separate interpreter for user POUs (rejected: one
runtime); text snippets as a "library" (rejected: not compiled, not checked, not
reusable).

## Consequences

User POUs run in the same runtime with the same budgets, transactions, traces and
monitoring. Program size grows with call sites, which the budget bounds. Retentive
instances, several source documents and libraries shared between projects build on this
and are separate increments.

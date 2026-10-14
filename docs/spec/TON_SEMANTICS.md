# PLC Lab Alpha TON semantics

Status: M1C authoritative assignment implemented for review. The typed signature is defined in [STANDARD_FUNCTION_BLOCKS.md](STANDARD_FUNCTION_BLOCKS.md). This is the specified educational Alpha behavior; it does not claim every vendor's timer policy.

## Private state and scan-start time

Each declared TON instance has private active_since (optional PlcTime), q (bool), and et (PlcTime). Its default is inactive, Q false, ET zero. Only Q/ET are publicly observable; activation time is not a PLC variable or public snapshot field.

Every executed call receives the same SimulationClock value captured at the start of its scan. It does not add the cycle period first. No wall-clock time, sleep, thread, UI frame, or real-time timer participates.

## State transition

- IN false: clear active_since, set Q false and ET zero.
- IN true while inactive: set active_since to now and elapsed to zero.
- IN true while active: elapsed is checked now minus active_since.
- For either true case: ET = min(elapsed, current PT); Q = elapsed >= current PT.

PT zero therefore asserts Q immediately and leaves ET zero. Checked subtraction is private monotonic-clock bookkeeping, not an approved PLC TIME arithmetic operator. A clock earlier than active_since is an internal invariant error (APP-CORE-001), not a student RuntimeFault.

PT changes do not restart the epoch while IN stays true. A shorter PT can assert Q immediately; a longer PT can deassert it again when elapsed is below the new preset. ET is capped against the current preset, not its previous value.

## Required timeline

With 100 ms cycles, PT 300 ms, and IN true before the first scan:

| Completed scan | Call's scan-start time | ET after call | Q after call | Time after successful completion |
| -------------- | ---------------------- | ------------- | ------------ | -------------------------------- |
| 1              | 0 ms                   | 0 ms          | false        | 100 ms                           |
| 2              | 100 ms                 | 100 ms        | false        | 200 ms                           |
| 3              | 200 ms                 | 200 ms        | false        | 300 ms                           |
| 4              | 300 ms                 | 300 ms        | true         | 400 ms                           |

Later true calls retain Q true and cap ET at 300 ms. IN false resets both; a later true call starts a fresh epoch.

## Skipped and repeated calls

A call in an unselected IF body costs no operations, writes no association, and changes no instance state. Other successful scans still advance virtual time. The next true call computes elapsed using current scan-start time and the retained epoch, so ET can jump over skipped cycles.

Multiple calls to one instance in one scan are allowed. They run sequentially at the same now and see earlier working instance updates. A false call resets the epoch; a later true call starts it again at that same now. No hidden one-call-per-scan restriction exists. Different instance IDs never share state, even when they use the same definition.

## Outputs and observation

Every executed call updates working Q/ET, including calls without output associations. Bound Q is copied to its writable BOOL symbol, then bound ET to its writable TIME symbol. Those writes are visible to subsequent instructions in the same scan. Omitted associations write no unrelated symbols.

RuntimeSnapshot contains an immutable map from FunctionBlockInstanceId to FunctionBlockStateSnapshot::Ton { q, et }. It observes committed instance outputs only and exposes no mutation, activation timestamp, member-access syntax, or frontend IPC.

## Transaction and reset

begin_scan clones both committed PLC writable memory and committed FB state. Calls modify only those working copies. Successful completion publishes both after all execution and clock/count checks succeed. Arithmetic, budget, internal, or clock/count completion failures discard both, preserving committed epoch/Q/ET, PLC values, outputs, time, and completed count. The accepted last-attempted input image is still not rolled back.

Explicit reset restores every declared executable TON instance to inactive/Q false/ET zero alongside the accepted M1B memory/time/count reset. No RETAIN policy or implicit fault recovery exists.

## Exact budget charging

1. Evaluate IN expression, charging its existing expression operations.
2. Evaluate PT expression, charging its existing expression operations.
3. Charge one operation before TON instance update.
4. If Q is bound, charge one operation before its write.
5. If ET is bound, charge one operation before its write.

For constant IN/PT: no associations costs 3 operations; Q only or ET only costs 4; both costs 5. Producer argument order cannot change this because lowering normalizes to signature order. A budget exhausted between state update and either association aborts the whole transaction, including earlier TON/PLC writes.

## Scope and evidence

Tests must cover the complete requested M1C matrix, including private epoch rollback, source-backed binding diagnostics, signature-order fault/budget behavior, independent instances, exact timeline, skipped/repeated calls, dynamic PT, omissions, read-after-association, and reset. Run debug and optimized runtime suites, then the canonical repository gate.

No ST/LAD/FBD/STL frontend, other timers/counters, user-defined FB/function execution, process/network model, trace recorder, grading, or UI execution is included.

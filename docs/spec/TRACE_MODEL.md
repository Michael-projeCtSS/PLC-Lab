# Deterministic manual session trace

## M4B extension

The original eight events retain their semantics. ExecutionStarted/ExecutionResumed record RUNNING; ExecutionPaused records PAUSED; ExecutionStopped records STOPPED. Each control row has its own sequence and unchanged PLC count/time, with a post-transition observation. Duplicate controls append nothing. ScanFaulted records STOPPED after lease invalidation; automatic faults need no extra ExecutionStopped. Internal failures fabricate no cycle event. Every live/historical observation now includes executionMode and read-only executionRunId in strict IPC v7. Wake signals trigger queries only; trace pages remain authoritative. Pinned history is independent from live refresh and explicitly reports selection eviction. Retention and saturation never control Run. See [continuous execution](CONTINUOUS_EXECUTION.md); the original M4A material below remains the accepted recorder specification.

Trace observes authoritative Runtime/Process state. It does not execute PLC logic, advance SimulationClock, integrate a process or derive sensors. M4A records session/cycle boundaries only, before any continuous execution.

## Identity, point and retention

TraceSequenceId is a nonzero checked u64 starting at 1 per committed RuntimeSessionId. It is independent of session generation, completed scan count and SimulationClock. Inputs/process actions share unchanged scan/time; reset may decrease scan/time while sequence continues increasing. Canonical time is exact PlcTime from the runtime, never wall time.

Default capacity: 4096 typed entries. Append evicts the oldest first and increments dropped count; retained IDs never change. The highest u64 ID can be recorded once; recorder then reports SATURATED and accepts no further records. Execution continues unchanged. No saturation is a student RuntimeFault. No persistence/export exists.

## Typed observation and recording

Internal application observations retain ordered variables (SymbolId, original name/type/role, ownership and PlcValue), visible TON instance/name/Q/ET, runtime state/Diagnostic including source provenance, and optional TankSnapshot. Private TON epoch, private process integration state, IR and plans never cross the boundary. Trace payloads are typed Rust structures, not DTOs/JSON. The same mapping presents live and historical observations.

| Event           | Exact append condition and observation                                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| SessionLoaded   | After successful latest-intent commit, initial new session, sequence 1                                                       |
| InputStaged     | After accepted user staging, post-stage input, unchanged scan/time; staged variable identity retained                        |
| ProcessAttached | After complete validated attachment and initial sensor staging                                                               |
| ScanCompleted   | After successful full PLC -> process -> next sensors cycle, one event                                                        |
| ScanFaulted     | A Ready scan produces a genuine student RuntimeFault; rolled-back count/time/PLC/process observation and diagnostic retained |
| RuntimeReset    | After existing Runtime::reset and preserved-plant sensor restage                                                             |
| ProcessReset    | After physical reset, PLC memory/count/time/fault unchanged                                                                  |
| ProcessDetached | After dropping plant/ownership; observation has no process and sensor inputs become USER                                     |

Mutation, coherent observation and append share the same session mutex. Valid append/eviction/saturation never fails execution. Failed load/attach/input/scene commands, stale IDs/intents, compiler operations, already-Faulted rescans, snapshot reads and trace queries append nothing. Internal execution errors retain their category in live observation but are not labeled ScanFaulted. A process/session failure before a coherent observation appends no entry and retains the accepted closed-session behavior.

Successful replacement discards old trace and starts sequence 1. Failed/stale Load preserves current history. Unload discards runtime/history without resetting session/intent counters.

## Query and IPC v6

get_trace_page requires contractVersion, RuntimeSessionId, optional afterSequence and integer limit 1..256. Query is read-only and returns ascending contiguous retained entries plus capacity, oldestSequence/latestSequence, droppedCount, saturated and gap. Every u64-like field uses canonical decimal strings; IDs are nonzero. Unknown fields/tags, malformed values, ownership, ordering and diagnostic categories are strictly validated.

null cursor starts at oldest retained. N returns sequence>N. gap=true exactly when N<oldest-1: requested entries after N were lost. N=oldest-1 loses none of its requested entries. null gap is false, while droppedCount still exposes history loss. N=latest returns empty/gap=false. N>latest (including a cursor into an empty history) is APP-TRACE-CURSOR. Bad limits are APP-TRACE-LIMIT. Stale session uses APP-RUNTIME-STALE-SESSION. These are application query errors, not PLC faults.

## Timeline

One request-driven catch-up follows each accepted Load/Input/Attach/Scan/Reset/Detach observation. Pages contain at most 256 entries; at most 16 pages cover the retained 4096-entry history. Merge uses sequence identity, drops frontend entries outside backend retention and keeps exact strings/BigInt-safe formatting. Replacement/unload/source changes invalidate pending responses. Old-source history may remain visibly STALE; it cannot navigate edited source.

Timeline lists sequence, event, scan, simulation time and summary. Selecting an entry displays Historical Trace with its recorded variables, visible TON state, diagnostic and process. It never substitutes live ProcessPanel state or mutates the runtime. Tank scan 8 records 800 permille/High TRUE/Inlet TRUE at 800 ms; scan 9 records the same level/High with Inlet FALSE at 900 ms. Dropped history and internal SATURATED status are visible. No replay, rewind, restore, grading, network, instruction stepping, streaming, scheduler or trace file exists.

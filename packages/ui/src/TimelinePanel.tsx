import { useState } from "react";
import type {
  CompilerDiagnostic,
  TraceEntryDto,
  TracePageDto,
} from "@plc-lab/ipc";
import { formatDuration, formatRuntimeValue } from "./runtime-values";

function eventName(entry: TraceEntryDto) {
  return entry.kind.replace(/([a-z])([A-Z])/g, "$1 $2").toUpperCase();
}
function summary(entry: TraceEntryDto) {
  if (entry.fault)
    return `${entry.fault.category === "runtimeFault" ? "PLC FAULT" : "APPLICATION ERROR"} · ${entry.fault.code}`;
  if (entry.stagedInput) return `Input ${entry.stagedInput.name}`;
  if (entry.process)
    return `Tank ${(entry.process.levelPermille / 10).toFixed(1)} %`;
  return entry.runtimeState;
}
export function TimelinePanel({
  page,
  current,
  pending,
  error,
  onDiagnostic,
}: {
  page: TracePageDto | null;
  current: boolean;
  pending: boolean;
  error: string | null;
  onDiagnostic: (diagnostic: CompilerDiagnostic) => void;
}) {
  const [selected, setSelected] = useState<string | null>(null);
  const event = page?.entries.find((e) => e.sequence === selected) ?? null;
  return (
    <div
      className="session-timeline"
      data-testid="session-timeline"
      data-trace-pending={pending}
      data-trace-session={page?.sessionId}
    >
      {!current && page && (
        <p className="trace-status">Previous loaded source · STALE</p>
      )}
      {pending && <p role="status">Reading trace…</p>}
      {error && <p role="alert">TRACE QUERY · {error}</p>}
      {page && (page.droppedCount !== "0" || page.gap) && (
        <p className="trace-status">
          Earlier trace entries were discarded. Dropped: {page.droppedCount}
        </p>
      )}
      {page?.saturated && (
        <p role="alert">
          INTERNAL TRACE STATUS · SATURATED. Recording stopped; PLC execution
          remains available.
        </p>
      )}
      {!page?.entries.length && (
        <div className="trace-status">
          <h3>No trace recorded</h3>
          <p>Load a program to begin session history.</p>
        </div>
      )}
      {!!page?.entries.length && (
        <>
          <div className="trace-columns" aria-hidden="true">
            <span>Sequence</span>
            <span>Event</span>
            <span>PLC scan / time</span>
            <span>Summary</span>
          </div>
          <div className="trace-list" role="list" aria-label="Trace events">
            {page.entries.map((e) => (
              <div role="listitem" key={e.sequence}>
                <button
                  type="button"
                  className={`trace-entry ${e.fault ? "trace-fault" : ""}`}
                  data-trace-sequence={e.sequence}
                  data-trace-kind={e.kind}
                  data-trace-scan={e.completedScans}
                  aria-pressed={e.sequence === selected}
                  aria-label={`#${e.sequence} ${eventName(e)} Scan ${e.completedScans} ${formatDuration(e.simulationTimeMicros)} ${summary(e)}`}
                  onClick={() => setSelected(e.sequence)}
                >
                  <span>#{e.sequence}</span>
                  <span>{eventName(e)}</span>
                  <span>
                    Scan {e.completedScans}
                    <br />
                    {formatDuration(e.simulationTimeMicros)}
                  </span>
                  <span>{summary(e)}</span>
                </button>
              </div>
            ))}
          </div>
          {event ? (
            <div
              className="trace-inspector"
              data-testid="trace-inspector"
              data-selected-sequence={event.sequence}
            >
              <h3>Historical Trace · #{event.sequence}</h3>
              <p>
                {eventName(event)} · {event.runtimeState} ·{" "}
                {event.executionMode} · Scan {event.completedScans} ·{" "}
                {formatDuration(event.simulationTimeMicros)}
              </p>
              <p className="trace-status">
                Recorded observation only · live state is unchanged
              </p>
              {event.process && (
                <dl className="trace-tank" data-testid="historical-tank">
                  <dt>Tank Level</dt>
                  <dd>{(event.process.levelPermille / 10).toFixed(1)} %</dd>
                  <dt>Low</dt>
                  <dd>{event.process.lowLevel ? "ACTIVE" : "INACTIVE"}</dd>
                  <dt>High</dt>
                  <dd>{event.process.highLevel ? "ACTIVE" : "INACTIVE"}</dd>
                  <dt>Inlet Valve</dt>
                  <dd>{event.process.inletValveOpen ? "OPEN" : "CLOSED"}</dd>
                  <dt>Drain Pump</dt>
                  <dd>
                    {event.process.drainPumpRunning ? "RUNNING" : "STOPPED"}
                  </dd>
                </dl>
              )}
              <h4>Recorded PLC variables</h4>
              <ul>
                {event.variables.map((v) => (
                  <li key={v.variableId}>
                    {v.name} · {v.typeName} · {v.role} {v.inputAuthority ?? ""}{" "}
                    · {formatRuntimeValue(v.value)}
                  </li>
                ))}
              </ul>
              {event.functionBlocks.map((b) => (
                <p key={b.instanceId}>
                  {b.name} · TON · Q {b.q ? "TRUE" : "FALSE"} · ET{" "}
                  {formatDuration(b.etMicros)}
                </p>
              ))}
              {event.fault && (
                <div className="historical-fault">
                  <p>
                    {event.fault.category === "runtimeFault"
                      ? "PLC FAULT"
                      : "APPLICATION ERROR"}{" "}
                    · {event.fault.code}: {event.fault.message}
                  </p>
                  <button
                    type="button"
                    disabled={!current || !event.fault.span}
                    onClick={() => {
                      if (event.fault) onDiagnostic({ ...event.fault });
                    }}
                  >
                    Focus historical source expression
                  </button>
                </div>
              )}
            </div>
          ) : (
            <p className="trace-status">
              {selected &&
              page.oldestSequence &&
              BigInt(selected) < BigInt(page.oldestSequence)
                ? "Selected event is no longer retained. Select another event."
                : "Select an event to inspect Historical Trace."}
            </p>
          )}
        </>
      )}
    </div>
  );
}

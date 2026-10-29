/**
 * The CPU diagnostic buffer, newest first: time, severity, category, source, message,
 * structured details and whether a condition is still active or when it cleared. The
 * backend classifies, pairs and filters the entries; this view only chooses filters and
 * pages.
 */
import { useState } from "react";
import { api, type DiagnosticBufferDto } from "@plc-lab/ipc";
import { seconds } from "../format";
import { Badge, ErrorNotice } from "../ui";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

const PAGE = 50;
const SEVERITY_KIND: Record<string, string> = {
  error: "fault",
  warning: "warning",
  info: "normal",
};

export function DiagnosticBuffer({ lab }: { lab: Lab }) {
  const ws = lab.ws;
  const [severity, setSeverity] = useState<string | null>(null);
  const [category, setCategory] = useState<string | null>(null);
  const [moduleId, setModuleId] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const buffer = useFrameQuery<DiagnosticBufferDto>(
    lab,
    () =>
      ws
        ? api.diagnosticBuffer(ws, {
            offset,
            limit: PAGE,
            severity,
            category,
            moduleId,
          })
        : Promise.reject(new Error("No workspace")),
    [offset, severity, category, moduleId],
  );
  const modules = lab.project?.station.modules ?? [];
  const select = (set: (v: string | null) => void) => (value: string) => {
    set(value === "" ? null : value);
    setOffset(0);
  };
  const data = buffer.data;
  return (
    <div className="panel">
      <div className="panel-head row wrap" style={{ gap: 8 }}>
        <h4>CPU diagnostic buffer</h4>
        <span className="muted">
          newest first
          {data
            ? ` · ${data.matching} of ${Math.min(Number(data.recorded), data.capacity)} entries (capacity ${data.capacity}, ${data.recorded} recorded)`
            : ""}
        </span>
        <span className="spacer" />
        <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
          Severity
          <select
            className="input"
            value={severity ?? ""}
            onChange={(e) => select(setSeverity)(e.target.value)}
          >
            <option value="">all</option>
            <option value="error">error</option>
            <option value="warning">warning</option>
            <option value="info">info</option>
          </select>
        </label>
        <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
          Category
          <select
            className="input"
            value={category ?? ""}
            onChange={(e) => select(setCategory)(e.target.value)}
          >
            <option value="">all</option>
            <option value="CPU">CPU</option>
            <option value="HARDWARE">hardware</option>
            <option value="NETWORK">network</option>
          </select>
        </label>
        <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
          Module
          <select
            className="input"
            value={moduleId ?? ""}
            onChange={(e) => select(setModuleId)(e.target.value)}
          >
            <option value="">all</option>
            {modules.map((m) => (
              <option key={m.id} value={m.id}>
                {m.id}
              </option>
            ))}
          </select>
        </label>
      </div>
      <ErrorNotice error={buffer.error} />
      <table className="table" aria-label="Diagnostic buffer">
        <thead>
          <tr>
            <th>Time</th>
            <th>Severity</th>
            <th>Category</th>
            <th>Source</th>
            <th>Message</th>
            <th>State</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {data?.entries.length === 0 && (
            <tr>
              <td colSpan={7} className="muted">
                No entry matches.
              </td>
            </tr>
          )}
          {data?.entries.map((d, i) => (
            <tr key={`${d.timeUs}-${offset + i}`}>
              <td className="mono">{seconds(d.timeUs)}</td>
              <td>
                <Badge kind={SEVERITY_KIND[d.severity] ?? "normal"}>
                  {d.severity}
                </Badge>
              </td>
              <td>{d.category.toLowerCase()}</td>
              <td>{d.source}</td>
              <td style={{ whiteSpace: "normal" }}>{d.text}</td>
              <td>
                {d.event === "EVENT"
                  ? "event"
                  : d.event === "OUTGOING"
                    ? "cleared"
                    : d.active
                      ? "active"
                      : d.clearedUs
                        ? `cleared after ${seconds(String(BigInt(d.clearedUs) - BigInt(d.timeUs)))}`
                        : "no longer in the buffer"}
              </td>
              <td className="dim">
                {d.details.map((x) => `${x.name}: ${x.value}`).join(", ")}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row" style={{ padding: 8, gap: 8 }}>
        <button
          className="btn btn-sm"
          disabled={offset === 0}
          onClick={() => setOffset(Math.max(0, offset - PAGE))}
        >
          Newer
        </button>
        <button
          className="btn btn-sm"
          disabled={!data || offset + PAGE >= data.matching}
          onClick={() => setOffset(offset + PAGE)}
        >
          Older
        </button>
      </div>
    </div>
  );
}

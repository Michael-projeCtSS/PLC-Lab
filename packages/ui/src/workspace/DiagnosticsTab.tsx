/** Online diagnostics: CPU state, active alarms and the CPU diagnostic buffer. */
import { api, type HardwareDto } from "@plc-lab/ipc";
import { seconds } from "../format";
import { Badge, ErrorNotice, PlcModeBadge } from "../ui";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

export function DiagnosticsTab({ lab }: { lab: Lab }) {
  const { ws, frame, workspace } = lab;
  const hw = useFrameQuery<HardwareDto>(lab, () =>
    ws ? api.hardware(ws) : Promise.reject(new Error("No workspace")),
  );
  useEvidence(lab, "diagnostic buffer");
  if (!frame || !workspace) return null;
  const byId = new Map(workspace.scene.components.map((c) => [c.id, c.label]));
  return (
    <div
      style={{ position: "absolute", inset: 0, overflow: "auto", padding: 16 }}
      className="col"
    >
      <ErrorNotice error={hw.error} />
      <div className="row wrap" style={{ gap: 16 }}>
        <div className="panel" style={{ minWidth: 300 }}>
          <div className="panel-head">
            <h4>CPU</h4>
            <span className="spacer" />
            <PlcModeBadge mode={frame.plc.mode} />
          </div>
          <dl className="kv panel-body">
            <dt>Mode switch</dt>
            <dd>{frame.plc.switch}</dd>
            <dt>Program</dt>
            <dd>{frame.plc.programName ?? "none loaded"}</dd>
            <dt>Completed scans</dt>
            <dd>{frame.plc.scans}</dd>
            <dt>Start segment</dt>
            <dd>{frame.plc.segment}</dd>
            <dt>Forced values</dt>
            <dd>{frame.plc.forces}</dd>
          </dl>
          {frame.plc.fault && (
            <div className="notice error" style={{ margin: 12 }}>
              <strong>{frame.plc.fault.code}</strong>: {frame.plc.fault.message}
              {frame.plc.fault.span &&
                ` (program line ${frame.plc.fault.span.start.line})`}
              <div className="muted" style={{ marginTop: 4 }}>
                Correct the program or the cause, then use Reset PLC before
                switching to RUN.
              </div>
            </div>
          )}
        </div>
        <div className="panel grow">
          <div className="panel-head">
            <h4>Active process alarms</h4>
          </div>
          <div className="panel-body col">
            {frame.alarms.length === 0 && (
              <span className="muted">No active process alarm.</span>
            )}
            {frame.alarms.map((a) => (
              <div key={a.component + a.kind} className="row">
                <Badge
                  kind={
                    a.kind === "OVERFLOW" ||
                    a.kind === "DRY_RUN" ||
                    a.kind === "OVERLOAD"
                      ? "fault"
                      : "warning"
                  }
                >
                  {a.kind.replaceAll("_", " ")}
                </Badge>
                <span>
                  {byId.get(a.component) ?? a.component}: {a.text}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
      <div className="panel">
        <div className="panel-head">
          <h4>CPU diagnostic buffer</h4>
          <span className="muted">
            newest last · {frame.activeDiagnostics} active
          </span>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Event</th>
              <th>Source</th>
              <th>Description</th>
            </tr>
          </thead>
          <tbody>
            {hw.data?.diagnostics.length === 0 && (
              <tr>
                <td colSpan={4} className="muted">
                  The diagnostic buffer is empty.
                </td>
              </tr>
            )}
            {hw.data?.diagnostics.map((d, i) => (
              <tr key={i}>
                <td className="mono">{seconds(d.timeUs)}</td>
                <td>
                  <Badge kind={d.event === "INCOMING" ? "warning" : "normal"}>
                    {d.event === "INCOMING" ? "incoming" : "outgoing"}
                  </Badge>
                </td>
                <td>{d.source}</td>
                <td style={{ whiteSpace: "normal" }}>{d.text}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/** Online diagnostics: CPU state, active process alarms and the CPU diagnostic buffer. */
import { PlcModeBadge } from "../ui";
import { useEvidence } from "./useFrameQuery";
import { useInterface } from "../locale";
import { AlarmSummary } from "./AlarmsTab";
import { DiagnosticBuffer } from "./DiagnosticBuffer";
import type { Lab } from "./useLab";

export function DiagnosticsTab({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const { frame, workspace } = lab;
  useEvidence(lab, "diagnostic buffer");
  if (!frame || !workspace) return null;
  return (
    <div
      style={{ position: "absolute", inset: 0, overflow: "auto", padding: 16 }}
      className="col"
    >
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
            <h4>{t("alarm.active")}</h4>
          </div>
          <div className="panel-body row wrap">
            {frame.alarms.length === 0 && (
              <span className="muted">{t("alarm.none")}</span>
            )}
            <AlarmSummary alarms={frame.alarms} />
          </div>
        </div>
      </div>
      <DiagnosticBuffer lab={lab} />
    </div>
  );
}

/** Online diagnostics: CPU state, active process alarms and the CPU diagnostic buffer. */
import { PlcModeBadge } from "../ui";
import { useEvidence } from "./useFrameQuery";
import { useInterface } from "../locale";
import { diagnosticTitle } from "../locale/messages";
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
            <dt>{t("cpu.switch")}</dt>
            <dd>{frame.plc.switch}</dd>
            <dt>{t("cpu.program")}</dt>
            <dd>{frame.plc.programName ?? t("cpu.noProgram")}</dd>
            <dt>{t("cpu.scans")}</dt>
            <dd>{frame.plc.scans}</dd>
            <dt>{t("cpu.segment")}</dt>
            <dd>{frame.plc.segment}</dd>
            <dt>{t("cpu.forces")}</dt>
            <dd>{frame.plc.forces}</dd>
          </dl>
          {frame.plc.fault && (
            <div className="notice error" style={{ margin: 12 }}>
              <strong>{frame.plc.fault.code}</strong>
              {diagnosticTitle(t, frame.plc.fault.code)}:{" "}
              {frame.plc.fault.message}
              {frame.plc.fault.span &&
                t("cpu.line", { line: frame.plc.fault.span.start.line })}
              <div className="muted" style={{ marginTop: 4 }}>
                {t("cpu.faultHelp")}
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

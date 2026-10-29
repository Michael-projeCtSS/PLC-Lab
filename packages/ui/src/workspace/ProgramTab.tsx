import { useMemo, useState } from "react";
import { useInterface } from "../locale";
import type { DiagnosticDto } from "@plc-lab/ipc";
import { StructuredTextEditor } from "../StructuredTextEditor";
import type { Lab } from "./useLab";

export function ProgramTab({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const [focused, setFocused] = useState<DiagnosticDto | null>(null);
  // Stable marker list: only a different fault (by content) updates editor markers.
  const faultKey = lab.frame?.plc.fault
    ? JSON.stringify(lab.frame.plc.fault)
    : "";
  const runtime = useMemo(
    () => (faultKey ? [JSON.parse(faultKey) as DiagnosticDto] : []),
    [faultKey],
  );
  return (
    <div className="program-panel">
      {lab.diagnostics.length > 0 && (
        <div className="compile-problems" role="alert">
          <strong>{t("control.failed")}</strong>
          {lab.diagnostics.map((d, i) => (
            <button
              key={i}
              className="btn btn-ghost btn-sm"
              onClick={() => setFocused(d)}
            >
              {d.span ? `${d.span.start.line}: ` : ""}
              {d.code} · {d.message}
            </button>
          ))}
        </div>
      )}
      <StructuredTextEditor
        source={lab.source}
        onChange={lab.setSource}
        diagnostics={lab.diagnostics}
        runtimeDiagnostics={runtime}
        focusedDiagnostic={focused}
      />
    </div>
  );
}

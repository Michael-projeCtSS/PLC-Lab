import { useMemo } from "react";
import type { DiagnosticDto } from "@plc-lab/ipc";
import { StructuredTextEditor } from "../StructuredTextEditor";
import type { Lab } from "./useLab";

export function ProgramTab({ lab }: { lab: Lab }) {
  // Stable marker list: only a different fault (by content) updates editor markers.
  const faultKey = lab.frame?.plc.fault
    ? JSON.stringify(lab.frame.plc.fault)
    : "";
  const runtime = useMemo(
    () => (faultKey ? [JSON.parse(faultKey) as DiagnosticDto] : []),
    [faultKey],
  );
  return (
    <StructuredTextEditor
      source={lab.source}
      onChange={lab.setSource}
      diagnostics={lab.diagnostics}
      runtimeDiagnostics={runtime}
      focusedDiagnostic={null}
    />
  );
}

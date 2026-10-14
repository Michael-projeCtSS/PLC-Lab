import { useEffect, useRef, useState } from "react";
import type { CompilerDiagnostic } from "@plc-lab/ipc";
import type { createStEditor } from "./monaco-host";

export function StructuredTextEditor({
  source,
  onChange,
  diagnostics,
  focusedDiagnostic,
}: {
  source: string;
  onChange: (value: string) => void;
  diagnostics: CompilerDiagnostic[];
  focusedDiagnostic: CompilerDiagnostic | null;
}) {
  const container = useRef<HTMLDivElement>(null),
    instance = useRef<ReturnType<typeof createStEditor> | null>(null);
  const latest = useRef({ source, onChange, diagnostics, focusedDiagnostic });
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    latest.current = { source, onChange, diagnostics, focusedDiagnostic };
    instance.current?.update(source, diagnostics);
    if (focusedDiagnostic) instance.current?.focus(focusedDiagnostic.span);
  }, [source, onChange, diagnostics, focusedDiagnostic]);
  useEffect(() => {
    let active = true;
    import("./monaco-host")
      .then((module) => {
        if (!active || !container.current) return;
        const current = latest.current;
        instance.current = module.createStEditor(
          container.current,
          current.source,
          (value) => latest.current.onChange(value),
        );
        instance.current.update(current.source, current.diagnostics);
      })
      .catch((error: unknown) => {
        if (active)
          setError(
            error instanceof Error
              ? error.message
              : "The editor could not load.",
          );
      });
    return () => {
      active = false;
      instance.current?.dispose();
      instance.current = null;
    };
  }, []);
  return (
    <div
      className="st-editor"
      data-testid="st-editor"
      aria-label="Structured Text editor"
    >
      <div className="monaco-surface" ref={container} />
      {error && <p role="alert">Editor unavailable: {error}</p>}
    </div>
  );
}

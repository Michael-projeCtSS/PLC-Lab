import { useEffect, useRef, useState } from "react";
import type { CompilerDiagnostic } from "@plc-lab/ipc";
import type { createStEditor } from "./monaco-host";

export function StructuredTextEditor({
  source,
  readOnly = false,
  onChange,
  diagnostics,
  runtimeDiagnostics = [],
  focusedDiagnostic,
}: {
  source: string;
  readOnly?: boolean;
  onChange: (value: string) => void;
  diagnostics: CompilerDiagnostic[];
  runtimeDiagnostics?: CompilerDiagnostic[];
  focusedDiagnostic: CompilerDiagnostic | null;
}) {
  const container = useRef<HTMLDivElement>(null),
    instance = useRef<ReturnType<typeof createStEditor> | null>(null);
  const latest = useRef({
    source,
    readOnly,
    onChange,
    diagnostics,
    runtimeDiagnostics,
    focusedDiagnostic,
  });
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    latest.current = {
      source,
      readOnly,
      onChange,
      diagnostics,
      runtimeDiagnostics,
      focusedDiagnostic,
    };
    instance.current?.update(source, diagnostics, runtimeDiagnostics, readOnly);
    if (focusedDiagnostic) instance.current?.focus(focusedDiagnostic.span);
  }, [
    source,
    readOnly,
    onChange,
    diagnostics,
    runtimeDiagnostics,
    focusedDiagnostic,
  ]);
  useEffect(() => {
    let active = true;
    import("./monaco-host")
      .then((module) => {
        if (!active || !container.current) return;
        const current = latest.current;
        instance.current = module.createStEditor(
          container.current,
          current.source,
          (value) => {
            if (!latest.current.readOnly) latest.current.onChange(value);
          },
        );
        instance.current.update(
          current.source,
          current.diagnostics,
          current.runtimeDiagnostics,
          current.readOnly,
        );
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

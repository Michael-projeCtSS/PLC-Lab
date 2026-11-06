import { useEffect, useRef, useState } from "react";
import type { DiagnosticDto } from "@plc-lab/ipc";
import type { createStEditor } from "./monaco-host";
import { useInterface } from "./locale";

/** Monaco-based Structured Text editor. Markers come only from the Rust compiler/runtime. */
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
  diagnostics: DiagnosticDto[];
  runtimeDiagnostics?: DiagnosticDto[];
  focusedDiagnostic: DiagnosticDto | null;
}) {
  const { t } = useInterface();
  const container = useRef<HTMLDivElement>(null);
  const instance = useRef<ReturnType<typeof createStEditor> | null>(null);
  const latest = useRef({
    source,
    readOnly,
    onChange,
    diagnostics,
    runtimeDiagnostics,
  });
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    latest.current = {
      source,
      readOnly,
      onChange,
      diagnostics,
      runtimeDiagnostics,
    };
    instance.current?.update(source, diagnostics, runtimeDiagnostics, readOnly);
  }, [source, readOnly, onChange, diagnostics, runtimeDiagnostics]);
  useEffect(() => {
    if (focusedDiagnostic) instance.current?.focus(focusedDiagnostic.span);
  }, [focusedDiagnostic]);
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
      .catch((e: unknown) => {
        if (active)
          setError(
            e instanceof Error ? e.message : "The editor could not load.",
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
      aria-label={t("editor.label")}
    >
      <div className="monaco-surface" ref={container} />
      {error && (
        <div className="st-editor-fallback">
          <p role="alert">{t("editor.failed", { error })}</p>
          <textarea
            aria-label={t("editor.source")}
            value={source}
            readOnly={readOnly}
            spellCheck={false}
            onChange={(e) => onChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
}

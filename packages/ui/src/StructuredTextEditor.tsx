import { useEffect, useRef, useState } from "react";
import type { DiagnosticDto } from "@plc-lab/ipc";
import type { SourceDiagnostics } from "./editor-sync";
import { recordProblem } from "./error-log";
import type { createStEditor } from "./monaco-host";
import { useInterface } from "./locale";

const NONE: SourceDiagnostics = { items: [], source: null };

/**
 * Monaco-based Structured Text editor. Markers come only from the Rust compiler/runtime,
 * each set with the exact text it was computed for; the Monaco model owns the text while
 * the student types (see editor-sync.ts).
 */
export function StructuredTextEditor({
  source,
  readOnly = false,
  onChange,
  diagnostics,
  runtimeDiagnostics = NONE,
  focusedDiagnostic,
}: {
  source: string;
  readOnly?: boolean;
  onChange: (value: string) => void;
  diagnostics: SourceDiagnostics;
  runtimeDiagnostics?: SourceDiagnostics;
  focusedDiagnostic: DiagnosticDto | null;
}) {
  const { t, theme } = useInterface();
  const container = useRef<HTMLDivElement>(null);
  const instance = useRef<ReturnType<typeof createStEditor> | null>(null);
  const latest = useRef({
    source,
    readOnly,
    onChange,
    diagnostics,
    runtimeDiagnostics,
    theme,
  });
  const host = useRef<typeof import("./monaco-host") | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    latest.current = {
      source,
      readOnly,
      onChange,
      diagnostics,
      runtimeDiagnostics,
      theme,
    };
    instance.current?.update(source, diagnostics, runtimeDiagnostics, readOnly);
  }, [source, readOnly, onChange, diagnostics, runtimeDiagnostics, theme]);
  useEffect(() => {
    if (focusedDiagnostic) instance.current?.focus(focusedDiagnostic.span);
  }, [focusedDiagnostic]);
  // A theme change restyles the existing editor; the program and its history stay.
  useEffect(() => {
    host.current?.setEditorTheme(theme);
  }, [theme]);
  useEffect(() => {
    let active = true;
    import("./monaco-host")
      .then((module) => {
        if (!active || !container.current) return;
        host.current = module;
        const current = latest.current;
        instance.current = module.createStEditor(
          container.current,
          current.source,
          (value) => {
            if (!latest.current.readOnly) latest.current.onChange(value);
          },
          current.theme,
          (problem) => recordProblem("editor", problem),
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

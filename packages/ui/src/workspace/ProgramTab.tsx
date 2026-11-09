import { useMemo, useState } from "react";
import { useInterface } from "../locale";
import { diagnosticTitle } from "../locale/messages";
import type { DiagnosticDto } from "@plc-lab/ipc";
import { StructuredTextEditor } from "../StructuredTextEditor";
import type { Lab } from "./useLab";

/**
 * The document in the editor: the main program or a project library document, which can
 * be renamed and deleted (ADR-0045).
 */
function DocumentBar({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const document = lab.library.find((d) => d.id === lab.activeDocument);
  const [name, setName] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  if (!document)
    return lab.library.length ? (
      <div className="document-bar" data-testid="document-bar">
        <strong>{lab.project?.programs[0]?.name ?? "Main"}</strong>
        <span className="muted">{t("doc.main")}</span>
      </div>
    ) : null;
  const busy = lab.busy !== null;
  return (
    <div className="document-bar" data-testid="document-bar">
      {name === null ? (
        <>
          <strong>{document.name}</strong>
          <span className="muted">{t("doc.libraryHelp")}</span>
          <span className="spacer" />
          <button
            className="btn btn-sm"
            disabled={busy}
            onClick={() => {
              setDeleting(false);
              setName(document.name);
            }}
          >
            {t("doc.rename")}
          </button>
          {deleting ? (
            <>
              <button
                className="btn btn-sm btn-danger"
                disabled={busy}
                onClick={() => {
                  setDeleting(false);
                  void lab.deleteDocument(document.id);
                }}
              >
                {t("doc.confirmDelete", { name: document.name })}
              </button>
              <button
                className="btn btn-sm btn-ghost"
                onClick={() => setDeleting(false)}
              >
                {t("doc.cancel")}
              </button>
            </>
          ) : (
            <button
              className="btn btn-sm btn-ghost"
              disabled={busy}
              onClick={() => setDeleting(true)}
            >
              {t("doc.delete")}
            </button>
          )}
        </>
      ) : (
        <form
          className="row"
          style={{ gap: 6 }}
          onSubmit={(e) => {
            e.preventDefault();
            const next = name.trim();
            if (!next) return;
            void lab
              .renameDocument(document.id, next)
              .then((ok) => ok && setName(null));
          }}
        >
          <input
            className="input"
            aria-label={t("doc.name")}
            value={name}
            maxLength={64}
            autoFocus
            onChange={(e) => setName(e.target.value)}
          />
          <button
            className="btn btn-sm btn-primary"
            type="submit"
            disabled={busy || !name.trim()}
          >
            {t("doc.save")}
          </button>
          <button
            className="btn btn-sm btn-ghost"
            type="button"
            onClick={() => setName(null)}
          >
            {t("doc.cancel")}
          </button>
        </form>
      )}
    </div>
  );
}

export function ProgramTab({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const [focused, setFocused] = useState<DiagnosticDto | null>(null);
  const active = lab.activeDocument;
  // Stable marker list: only a different fault (by content) updates editor markers.
  const faultKey = lab.frame?.plc.fault
    ? JSON.stringify(lab.frame.plc.fault)
    : "";
  // A runtime fault's position refers to the program the PLC runs, in its document.
  const runtime = useMemo(() => {
    const fault = faultKey ? (JSON.parse(faultKey) as DiagnosticDto) : null;
    return {
      items: fault && (fault.document ?? null) === active ? [fault] : [],
      source:
        active === null
          ? lab.loadedSource
          : (lab.loadedLibrary[active] ?? null),
    };
  }, [faultKey, active, lab.loadedSource, lab.loadedLibrary]);
  // Compiler diagnostics mark the document they are in, in the text they were made for.
  const compiler = useMemo(
    () => ({
      items: lab.diagnostics.filter((d) => (d.document ?? null) === active),
      source:
        active === null
          ? lab.diagnosticsSource
          : (lab.diagnosticsLibrary[active] ?? null),
    }),
    [lab.diagnostics, lab.diagnosticsSource, lab.diagnosticsLibrary, active],
  );
  return (
    <div className="program-panel">
      <DocumentBar key={active ?? ""} lab={lab} />
      {lab.diagnostics.length > 0 && (
        <div className="compile-problems" role="alert">
          <strong>{t("control.failed")}</strong>
          {lab.diagnostics.map((d, i) => {
            const document = lab.documentName(d.document);
            return (
              <button
                key={i}
                className="btn btn-ghost btn-sm"
                onClick={() => {
                  lab.setActiveDocument(d.document ?? null);
                  setFocused(d);
                }}
              >
                {document ? `${document} · ` : ""}
                {d.span ? `${d.span.start.line}: ` : ""}
                {d.code}
                {diagnosticTitle(t, d.code)} · {d.message}
              </button>
            );
          })}
        </div>
      )}
      <StructuredTextEditor
        source={lab.editorText}
        onChange={lab.setEditorText}
        diagnostics={compiler}
        runtimeDiagnostics={runtime}
        focusedDiagnostic={
          focused && (focused.document ?? null) === active ? focused : null
        }
      />
    </div>
  );
}

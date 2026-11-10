import { useMemo, useState } from "react";
import { useInterface } from "../locale";
import { diagnosticTitle } from "../locale/messages";
import type { DiagnosticDto } from "@plc-lab/ipc";
import { StructuredTextEditor } from "../StructuredTextEditor";
import type { Lab } from "./useLab";
import { LadEditor } from "./LadEditor";
import { newLadDocument, parseLad, serializeLad } from "./ladModel";

/** A Structured Text program declaring the ladder program's variables. */
function stSkeleton(lab: Lab): string {
  const parsed = parseLad(lab.source);
  const variables = "document" in parsed ? parsed.document.variables : [];
  const block = (keyword: string, direction: string) => {
    const lines = variables
      .filter((v) => v.direction === direction)
      .map(
        (v) =>
          `    ${v.name} : BOOL;${v.comment ? ` (* ${v.comment.replaceAll("*)", "")} *)` : ""}`,
      );
    return lines.length ? [keyword, ...lines, "END_VAR"] : [];
  };
  return [
    "PROGRAM Main",
    ...block("VAR_INPUT", "input"),
    ...block("VAR_OUTPUT", "output"),
    ...block("VAR", "local"),
    "",
    "END_PROGRAM",
    "",
  ].join("\n");
}

/** Free Lab programs choose their language: Structured Text or Ladder Diagram (ADR-0050). */
function LanguageSwitch({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const [target, setTarget] = useState<"ST" | "LAD" | null>(null);
  if (
    lab.workspace?.exercise ||
    lab.activeDocument !== null ||
    lab.library.length
  )
    return null;
  const current = lab.mainLanguage === "LAD" ? "LAD" : "ST";
  const replacement = (language: "ST" | "LAD") =>
    language === "LAD"
      ? serializeLad(
          newLadDocument("Main", lab.workspace?.scene.interface ?? []),
        )
      : stSkeleton(lab);
  return (
    <div className="document-bar" data-testid="language-bar">
      <span>{t("lang.label")}</span>
      <div className="segmented" role="group" aria-label={t("lang.label")}>
        {(["ST", "LAD"] as const).map((language) => (
          <button
            key={language}
            aria-pressed={current === language}
            disabled={lab.busy !== null}
            onClick={() => setTarget(language === current ? null : language)}
          >
            {t(language === "ST" ? "lang.st" : "lang.lad")}
          </button>
        ))}
      </div>
      {target && (
        <>
          <span className="muted">{t("lang.replaceWarning")}</span>
          <button
            className="btn btn-sm btn-danger"
            onClick={() => {
              const language = target;
              setTarget(null);
              void lab.switchLanguage(language, replacement(language));
            }}
          >
            {t(target === "LAD" ? "lang.toLad" : "lang.toSt")}
          </button>
          <button
            className="btn btn-sm btn-ghost"
            onClick={() => setTarget(null)}
          >
            {t("doc.cancel")}
          </button>
        </>
      )}
    </div>
  );
}

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
      <LanguageSwitch lab={lab} />
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
      {active === null && lab.mainLanguage === "LAD" ? (
        <LadEditor lab={lab} />
      ) : (
        <StructuredTextEditor
          source={lab.editorText}
          onChange={lab.setEditorText}
          diagnostics={compiler}
          runtimeDiagnostics={runtime}
          focusedDiagnostic={
            focused && (focused.document ?? null) === active ? focused : null
          }
        />
      )}
    </div>
  );
}

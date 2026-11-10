/**
 * The language of a Free Lab main program: Structured Text or Ladder Diagram (ADR-0050).
 * It sits in the view tabs so the editor keeps its full height. Switching starts a new
 * program in the other language after a confirmation; nothing is converted.
 */
import { useState } from "react";
import { useInterface } from "../locale";
import type { Lab } from "./useLab";
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

export function LanguageSwitch({ lab }: { lab: Lab }) {
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
    <div className="row language-switch" data-testid="language-bar">
      <div className="segmented" role="group" aria-label={t("lang.label")}>
        {(["ST", "LAD"] as const).map((language) => (
          <button
            key={language}
            aria-pressed={current === language}
            aria-label={t(language === "ST" ? "lang.st" : "lang.lad")}
            title={t(language === "ST" ? "lang.st" : "lang.lad")}
            disabled={lab.busy !== null}
            onClick={() => setTarget(language === current ? null : language)}
          >
            {language}
          </button>
        ))}
      </div>
      {target && (
        <>
          <button
            className="btn btn-sm btn-danger"
            title={t("lang.replaceWarning")}
            aria-description={t("lang.replaceWarning")}
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

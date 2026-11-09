/**
 * Library browser (ADR-0044): the program's own functions and function blocks from the
 * latest successful compile, and the standard blocks of the PLC runtime, with
 * signatures, parameters, descriptions and usage examples.
 */
import { api, type PouDto, type StandardBlockDto } from "@plc-lab/ipc";
import { useState } from "react";
import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { Badge } from "../ui";
import { useLoad } from "../useLoad";
import {
  pouExample,
  signature,
  standardSignature,
  STANDARD_EXAMPLES,
} from "./libraryText";

const DIRECTION: Record<string, TranslationKey> = {
  Input: "lib.direction.Input",
  Output: "lib.direction.Output",
  InOut: "lib.direction.InOut",
};
const STANDARD: Record<string, TranslationKey> = {
  TON: "lib.std.TON",
  TOF: "lib.std.TOF",
  TP: "lib.std.TP",
  R_TRIG: "lib.std.R_TRIG",
  F_TRIG: "lib.std.F_TRIG",
  CTU: "lib.std.CTU",
  CTD: "lib.std.CTD",
  PID: "lib.std.PID",
};

function Example({ code }: { code: string }) {
  const { t } = useInterface();
  return (
    <div className="lib-example">
      <div className="row" style={{ gap: 8 }}>
        <span className="muted">{t("lib.example")}</span>
        <button
          className="btn btn-sm btn-ghost"
          onClick={() =>
            void navigator.clipboard?.writeText(code).catch(() => undefined)
          }
        >
          {t("lib.copy")}
        </button>
      </div>
      <pre className="mono">{code}</pre>
    </div>
  );
}

function Parameters({
  parameters,
}: {
  parameters: {
    name: string;
    dataType: string;
    direction: string;
    description?: string | null;
  }[];
}) {
  const { t } = useInterface();
  if (!parameters.length)
    return <p className="muted">{t("lib.noParameters")}</p>;
  return (
    <table className="table lib-parameters">
      <thead>
        <tr>
          <th>{t("lib.parameter")}</th>
          <th>{t("lib.type")}</th>
          <th>{t("lib.direction")}</th>
          <th>{t("lib.description")}</th>
        </tr>
      </thead>
      <tbody>
        {parameters.map((p) => (
          <tr key={p.name}>
            <td className="mono">{p.name}</td>
            <td className="mono">{p.dataType}</td>
            <td>{t(DIRECTION[p.direction] ?? "lib.direction.Input")}</td>
            <td>{p.description ?? ""}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function UserPou({
  pou,
  documentName,
}: {
  pou: PouDto;
  documentName: (id: string | null) => string | null;
}) {
  const { t } = useInterface();
  const document = documentName(pou.document);
  return (
    <details className="lib-entry" data-testid={`lib-${pou.name}`}>
      <summary>
        <span className="mono">{pou.name}</span>{" "}
        <Badge kind="normal">
          {t(pou.kind === "FUNCTION" ? "lib.function" : "lib.functionBlock")}
        </Badge>
      </summary>
      <code className="lib-signature">
        {signature(pou.name, pou.parameters, pou.returnType)}
      </code>
      <p>{pou.description ?? t("lib.noDescription")}</p>
      <p className="muted">
        {document
          ? t("lib.lineIn", { line: pou.line, document })
          : t("lib.line", { line: pou.line })}
      </p>
      <Parameters parameters={pou.parameters} />
      <Example code={pouExample(pou)} />
    </details>
  );
}

function Standard({ block }: { block: StandardBlockDto }) {
  const { t } = useInterface();
  const description = STANDARD[block.name];
  const example = STANDARD_EXAMPLES[block.name];
  return (
    <details className="lib-entry" data-testid={`lib-${block.name}`}>
      <summary>
        <span className="mono">{block.name}</span>{" "}
        <Badge kind="normal">{t("lib.functionBlock")}</Badge>
      </summary>
      <code className="lib-signature">{standardSignature(block)}</code>
      <p>{description ? t(description) : ""}</p>
      <Parameters parameters={block.parameters} />
      {example && <Example code={example} />}
    </details>
  );
}

export function LibraryTab({
  pous,
  current,
  documentName = () => null,
}: {
  pous: PouDto[];
  current: boolean;
  /** The name of a project library document (ADR-0045). */
  documentName?: (id: string | null) => string | null;
}) {
  const { t } = useInterface();
  const library = useLoad(() => api.library(), []);
  const [query, setQuery] = useState("");
  const match = (name: string) =>
    name.toLowerCase().includes(query.trim().toLowerCase());
  const functions = pous.filter((p) => p.kind === "FUNCTION" && match(p.name));
  const blocks = pous.filter(
    (p) => p.kind === "FUNCTION_BLOCK" && match(p.name),
  );
  const standard = (library.data?.standardBlocks ?? []).filter((b) =>
    match(b.name),
  );
  return (
    <div className="library-browser">
      <section className="inspector-section">
        <input
          className="input"
          type="search"
          value={query}
          placeholder={t("lib.search")}
          aria-label={t("lib.search")}
          onChange={(e) => setQuery(e.target.value)}
        />
      </section>
      <section className="inspector-section" aria-label={t("lib.user")}>
        <h4>{t("lib.user")}</h4>
        <p className="muted">{t("lib.userHelp")}</p>
        {!current && (
          <p className="notice warning" role="note">
            {t("lib.stale")}
          </p>
        )}
        <h5>{t("lib.functionBlocks")}</h5>
        {blocks.length ? (
          blocks.map((p) => (
            <UserPou key={p.name} pou={p} documentName={documentName} />
          ))
        ) : (
          <p className="muted">{t("lib.none")}</p>
        )}
        <h5>{t("lib.functions")}</h5>
        {functions.length ? (
          functions.map((p) => (
            <UserPou key={p.name} pou={p} documentName={documentName} />
          ))
        ) : (
          <p className="muted">{t("lib.none")}</p>
        )}
      </section>
      <section className="inspector-section" aria-label={t("lib.standard")}>
        <h4>{t("lib.standard")}</h4>
        {standard.map((b) => (
          <Standard key={b.name} block={b} />
        ))}
      </section>
    </div>
  );
}

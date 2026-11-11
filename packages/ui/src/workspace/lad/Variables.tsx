/** The variables of a ladder program: name, type, kind and comment (ADR-0050, ADR-0051). */
import { useState } from "react";
import {
  LAD_TYPES,
  addVariable,
  removeVariable,
  setDirection,
  setVariableType,
  type Direction,
  type LadDocument,
  type LadType,
} from "../ladModel";
import type { T } from "./common";

const DIRECTIONS: Direction[] = ["input", "output", "local"];

export function Variables({
  t,
  document,
  commit,
}: {
  t: T;
  document: LadDocument;
  commit: (d: LadDocument) => void;
}) {
  const [name, setName] = useState("");
  const [direction, setNewDirection] = useState<Direction>("local");
  const [type, setType] = useState<LadType>("BOOL");
  const [invalid, setInvalid] = useState(false);
  return (
    <section className="lad-variables" aria-label={t("lad.variables")}>
      <h3>{t("lad.variables")}</h3>
      <table className="table">
        <thead>
          <tr>
            <th>{t("lad.variableName")}</th>
            <th>{t("lad.variableType")}</th>
            <th>{t("lad.direction")}</th>
            <th>{t("lad.variableComment")}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {document.variables.map((v) => (
            <tr key={v.name}>
              <th scope="row" className="mono">
                {v.name}
              </th>
              <td>
                <select
                  className="input"
                  aria-label={t("lad.typeOf", { name: v.name })}
                  value={v.type}
                  onChange={(e) =>
                    commit(
                      setVariableType(
                        document,
                        v.name,
                        e.target.value as LadType,
                      ),
                    )
                  }
                >
                  {LAD_TYPES.map((x) => (
                    <option key={x} value={x}>
                      {x}
                    </option>
                  ))}
                </select>
              </td>
              <td>
                <select
                  className="input"
                  aria-label={t("lad.directionOf", { name: v.name })}
                  value={v.direction}
                  onChange={(e) =>
                    commit(
                      setDirection(
                        document,
                        v.name,
                        e.target.value as Direction,
                      ),
                    )
                  }
                >
                  {DIRECTIONS.map((d) => (
                    <option key={d} value={d}>
                      {t(`lad.dir.${d}`)}
                    </option>
                  ))}
                </select>
              </td>
              <td className="muted">{v.comment ?? ""}</td>
              <td>
                <button
                  className="btn btn-sm btn-ghost"
                  aria-label={t("lad.removeVariable", { name: v.name })}
                  onClick={() => commit(removeVariable(document, v.name))}
                >
                  ×
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <form
        className="row"
        style={{ gap: 6 }}
        onSubmit={(e) => {
          e.preventDefault();
          const next = addVariable(document, name, direction, type);
          setInvalid(next === null);
          if (next) {
            commit(next);
            setName("");
          }
        }}
      >
        <input
          className="input"
          aria-label={t("lad.newVariable")}
          aria-invalid={invalid}
          value={name}
          maxLength={40}
          onChange={(e) => setName(e.target.value)}
        />
        <select
          className="input"
          aria-label={t("lad.newType")}
          value={type}
          onChange={(e) => setType(e.target.value as LadType)}
        >
          {LAD_TYPES.map((x) => (
            <option key={x} value={x}>
              {x}
            </option>
          ))}
        </select>
        <select
          className="input"
          aria-label={t("lad.newDirection")}
          value={direction}
          onChange={(e) => setNewDirection(e.target.value as Direction)}
        >
          {DIRECTIONS.map((d) => (
            <option key={d} value={d}>
              {t(`lad.dir.${d}`)}
            </option>
          ))}
        </select>
        <button className="btn btn-sm" type="submit">
          {t("lad.addVariable")}
        </button>
      </form>
    </section>
  );
}

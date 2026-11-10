/**
 * Ladder Diagram editor (ADR-0050): a rung/grid editor over the ladder document, not a
 * node graph. The document is the program text the workspace saves, compiles and loads;
 * the backend compiles it into the canonical IR and runs it in the one PLC runtime.
 *
 * Online, each contact and coil shows the value of its variable as the runtime reports
 * it. The editor never evaluates the network: it computes no power flow.
 */
import { useMemo, useState, type ReactNode } from "react";
import { api, type SignalValueDto, type WatchDto } from "@plc-lab/ipc";
import { useInterface } from "../locale";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";
import {
  addCoil,
  addRung,
  addVariable,
  insertParallel,
  insertSeries,
  newLadDocument,
  newestElement,
  parseLad,
  removeCoil,
  removeElement,
  removeRung,
  removeVariable,
  serializeLad,
  setCoilVariable,
  setContactVariable,
  setDirection,
  setRungComment,
  toggleContact,
  type ContactKind,
  type Direction,
  type LadDocument,
  type LadNode,
} from "./ladModel";

type Selected = { kind: "node" | "coil"; id: number } | null;

export function LadEditor({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const parsed = useMemo(() => parseLad(lab.source), [lab.source]);
  const [selected, setSelected] = useState<Selected>(null);
  const live = useLadValues(lab, "document" in parsed ? parsed.document : null);
  if ("error" in parsed)
    return (
      <div className="lad" role="alert">
        <p>{t("lad.invalid", { error: parsed.error })}</p>
        <button
          className="btn"
          onClick={() =>
            lab.setSource(
              serializeLad(
                newLadDocument("Main", lab.workspace?.scene.interface ?? []),
              ),
            )
          }
        >
          {t("lad.startOver")}
        </button>
      </div>
    );
  const document = parsed.document;
  const update = (next: LadDocument) => lab.setSource(serializeLad(next));
  const coilTargets = document.variables.filter((v) => v.direction !== "input");
  return (
    <div className="lad" data-testid="lad-editor">
      <LadToolbar
        document={document}
        selected={selected}
        update={update}
        select={setSelected}
      />
      {live.online ? (
        <p className="muted lad-status" role="status">
          {t("lad.online")}
        </p>
      ) : (
        <p className="muted lad-status" role="status">
          {t("lad.offline")}
        </p>
      )}
      <div className="lad-rungs" role="list" aria-label={t("lad.rungs")}>
        {document.rungs.map((rung, index) => (
          <section
            key={rung.id}
            className="lad-rung"
            role="listitem"
            aria-label={t("lad.rung", { number: index + 1 })}
          >
            <header className="row" style={{ gap: 6 }}>
              <strong>{t("lad.rung", { number: index + 1 })}</strong>
              <input
                className="input lad-comment"
                aria-label={t("lad.comment", { number: index + 1 })}
                placeholder={t("lad.commentPlaceholder")}
                value={rung.comment ?? ""}
                onChange={(e) =>
                  update(setRungComment(document, rung.id, e.target.value))
                }
              />
              <span className="spacer" />
              <button
                className="btn btn-sm"
                onClick={() => update(addCoil(document, rung.id))}
              >
                {t("lad.addCoil")}
              </button>
              <button
                className="btn btn-sm btn-ghost"
                onClick={() => update(removeRung(document, rung.id))}
              >
                {t("lad.removeRung")}
              </button>
            </header>
            <div className="lad-line">
              <span className="lad-rail" aria-hidden="true" />
              <div className="lad-network">
                <NetworkView
                  node={rung.network}
                  root
                  selected={selected}
                  select={setSelected}
                  value={live.value}
                />
              </div>
              <span className="lad-wire grow" aria-hidden="true" />
              <div className="lad-coils">
                {rung.coils.map((coil) => {
                  const v = live.value(coil.variable);
                  return (
                    <button
                      key={coil.id}
                      className={`lad-coil${on(v) ? " on" : ""}${
                        selected?.kind === "coil" && selected.id === coil.id
                          ? " selected"
                          : ""
                      }`}
                      aria-pressed={
                        selected?.kind === "coil" && selected.id === coil.id
                      }
                      aria-label={describe(
                        t("lad.coilNamed", {
                          variable: coil.variable || t("lad.noVariable"),
                        }),
                        v,
                      )}
                      onClick={() => setSelected({ kind: "coil", id: coil.id })}
                    >
                      <span className="lad-symbol">( )</span>
                      <span className="lad-name">
                        {coil.variable || t("lad.noVariable")}
                      </span>
                      {v && <span className="lad-value">{text(v)}</span>}
                    </button>
                  );
                })}
              </div>
              <span className="lad-rail" aria-hidden="true" />
            </div>
          </section>
        ))}
        <button className="btn" onClick={() => update(addRung(document))}>
          {t("lad.addRung")}
        </button>
      </div>
      <VariableAssignment
        document={document}
        selected={selected}
        update={update}
        coilTargets={coilTargets.map((v) => v.name)}
      />
      <Variables document={document} update={update} />
    </div>
  );
}

function on(value: SignalValueDto | null): boolean {
  return value?.type === "BOOL" && value.value;
}
function text(value: SignalValueDto): string {
  return value.type === "BOOL" ? (value.value ? "TRUE" : "FALSE") : "";
}
function describe(label: string, value: SignalValueDto | null): string {
  return value ? `${label}: ${text(value)}` : label;
}

/** The values of the document's variables, while the PLC runs this very document. */
function useLadValues(lab: Lab, document: LadDocument | null) {
  const ws = lab.ws;
  const online =
    !!document &&
    lab.loadedSource === lab.source &&
    lab.frame?.plc.mode === "RUN";
  const ids = useMemo(() => {
    if (!document) return new Map<string, number>();
    const byPath = new Map(
      lab.signals.map((s) => [s.path.toLowerCase(), s.id]),
    );
    return new Map(
      document.variables.flatMap((v) => {
        const id = byPath.get(`plc.${v.name}`.toLowerCase());
        return id === undefined ? [] : [[v.name.toLowerCase(), id] as const];
      }),
    );
  }, [document, lab.signals]);
  const signals = [...ids.values()];
  const watch = useFrameQuery<WatchDto | null>(
    lab,
    () =>
      ws && online && signals.length
        ? api.watch(ws, signals)
        : Promise.resolve(null),
    [online, signals.join(",")],
  );
  const values = new Map(
    (watch.data?.rows ?? []).map((r) => [r.signal, r.value]),
  );
  return {
    online,
    value: (name: string): SignalValueDto | null => {
      if (!online) return null;
      const id = ids.get(name.toLowerCase());
      return id === undefined ? null : (values.get(id) ?? null);
    },
  };
}

function NetworkView({
  node,
  root = false,
  selected,
  select,
  value,
}: {
  node: LadNode;
  root?: boolean;
  selected: Selected;
  select: (s: Selected) => void;
  value: (name: string) => SignalValueDto | null;
}): ReactNode {
  const { t } = useInterface();
  const isSelected = selected?.kind === "node" && selected.id === node.id;
  if (node.type === "contact") {
    const v = value(node.variable);
    const label = t(
      node.contact === "normallyOpen" ? "lad.noContact" : "lad.ncContact",
      { variable: node.variable || t("lad.noVariable") },
    );
    return (
      <button
        className={`lad-contact${on(v) ? " on" : ""}${isSelected ? " selected" : ""}`}
        aria-pressed={isSelected}
        aria-label={describe(label, v)}
        onClick={() => select({ kind: "node", id: node.id })}
      >
        <span className="lad-symbol">
          {node.contact === "normallyOpen" ? "| |" : "|/|"}
        </span>
        <span className="lad-name">{node.variable || t("lad.noVariable")}</span>
        {v && <span className="lad-value">{text(v)}</span>}
      </button>
    );
  }
  if (node.type === "series") {
    if (!node.items.length)
      return (
        <button
          className={`lad-empty${isSelected ? " selected" : ""}`}
          aria-pressed={isSelected}
          aria-label={t("lad.empty")}
          onClick={() => select({ kind: "node", id: node.id })}
        >
          {t("lad.empty")}
        </button>
      );
    return (
      <div className={`lad-series${root ? " root" : ""}`}>
        {node.items.map((item) => (
          <NetworkView
            key={item.id}
            node={item}
            selected={selected}
            select={select}
            value={value}
          />
        ))}
      </div>
    );
  }
  return (
    <div className={`lad-parallel${isSelected ? " selected" : ""}`}>
      <button
        className="lad-block"
        aria-pressed={isSelected}
        aria-label={t("lad.parallelBlock", { count: node.branches.length })}
        onClick={() => select({ kind: "node", id: node.id })}
      >
        ∥
      </button>
      <div className="lad-branches">
        {node.branches.map((branch) => (
          <div key={branch.id} className="lad-branch">
            <NetworkView
              node={branch}
              selected={selected}
              select={select}
              value={value}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

function LadToolbar({
  document,
  selected,
  update,
  select,
}: {
  document: LadDocument;
  selected: Selected;
  update: (d: LadDocument) => void;
  select: (s: Selected) => void;
}) {
  const { t } = useInterface();
  const node = selected?.kind === "node" ? selected.id : null;
  const coil = selected?.kind === "coil" ? selected.id : null;
  const add = (where: "series" | "parallel", kind: ContactKind) => {
    if (node === null) return;
    const next =
      where === "series"
        ? insertSeries(document, node, kind)
        : insertParallel(document, node, kind);
    update(next);
    // Continue with the new contact: its variable is chosen next.
    select({ kind: "node", id: newestElement(next) });
  };
  const isContact =
    node !== null &&
    document.rungs.some((r) => findNode(r.network, node)?.type === "contact");
  return (
    <div
      className="lad-toolbar row wrap"
      role="toolbar"
      aria-label={t("lad.tools")}
    >
      <span className="muted">
        {selected ? t("lad.selected") : t("lad.selectHint")}
      </span>
      <button
        className="btn btn-sm"
        disabled={node === null}
        onClick={() => add("series", "normallyOpen")}
      >
        {t("lad.seriesNo")}
      </button>
      <button
        className="btn btn-sm"
        disabled={node === null}
        onClick={() => add("series", "normallyClosed")}
      >
        {t("lad.seriesNc")}
      </button>
      <button
        className="btn btn-sm"
        disabled={node === null}
        onClick={() => add("parallel", "normallyOpen")}
      >
        {t("lad.parallelNo")}
      </button>
      <button
        className="btn btn-sm"
        disabled={node === null}
        onClick={() => add("parallel", "normallyClosed")}
      >
        {t("lad.parallelNc")}
      </button>
      <button
        className="btn btn-sm"
        disabled={!isContact}
        onClick={() => node !== null && update(toggleContact(document, node))}
      >
        {t("lad.toggle")}
      </button>
      <button
        className="btn btn-sm btn-ghost"
        disabled={selected === null}
        onClick={() => {
          if (node !== null) update(removeElement(document, node));
          if (coil !== null) update(removeCoil(document, coil));
          select(null);
        }}
      >
        {t("lad.delete")}
      </button>
    </div>
  );
}

function findNode(node: LadNode, id: number): LadNode | null {
  if (node.id === id) return node;
  if (node.type === "contact") return null;
  for (const child of node.type === "series" ? node.items : node.branches) {
    const found = findNode(child, id);
    if (found) return found;
  }
  return null;
}

/** The variable of the selected contact or coil. */
function VariableAssignment({
  document,
  selected,
  update,
  coilTargets,
}: {
  document: LadDocument;
  selected: Selected;
  update: (d: LadDocument) => void;
  coilTargets: string[];
}) {
  const { t } = useInterface();
  if (!selected) return null;
  const current = assignedVariable(document, selected);
  if (current === null) return null;
  const options =
    selected.kind === "coil"
      ? coilTargets
      : document.variables.map((v) => v.name);
  return (
    <label className="field lad-assign">
      {selected.kind === "coil"
        ? t("lad.coilVariable")
        : t("lad.contactVariable")}
      <select
        className="input"
        value={current}
        onChange={(e) =>
          update(
            selected.kind === "coil"
              ? setCoilVariable(document, selected.id, e.target.value)
              : setContactVariable(document, selected.id, e.target.value),
          )
        }
      >
        <option value="">{t("lad.noVariable")}</option>
        {options.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
    </label>
  );
}

/** The variable of the selected contact or coil; `null` for other elements. */
function assignedVariable(
  document: LadDocument,
  selected: { kind: "node" | "coil"; id: number },
): string | null {
  if (selected.kind === "coil")
    return (
      document.rungs.flatMap((r) => r.coils).find((c) => c.id === selected.id)
        ?.variable ?? null
    );
  const node = document.rungs
    .map((r) => findNode(r.network, selected.id))
    .find((n) => n !== null);
  return node?.type === "contact" ? node.variable : null;
}

const DIRECTIONS: Direction[] = ["input", "output", "local"];

function Variables({
  document,
  update,
}: {
  document: LadDocument;
  update: (d: LadDocument) => void;
}) {
  const { t } = useInterface();
  const [name, setName] = useState("");
  const [direction, setNewDirection] = useState<Direction>("local");
  const [invalid, setInvalid] = useState(false);
  return (
    <section className="lad-variables" aria-label={t("lad.variables")}>
      <h3>{t("lad.variables")}</h3>
      <table className="table">
        <thead>
          <tr>
            <th>{t("lad.variableName")}</th>
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
                  aria-label={t("lad.directionOf", { name: v.name })}
                  value={v.direction}
                  onChange={(e) =>
                    update(
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
                  onClick={() => update(removeVariable(document, v.name))}
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
          const next = addVariable(document, name, direction);
          setInvalid(next === null);
          if (next) {
            update(next);
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

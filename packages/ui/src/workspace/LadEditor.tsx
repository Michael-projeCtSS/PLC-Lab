/**
 * Ladder Diagram editor (ADR-0050, ADR-0051): a rung/grid editor over the ladder
 * document, not a node graph. The document is the program text the workspace saves,
 * compiles and loads; the backend compiles it into the canonical IR and runs it in the
 * one PLC runtime.
 *
 * Online, everything shown on the rungs comes from `ladder_state`: the power leaving each
 * element as the runtime computed it, the values its variables had in that scan, timer
 * elapsed times and counter values. The editor computes no power flow.
 */
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  api,
  type DiagnosticDto,
  type LadderStateDto,
  type SignalValueDto,
} from "@plc-lab/ipc";
import { formatValue } from "../format";
import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";
import {
  LAD_TYPES,
  addCoil,
  addRung,
  addVariable,
  duplicateRung,
  findCoil,
  findNode,
  insertParallel,
  insertSeries,
  isTimer,
  moveElement,
  moveRung,
  newLadDocument,
  newestElement,
  parseLad,
  pasteRung,
  readingOrder,
  removeCoil,
  removeElement,
  removeRung,
  removeVariable,
  rungOf,
  serializeLad,
  setBlockPin,
  setCoil,
  setDirection,
  setRungComment,
  setVariableType,
  updateElement,
  type BlockKind,
  type CoilKind,
  type CompareOp,
  type ContactKind,
  type Direction,
  type ElementSpec,
  type LadDocument,
  type LadNode,
  type LadType,
  type Operand,
  type Rung,
} from "./ladModel";

type Selected = { kind: "node" | "coil" | "rung"; id: number } | null;
type Where = "before" | "after" | "parallel";
type T = (
  key: TranslationKey,
  values?: Record<string, string | number>,
) => string;

const CONTACTS: { kind: ContactKind; symbol: string; label: TranslationKey }[] =
  [
    { kind: "normallyOpen", symbol: "| |", label: "lad.noContact" },
    { kind: "normallyClosed", symbol: "|/|", label: "lad.ncContact" },
    { kind: "positiveEdge", symbol: "|P|", label: "lad.pContact" },
    { kind: "negativeEdge", symbol: "|N|", label: "lad.nContact" },
  ];
const OPS: { op: CompareOp; symbol: string }[] = [
  { op: "eq", symbol: "=" },
  { op: "ne", symbol: "<>" },
  { op: "gt", symbol: ">" },
  { op: "ge", symbol: ">=" },
  { op: "lt", symbol: "<" },
  { op: "le", symbol: "<=" },
];
const BLOCKS: BlockKind[] = ["TON", "TOF", "TP", "CTU", "CTD"];
const COILS: { kind: CoilKind; symbol: string; label: TranslationKey }[] = [
  { kind: "output", symbol: "( )", label: "lad.addCoil" },
  { kind: "set", symbol: "(S)", label: "lad.addSetCoil" },
  { kind: "reset", symbol: "(R)", label: "lad.addResetCoil" },
];
const ZOOMS = [0.6, 0.7, 0.8, 0.9, 1, 1.1, 1.25, 1.5];
const HISTORY = 100;

const symbolOf = (kind: ContactKind) =>
  CONTACTS.find((c) => c.kind === kind)?.symbol ?? "| |";
const coilSymbol = (kind: CoilKind | undefined) =>
  kind === "set" ? "(S)" : kind === "reset" ? "(R)" : "( )";
const operandText = (o: Operand | undefined) =>
  !o ? "" : "variable" in o ? o.variable : o.constant;
const isEditing = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);

/** What the runtime reported for this ladder program in its latest recorded cycle. */
type Observed = {
  online: boolean;
  scanned: boolean;
  cycle: string | null;
  power: (element: number) => boolean | null;
  value: (name: string) => SignalValueDto | null;
};

function useObserved(lab: Lab, document: LadDocument | null): Observed {
  const ws = lab.ws;
  const loaded = !!document && lab.loadedSource === lab.source;
  const state = useFrameQuery<LadderStateDto | null>(
    lab,
    () => (ws && loaded ? api.ladderState(ws) : Promise.resolve(null)),
    [loaded],
  );
  const byPath = useMemo(
    () => new Map(lab.signals.map((s) => [s.path.toLowerCase(), s.id])),
    [lab.signals],
  );
  const data = loaded ? state.data : null;
  const power = new Map((data?.power ?? []).map((p) => [p.element, p.powered]));
  const values = new Map((data?.values ?? []).map((v) => [v.signal, v.value]));
  return {
    online: !!data,
    scanned: data?.scanned ?? false,
    cycle: data?.cycle ?? null,
    power: (element) => power.get(element) ?? null,
    value: (name) => {
      const id = byPath.get(`plc.${name}`.toLowerCase());
      return id === undefined ? null : (values.get(id) ?? null);
    },
  };
}

function on(value: SignalValueDto | null): boolean {
  return value?.type === "BOOL" && value.value;
}

export function LadEditor({
  lab,
  focused = null,
}: {
  lab: Lab;
  focused?: DiagnosticDto | null;
}) {
  const { t } = useInterface();
  const parsed = useMemo(() => parseLad(lab.source), [lab.source]);
  const document = "document" in parsed ? parsed.document : null;
  const [selected, setSelected] = useState<Selected>(null);
  const [where, setWhere] = useState<Where>("after");
  const [zoom, setZoom] = useState(1);
  const [clipboard, setClipboard] = useState<Rung | null>(null);
  const [history, setHistory] = useState<{ past: string[]; future: string[] }>({
    past: [],
    future: [],
  });
  const root = useRef<HTMLDivElement>(null);
  const observed = useObserved(lab, document);
  // A diagnostic chosen in the problem list selects its element.
  const [chosen, setChosen] = useState(focused);
  if (focused !== chosen) {
    setChosen(focused);
    if (focused?.element != null && document)
      setSelected(kindOf(document, focused.element));
  }
  // The selected element keeps the keyboard focus.
  useEffect(() => {
    if (!selected) return;
    const el = root.current?.querySelector<HTMLElement>(
      `[data-element="${selected.id}"]`,
    );
    if (el && !el.contains(window.document.activeElement)) el.focus();
  }, [selected]);
  if (!document)
    return (
      <div className="lad" role="alert">
        <p>
          {t("lad.invalid", { error: "error" in parsed ? parsed.error : "" })}
        </p>
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

  const commit = (next: LadDocument) => {
    const text = serializeLad(next);
    if (text === lab.source) return;
    setHistory((h) => ({
      past: [...h.past, lab.source].slice(-HISTORY),
      future: [],
    }));
    lab.setSource(text);
  };
  const undo = () => {
    const previous = history.past.at(-1);
    if (previous === undefined) return;
    setHistory((h) => ({
      past: h.past.slice(0, -1),
      future: [lab.source, ...h.future],
    }));
    lab.setSource(previous);
  };
  const redo = () => {
    const next = history.future[0];
    if (next === undefined) return;
    setHistory((h) => ({
      past: [...h.past, lab.source],
      future: h.future.slice(1),
    }));
    lab.setSource(next);
  };
  const errors = new Map<number, string[]>();
  for (const d of lab.diagnostics)
    if (d.element != null && (d.document ?? null) === null)
      errors.set(d.element, [...(errors.get(d.element) ?? []), d.message]);
  const fault = lab.frame?.plc.fault ?? null;
  if (fault?.element != null)
    errors.set(fault.element, [
      ...(errors.get(fault.element) ?? []),
      fault.message,
    ]);

  const rung = selected ? rungOf(document, selected.id) : null;
  const insert = (spec: ElementSpec) => {
    const target =
      selected?.kind === "node"
        ? selected.id
        : rung
          ? rung.network.type === "series" && rung.network.items.length
            ? (rung.network.items.at(-1)?.id ?? rung.network.id)
            : rung.network.id
          : null;
    if (target === null) return;
    const next =
      where === "parallel"
        ? insertParallel(document, target, spec)
        : insertSeries(document, target, spec, where);
    commit(next);
    // Continue with the new element: its variable or parameters are set next.
    setSelected({ kind: "node", id: newestElement(next) });
  };
  const remove = () => {
    if (!selected) return;
    if (selected.kind === "node") commit(removeElement(document, selected.id));
    if (selected.kind === "coil") commit(removeCoil(document, selected.id));
    if (selected.kind === "rung") commit(removeRung(document, selected.id));
    setSelected(null);
  };
  const order = readingOrder(document);
  const navigate = (key: string) => {
    const r = order.findIndex((ids) => selected && ids.includes(selected.id));
    if (r < 0) {
      const first = order[0]?.[0];
      if (first !== undefined) setSelected(kindOf(document, first));
      return;
    }
    const row = order[r] ?? [];
    const i = selected ? row.indexOf(selected.id) : 0;
    let target: number | undefined;
    if (key === "ArrowLeft") target = row[Math.max(0, i - 1)];
    if (key === "ArrowRight") target = row[Math.min(row.length - 1, i + 1)];
    if (key === "ArrowUp" || key === "ArrowDown") {
      const other = order[r + (key === "ArrowUp" ? -1 : 1)];
      if (other?.length) target = other[Math.min(i, other.length - 1)];
    }
    if (target !== undefined) setSelected(kindOf(document, target));
  };
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (isEditing(e.target)) return;
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key;
    if (ctrl && key.toLowerCase() === "z" && !e.shiftKey) undo();
    else if (
      ctrl &&
      (key.toLowerCase() === "y" || (key.toLowerCase() === "z" && e.shiftKey))
    )
      redo();
    else if (ctrl && key.toLowerCase() === "c" && rung) setClipboard(rung);
    else if (ctrl && key.toLowerCase() === "v" && clipboard)
      commit(pasteRung(document, clipboard, rung?.id));
    else if (key === "Delete") remove();
    else if (
      e.altKey &&
      selected?.kind === "node" &&
      (key === "ArrowLeft" || key === "ArrowRight")
    )
      commit(moveElement(document, selected.id, key === "ArrowLeft" ? -1 : 1));
    else if (e.altKey && rung && (key === "ArrowUp" || key === "ArrowDown"))
      commit(moveRung(document, rung.id, key === "ArrowUp" ? -1 : 1));
    else if (key.startsWith("Arrow")) navigate(key);
    else if (key === "Escape") setSelected(null);
    else return;
    e.preventDefault();
  };

  return (
    <div
      className="lad"
      data-testid="lad-editor"
      ref={root}
      onKeyDown={onKeyDown}
    >
      <Palette
        t={t}
        where={where}
        setWhere={setWhere}
        canInsert={!!selected || !!rung}
        insert={insert}
        addCoil={(kind) => {
          const target = rung ?? document.rungs.at(-1);
          if (!target) return;
          const next = addCoil(document, target.id, kind);
          commit(next);
          setSelected({ kind: "coil", id: newestElement(next) });
        }}
        remove={selected ? remove : null}
        move={
          selected?.kind === "node"
            ? (delta) => commit(moveElement(document, selected.id, delta))
            : null
        }
        undo={history.past.length ? undo : null}
        redo={history.future.length ? redo : null}
        copy={rung ? () => setClipboard(rung) : null}
        paste={
          clipboard
            ? () => commit(pasteRung(document, clipboard, rung?.id))
            : null
        }
        zoom={zoom}
        setZoom={setZoom}
      />
      <Status t={t} lab={lab} observed={observed} fault={fault} />
      <div
        className="lad-rungs"
        role="list"
        aria-label={t("lad.rungs")}
        style={{ zoom }}
      >
        {document.rungs.map((r, index) => (
          <RungView
            key={r.id}
            t={t}
            rung={r}
            number={index + 1}
            count={document.rungs.length}
            selected={selected}
            select={setSelected}
            observed={observed}
            errors={errors}
            change={(next) => commit(next(document))}
          />
        ))}
        <button
          className="btn"
          onClick={() => {
            const next = addRung(document, rung?.id);
            commit(next);
            const added = next.rungs.find(
              (x) => !document.rungs.some((y) => y.id === x.id),
            );
            const first =
              added?.network.type === "series" ? added.network.items[0] : null;
            if (first) setSelected({ kind: "node", id: first.id });
          }}
        >
          {t("lad.addRung")}
        </button>
      </div>
      <Properties
        t={t}
        document={document}
        selected={selected}
        commit={commit}
      />
      <Variables t={t} document={document} commit={commit} />
    </div>
  );
}

function kindOf(document: LadDocument, id: number): Selected {
  if (findCoil(document, id)) return { kind: "coil", id };
  if (document.rungs.some((r) => r.id === id)) return { kind: "rung", id };
  return { kind: "node", id };
}

function Status({
  t,
  lab,
  observed,
  fault,
}: {
  t: T;
  lab: Lab;
  observed: Observed;
  fault: DiagnosticDto | null;
}) {
  const text = !observed.online
    ? t("lad.offline")
    : observed.scanned
      ? t("lad.online", { cycle: observed.cycle ?? "" })
      : t("lad.notScanning", { mode: lab.frame?.plc.mode ?? "" });
  return (
    <>
      <p className="muted lad-status" role="status">
        {text}
      </p>
      {fault && observed.online && (
        <p className="lad-fault" role="alert">
          {t("lad.fault", { message: fault.message })}
        </p>
      )}
    </>
  );
}

function Palette({
  t,
  where,
  setWhere,
  canInsert,
  insert,
  addCoil,
  remove,
  move,
  undo,
  redo,
  copy,
  paste,
  zoom,
  setZoom,
}: {
  t: T;
  where: Where;
  setWhere: (w: Where) => void;
  canInsert: boolean;
  insert: (spec: ElementSpec) => void;
  addCoil: (kind: CoilKind) => void;
  remove: (() => void) | null;
  move: ((delta: -1 | 1) => void) | null;
  undo: (() => void) | null;
  redo: (() => void) | null;
  copy: (() => void) | null;
  paste: (() => void) | null;
  zoom: number;
  setZoom: (z: number) => void;
}) {
  const zi = ZOOMS.indexOf(zoom);
  return (
    <div className="lad-toolbar" role="toolbar" aria-label={t("lad.tools")}>
      <div className="segmented" role="group" aria-label={t("lad.where")}>
        {(["before", "after", "parallel"] as const).map((w) => (
          <button
            key={w}
            aria-pressed={where === w}
            onClick={() => setWhere(w)}
          >
            {t(`lad.where.${w}`)}
          </button>
        ))}
      </div>
      <div
        className="row lad-palette"
        role="group"
        aria-label={t("lad.palette")}
      >
        {CONTACTS.map((c) => (
          <button
            key={c.kind}
            className="btn btn-sm mono"
            disabled={!canInsert}
            aria-label={t("lad.insert", {
              element: t(c.label, { variable: "" }).trim(),
            })}
            title={t(c.label, { variable: "" }).trim()}
            onClick={() => insert({ type: "contact", contact: c.kind })}
          >
            {c.symbol}
          </button>
        ))}
        <button
          className="btn btn-sm mono"
          disabled={!canInsert}
          aria-label={t("lad.insert", { element: t("lad.compare") })}
          title={t("lad.compare")}
          onClick={() => insert({ type: "compare", op: "ge" })}
        >
          CMP
        </button>
        {BLOCKS.map((b) => (
          <button
            key={b}
            className="btn btn-sm mono"
            disabled={!canInsert}
            aria-label={t("lad.insert", { element: t(`lad.block.${b}`) })}
            title={t(`lad.block.${b}`)}
            onClick={() => insert({ type: "block", block: b })}
          >
            {b}
          </button>
        ))}
        {COILS.map((c) => (
          <button
            key={c.kind}
            className="btn btn-sm mono"
            disabled={!canInsert}
            aria-label={t(c.label)}
            title={t(c.label)}
            onClick={() => addCoil(c.kind)}
          >
            {c.symbol}
          </button>
        ))}
      </div>
      <div className="row" style={{ gap: 4 }}>
        <button
          className="btn btn-sm"
          disabled={!move}
          aria-label={t("lad.moveLeft")}
          onClick={() => move?.(-1)}
        >
          ◀
        </button>
        <button
          className="btn btn-sm"
          disabled={!move}
          aria-label={t("lad.moveRight")}
          onClick={() => move?.(1)}
        >
          ▶
        </button>
        <button
          className="btn btn-sm btn-ghost"
          disabled={!remove}
          onClick={() => remove?.()}
        >
          {t("lad.delete")}
        </button>
        <button
          className="btn btn-sm"
          disabled={!undo}
          onClick={() => undo?.()}
        >
          {t("lad.undo")}
        </button>
        <button
          className="btn btn-sm"
          disabled={!redo}
          onClick={() => redo?.()}
        >
          {t("lad.redo")}
        </button>
        <button
          className="btn btn-sm"
          disabled={!copy}
          onClick={() => copy?.()}
        >
          {t("lad.copyRung")}
        </button>
        <button
          className="btn btn-sm"
          disabled={!paste}
          onClick={() => paste?.()}
        >
          {t("lad.pasteRung")}
        </button>
        <span className="lad-zoom row" role="group" aria-label={t("lad.zoom")}>
          <button
            className="btn btn-sm"
            disabled={zi <= 0}
            aria-label={t("lad.zoomOut")}
            onClick={() => setZoom(ZOOMS[zi - 1] ?? zoom)}
          >
            −
          </button>
          <span className="mono" style={{ fontSize: 11 }}>
            {Math.round(zoom * 100)} %
          </span>
          <button
            className="btn btn-sm"
            disabled={zi >= ZOOMS.length - 1}
            aria-label={t("lad.zoomIn")}
            onClick={() => setZoom(ZOOMS[zi + 1] ?? zoom)}
          >
            +
          </button>
        </span>
      </div>
    </div>
  );
}

function RungView({
  t,
  rung,
  number,
  count,
  selected,
  select,
  observed,
  errors,
  change,
}: {
  t: T;
  rung: Rung;
  number: number;
  count: number;
  selected: Selected;
  select: (s: Selected) => void;
  observed: Observed;
  errors: Map<number, string[]>;
  change: (edit: (d: LadDocument) => LadDocument) => void;
}) {
  const power = observed.power(rung.network.id);
  const rungSelected = selected?.kind === "rung" && selected.id === rung.id;
  const rungErrors = errors.get(rung.id);
  return (
    <section
      className={`lad-rung${rungSelected ? " selected" : ""}${rungErrors ? " error" : ""}`}
      role="listitem"
      aria-label={t("lad.rung", { number })}
    >
      <header className="row" style={{ gap: 6 }}>
        <button
          className="btn btn-sm btn-ghost lad-rung-handle"
          data-element={rung.id}
          aria-pressed={rungSelected}
          title={rungErrors?.join("\n")}
          onClick={() => select({ kind: "rung", id: rung.id })}
        >
          <strong>{t("lad.rung", { number })}</strong>
        </button>
        <input
          className="input lad-comment"
          aria-label={t("lad.comment", { number })}
          placeholder={t("lad.commentPlaceholder")}
          value={rung.comment ?? ""}
          onChange={(e) =>
            change((d) => setRungComment(d, rung.id, e.target.value))
          }
        />
        <span className="spacer" />
        <button
          className="btn btn-sm"
          disabled={number === 1}
          aria-label={t("lad.rungUp", { number })}
          onClick={() => change((d) => moveRung(d, rung.id, -1))}
        >
          ↑
        </button>
        <button
          className="btn btn-sm"
          disabled={number === count}
          aria-label={t("lad.rungDown", { number })}
          onClick={() => change((d) => moveRung(d, rung.id, 1))}
        >
          ↓
        </button>
        <button
          className="btn btn-sm"
          onClick={() => change((d) => duplicateRung(d, rung.id))}
        >
          {t("lad.duplicateRung")}
        </button>
        <button
          className="btn btn-sm btn-ghost"
          onClick={() => change((d) => removeRung(d, rung.id))}
        >
          {t("lad.removeRung")}
        </button>
      </header>
      {rungErrors && (
        <p className="lad-error-text" role="alert">
          {rungErrors.join(" ")}
        </p>
      )}
      <div className="lad-line">
        <span
          className={`lad-rail${observed.online ? " powered" : ""}`}
          aria-hidden="true"
        />
        <span
          className={`lad-seg${observed.online ? " powered" : ""}`}
          aria-hidden="true"
        />
        <div className="lad-network">
          <NetworkView
            t={t}
            node={rung.network}
            selected={selected}
            select={select}
            observed={observed}
            errors={errors}
          />
        </div>
        <span
          className={`lad-wire grow${power ? " powered" : ""}`}
          aria-hidden="true"
        />
        <div className="lad-coils">
          {rung.coils.map((coil) => {
            const v = observed.value(coil.variable);
            const isSelected =
              selected?.kind === "coil" && selected.id === coil.id;
            const problems = errors.get(coil.id);
            const name = coil.variable || t("lad.noVariable");
            const label = t(
              coil.kind === "set"
                ? "lad.setCoilNamed"
                : coil.kind === "reset"
                  ? "lad.resetCoilNamed"
                  : "lad.coilNamed",
              { variable: name },
            );
            return (
              <button
                key={coil.id}
                data-element={coil.id}
                className={`lad-coil${power ? " powered" : ""}${on(v) ? " on" : ""}${isSelected ? " selected" : ""}${problems ? " error" : ""}`}
                aria-pressed={isSelected}
                aria-invalid={problems ? true : undefined}
                aria-label={describe(t, label, v, power)}
                title={problems?.join("\n")}
                onClick={() => select({ kind: "coil", id: coil.id })}
              >
                <span className="lad-symbol">{coilSymbol(coil.kind)}</span>
                <span className="lad-name">{name}</span>
                {v && <span className="lad-value">{formatValue(v)}</span>}
              </button>
            );
          })}
        </div>
        <span className="lad-rail" aria-hidden="true" />
      </div>
    </section>
  );
}

/** "Normally open contact X: TRUE, power flows" — values and power from the runtime. */
function describe(
  t: T,
  label: string,
  value: SignalValueDto | null,
  power: boolean | null,
): string {
  const parts = [value ? `${label}: ${formatValue(value)}` : label];
  if (power !== null) parts.push(t(power ? "lad.powered" : "lad.unpowered"));
  return parts.join(", ");
}

function NetworkView({
  t,
  node,
  selected,
  select,
  observed,
  errors,
}: {
  t: T;
  node: LadNode;
  selected: Selected;
  select: (s: Selected) => void;
  observed: Observed;
  errors: Map<number, string[]>;
}): ReactNode {
  const isSelected = selected?.kind === "node" && selected.id === node.id;
  const power = observed.power(node.id);
  const problems = errors.get(node.id);
  const common = {
    "data-element": node.id,
    "aria-pressed": isSelected,
    "aria-invalid": problems ? true : undefined,
    title: problems?.join("\n"),
    onClick: () => select({ kind: "node", id: node.id }),
  };
  const cls = (base: string, active = false) =>
    `${base}${power ? " powered" : ""}${active ? " on" : ""}${isSelected ? " selected" : ""}${problems ? " error" : ""}`;
  switch (node.type) {
    case "contact": {
      const v = observed.value(node.variable);
      const kind = CONTACTS.find((c) => c.kind === node.contact);
      const name = node.variable || t("lad.noVariable");
      return (
        <button
          {...common}
          className={cls("lad-contact", on(v))}
          aria-label={describe(
            t,
            t(kind?.label ?? "lad.noContact", { variable: name }),
            v,
            power,
          )}
        >
          <span className="lad-symbol">{symbolOf(node.contact)}</span>
          <span className="lad-name">{name}</span>
          {v && <span className="lad-value">{formatValue(v)}</span>}
        </button>
      );
    }
    case "compare": {
      const symbol = OPS.find((o) => o.op === node.op)?.symbol ?? node.op;
      const side = (o: Operand) => {
        if ("constant" in o) return o.constant;
        const v = observed.value(o.variable);
        return v ? `${o.variable} (${formatValue(v)})` : o.variable || "?";
      };
      const text = `${side(node.left)} ${symbol} ${side(node.right)}`;
      return (
        <button
          {...common}
          className={cls("lad-compare")}
          aria-label={describe(
            t,
            t("lad.compareNamed", { comparison: text }),
            null,
            power,
          )}
        >
          <span className="lad-symbol">CMP {symbol}</span>
          <span className="lad-name">{text}</span>
        </button>
      );
    }
    case "block": {
      const timer = isTimer(node.block);
      const out = observed.value(`${node.instance}.${timer ? "ET" : "CV"}`);
      const q = observed.value(`${node.instance}.Q`);
      const preset = operandText(node.preset) || "?";
      return (
        <button
          {...common}
          className={cls("lad-block", on(q))}
          aria-label={describe(
            t,
            t("lad.blockNamed", {
              block: node.block,
              instance: node.instance,
              preset,
              value: out ? formatValue(out) : "—",
            }),
            null,
            power,
          )}
        >
          <span className="lad-symbol">{node.block}</span>
          <span className="lad-name">{node.instance}</span>
          <span className="lad-pin">
            {timer ? "PT" : "PV"} {preset}
          </span>
          {node.block === "CTU" && node.reset && (
            <span className="lad-pin">R {operandText(node.reset)}</span>
          )}
          {node.block === "CTD" && node.reset && (
            <span className="lad-pin">LD {operandText(node.reset)}</span>
          )}
          <span className="lad-value">
            {timer ? "ET" : "CV"} {out ? formatValue(out) : "—"}
          </span>
        </button>
      );
    }
    case "series":
      if (!node.items.length)
        return (
          <button
            {...common}
            className={cls("lad-empty")}
            aria-label={t("lad.empty")}
          >
            {t("lad.empty")}
          </button>
        );
      return (
        <div className="lad-series">
          {node.items.map((item, i) => (
            <span key={item.id} className="lad-cell">
              {i > 0 && (
                <span
                  className={`lad-seg${observed.power(node.items[i - 1]?.id ?? 0) ? " powered" : ""}`}
                  aria-hidden="true"
                />
              )}
              <NetworkView
                t={t}
                node={item}
                selected={selected}
                select={select}
                observed={observed}
                errors={errors}
              />
            </span>
          ))}
        </div>
      );
    case "parallel":
      return (
        <div className={cls("lad-parallel")}>
          <button
            {...common}
            className="lad-block-handle"
            aria-label={t("lad.parallelBlock", { count: node.branches.length })}
          >
            ∥
          </button>
          <div className="lad-branches">
            {node.branches.map((branch) => (
              <div key={branch.id} className="lad-branch">
                <NetworkView
                  t={t}
                  node={branch}
                  selected={selected}
                  select={select}
                  observed={observed}
                  errors={errors}
                />
                <span
                  className={`lad-seg grow${observed.power(branch.id) ? " powered" : ""}`}
                  aria-hidden="true"
                />
              </div>
            ))}
          </div>
        </div>
      );
  }
}

/** The settings of the selected element: variables, kinds, operands, block pins. */
function Properties({
  t,
  document,
  selected,
  commit,
}: {
  t: T;
  document: LadDocument;
  selected: Selected;
  commit: (d: LadDocument) => void;
}) {
  if (!selected) return null;
  const vars = (type: LadType, writable = false) =>
    document.variables
      .filter((v) => v.type === type && (!writable || v.direction !== "input"))
      .map((v) => v.name);
  if (selected.kind === "coil") {
    const coil = findCoil(document, selected.id);
    if (!coil) return null;
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <VariableSelect
          t={t}
          label={t("lad.coilVariable")}
          value={coil.variable}
          options={vars("BOOL", true)}
          onChange={(variable) =>
            commit(setCoil(document, coil.id, { variable }))
          }
        />
        <label className="field">
          {t("lad.coilKind")}
          <select
            className="input"
            value={coil.kind ?? "output"}
            onChange={(e) =>
              commit(
                setCoil(document, coil.id, {
                  kind: e.target.value as CoilKind,
                }),
              )
            }
          >
            {COILS.map((c) => (
              <option key={c.kind} value={c.kind}>
                {c.symbol} {t(`lad.coil.${c.kind}`)}
              </option>
            ))}
          </select>
        </label>
      </section>
    );
  }
  if (selected.kind === "rung") return null;
  const node = findNode(document, selected.id);
  if (!node) return null;
  const edit = (change: (n: LadNode) => LadNode) =>
    commit(updateElement(document, node.id, change));
  if (node.type === "contact")
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <VariableSelect
          t={t}
          label={t("lad.contactVariable")}
          value={node.variable}
          options={vars("BOOL")}
          onChange={(variable) =>
            edit((n) => (n.type === "contact" ? { ...n, variable } : n))
          }
        />
        <label className="field">
          {t("lad.contactKind")}
          <select
            className="input"
            value={node.contact}
            onChange={(e) =>
              edit((n) =>
                n.type === "contact"
                  ? { ...n, contact: e.target.value as ContactKind }
                  : n,
              )
            }
          >
            {CONTACTS.map((c) => (
              <option key={c.kind} value={c.kind}>
                {c.symbol} {t(c.label, { variable: "" }).trim()}
              </option>
            ))}
          </select>
        </label>
      </section>
    );
  if (node.type === "compare") {
    const all = document.variables.map((v) => v.name);
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <OperandEditor
          key={`l${node.id}${JSON.stringify(node.left)}`}
          t={t}
          label={t("lad.left")}
          value={node.left}
          options={all}
          onChange={(left) =>
            edit((n) => (n.type === "compare" ? { ...n, left } : n))
          }
        />
        <label className="field">
          {t("lad.comparison")}
          <select
            className="input"
            value={node.op}
            onChange={(e) =>
              edit((n) =>
                n.type === "compare"
                  ? { ...n, op: e.target.value as CompareOp }
                  : n,
              )
            }
          >
            {OPS.map((o) => (
              <option key={o.op} value={o.op}>
                {o.op.toUpperCase()} {o.symbol}
              </option>
            ))}
          </select>
        </label>
        <OperandEditor
          key={`r${node.id}${JSON.stringify(node.right)}`}
          t={t}
          label={t("lad.right")}
          value={node.right}
          options={all}
          onChange={(right) =>
            edit((n) => (n.type === "compare" ? { ...n, right } : n))
          }
        />
      </section>
    );
  }
  if (node.type === "block") {
    const timer = isTimer(node.block);
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <InstanceName
          key={`i${node.id}${node.instance}`}
          t={t}
          value={node.instance}
          onChange={(instance) =>
            edit((n) => (n.type === "block" ? { ...n, instance } : n))
          }
        />
        <OperandEditor
          key={`p${node.id}${JSON.stringify(node.preset)}`}
          t={t}
          label={t(timer ? "lad.presetPT" : "lad.presetPV")}
          value={node.preset ?? { constant: "" }}
          options={vars(timer ? "TIME" : "INT")}
          onChange={(preset) =>
            edit((n) => (n.type === "block" ? { ...n, preset } : n))
          }
        />
        {!timer && (
          <OperandEditor
            key={`z${node.id}${JSON.stringify(node.reset)}`}
            t={t}
            label={t(node.block === "CTD" ? "lad.loadLD" : "lad.resetR")}
            value={node.reset ?? { constant: "FALSE" }}
            options={vars("BOOL")}
            onChange={(reset) =>
              commit(
                setBlockPin(
                  document,
                  node.id,
                  "reset",
                  "constant" in reset &&
                    reset.constant.toUpperCase() === "FALSE"
                    ? null
                    : reset,
                ),
              )
            }
          />
        )}
        <VariableSelect
          t={t}
          label={t(timer ? "lad.outputET" : "lad.outputCV")}
          value={node.output ?? ""}
          options={vars(timer ? "TIME" : "INT", true)}
          none={t("lad.noOutput")}
          onChange={(output) =>
            commit(setBlockPin(document, node.id, "output", output))
          }
        />
      </section>
    );
  }
  return null;
}

function VariableSelect({
  t,
  label,
  value,
  options,
  none,
  onChange,
}: {
  t: T;
  label: string;
  value: string;
  options: string[];
  none?: string;
  onChange: (v: string) => void;
}) {
  const list =
    value && !options.includes(value) ? [value, ...options] : options;
  return (
    <label className="field lad-assign">
      {label}
      <select
        className="input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">{none ?? t("lad.noVariable")}</option>
        {list.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
    </label>
  );
}

/** A variable or a literal; literals are checked by the compiler in Rust. */
function OperandEditor({
  t,
  label,
  value,
  options,
  onChange,
}: {
  t: T;
  label: string;
  value: Operand;
  options: string[];
  onChange: (o: Operand) => void;
}) {
  const isVariable = "variable" in value;
  const [text, setText] = useState(isVariable ? "" : value.constant);
  return (
    <fieldset className="lad-operand">
      <legend>{label}</legend>
      <select
        className="input"
        aria-label={t("lad.operandKind", { operand: label })}
        value={isVariable ? "variable" : "constant"}
        onChange={(e) =>
          onChange(
            e.target.value === "variable"
              ? { variable: options[0] ?? "" }
              : { constant: text || "0" },
          )
        }
      >
        <option value="variable">{t("lad.operand.variable")}</option>
        <option value="constant">{t("lad.operand.constant")}</option>
      </select>
      {isVariable ? (
        <select
          className="input"
          aria-label={t("lad.operandVariable", { operand: label })}
          value={value.variable}
          onChange={(e) => onChange({ variable: e.target.value })}
        >
          <option value="">{t("lad.noVariable")}</option>
          {options.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      ) : (
        <input
          className="input"
          aria-label={t("lad.operandConstant", { operand: label })}
          value={text}
          size={9}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => text !== value.constant && onChange({ constant: text })}
          onKeyDown={(e) => {
            if (e.key === "Enter") onChange({ constant: text });
          }}
        />
      )}
    </fieldset>
  );
}

function InstanceName({
  t,
  value,
  onChange,
}: {
  t: T;
  value: string;
  onChange: (v: string) => void;
}) {
  const [text, setText] = useState(value);
  return (
    <label className="field">
      {t("lad.instance")}
      <input
        className="input mono"
        value={text}
        size={10}
        maxLength={40}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => text.trim() && text !== value && onChange(text.trim())}
        onKeyDown={(e) => {
          if (e.key === "Enter" && text.trim()) onChange(text.trim());
        }}
      />
    </label>
  );
}

const DIRECTIONS: Direction[] = ["input", "output", "local"];

function Variables({
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

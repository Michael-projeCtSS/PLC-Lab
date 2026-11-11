/**
 * One rung drawn from its layout (ADR-0052): rails, wires and junctions in SVG, elements
 * as buttons at their layout boxes, box pins as editable fields on the diagram, insertion
 * points on the series wires, and the drop preview while something is dragged. Wire and
 * element power comes only from the runtime's observation; the preview shows topology,
 * never power.
 */
import type { CSSProperties, ReactNode } from "react";
import { formatValue } from "../../format";
import { isTimer, type LadNode, type Rung } from "../ladModel";
import {
  CONTACTS,
  coilSymbol,
  describe,
  elementName,
  fieldLabel,
  isOn,
  opSymbol,
  operandText,
  type Observed,
  type Selected,
  type T,
} from "./common";
import type { Field } from "./complete";
import {
  G,
  fieldBox,
  targetBox,
  type Box,
  type Feed,
  type RungLayout,
} from "./layout";
import type { Source, Target } from "./placement";

/** What the canvas shows while something is dragged over this rung. */
export type Preview =
  | { kind: "target"; target: Target; copy: boolean }
  | { kind: "invalid"; x: number; y: number };

const at = (b: Box): CSSProperties => ({
  left: b.x,
  top: b.y,
  width: b.w,
  height: b.h,
});

export function RungCanvas({
  t,
  rung,
  layout,
  zoom,
  selected,
  select,
  observed,
  errors,
  preview,
  editor,
  register,
  grab,
  edit,
  menu,
}: {
  t: T;
  rung: Rung;
  layout: RungLayout;
  zoom: number;
  selected: Selected;
  select: (s: Selected) => void;
  observed: Observed;
  errors: Map<number, string[]>;
  preview: Preview | null;
  /** The inline editor, already positioned by the caller, when it edits this rung. */
  editor: ReactNode;
  register: (rung: number, el: HTMLDivElement | null) => void;
  grab: (e: React.PointerEvent, source: Source, label: string) => void;
  edit: (element: number, field: Field) => void;
  menu: (e: React.MouseEvent, id: number) => void;
}) {
  const scanned = observed.online && observed.scanned;
  const power = (f: Feed) =>
    f === "rail" ? (scanned ? true : null) : observed.power(f);
  const isSel = (kind: "node" | "coil", id: number) =>
    selected?.kind === kind && selected.id === id;
  const common = (id: number, kind: "node" | "coil", label: string) => {
    const problems = errors.get(id);
    return {
      "data-element": id,
      "aria-pressed": isSel(kind, id),
      "aria-invalid": problems ? true : undefined,
      title: problems?.join("\n"),
      onClick: () => select({ kind, id }),
      onPointerDown: (e: React.PointerEvent) => {
        if (e.button !== 0) return;
        grab(e, { kind: "move", id, copy: e.ctrlKey }, label);
      },
      onContextMenu: (e: React.MouseEvent) => {
        e.preventDefault();
        select({ kind, id });
        menu(e, id);
      },
    };
  };
  const cls = (base: string, id: number, kind: "node" | "coil", on = false) =>
    `lad-el ${base}${observed.power(id) ? " powered" : ""}${on ? " on" : ""}${isSel(kind, id) ? " selected" : ""}${errors.get(id) ? " error" : ""}`;

  /** A box pin shown on the diagram and edited in place. */
  const pin = (n: LadNode, field: Field, value: string, b: Box) => (
    <button
      key={field}
      type="button"
      className={`lad-field${value ? "" : " empty"}`}
      data-field={`${n.id}:${field}`}
      style={at(b)}
      aria-label={t("lad.pinField", {
        field: fieldLabel(t, n, field, false),
        element: elementName(t, n),
        value: value || t("lad.noVariable"),
      })}
      onClick={(e) => {
        e.stopPropagation();
        select({ kind: "node", id: n.id });
        edit(n.id, field);
      }}
    >
      {/* A required pin shows "?", an optional one (R/LD, ET/CV) "—" while unset. */}
      {value || (field === "reset" || field === "output" ? "—" : "?")}
    </button>
  );

  const cells = layout.cells.map((c) => {
    if (c.kind === "empty")
      return (
        <button
          key={c.id}
          type="button"
          data-element={c.id}
          className={`lad-el lad-empty${selected?.kind === "point" && selected.after === null && selected.rung === rung.id ? " selected" : ""}`}
          style={at({ ...c.box, y: c.box.y + G.TOP, h: G.BODY })}
          aria-label={t("lad.empty")}
          onClick={() =>
            select({
              kind: "point",
              rung: rung.id,
              target: { zone: "start", rung: rung.id },
              after: null,
            })
          }
        >
          {t("lad.empty")}
        </button>
      );
    if (c.kind === "coil") {
      const coil = c.coil;
      const v = observed.value(coil.variable);
      const name = coil.variable || t("lad.noVariable");
      const label = t(
        coil.kind === "set"
          ? "lad.setCoilNamed"
          : coil.kind === "reset"
            ? "lad.resetCoilNamed"
            : "lad.coilNamed",
        { variable: name },
      );
      const p = observed.power(layout.root);
      return (
        <button
          key={coil.id}
          type="button"
          {...common(coil.id, "coil", label)}
          onDoubleClick={() => edit(coil.id, "variable")}
          className={`lad-el lad-coil${p ? " powered" : ""}${isOn(v) ? " on" : ""}${isSel("coil", coil.id) ? " selected" : ""}${errors.get(coil.id) ? " error" : ""}`}
          style={at({ ...c.box, y: c.box.y + G.TOP, h: G.BODY })}
          aria-label={describe(t, label, v, p)}
        >
          <span className="lad-name" data-field={`${coil.id}:variable`}>
            {name}
          </span>
          <span className="lad-symbol">{coilSymbol(coil.kind)}</span>
          {v && <span className="lad-value">{formatValue(v)}</span>}
        </button>
      );
    }
    const n = c.node;
    const p = observed.power(n.id);
    const body = {
      x: c.box.x,
      y: c.box.y + G.TOP,
      w: c.box.w,
      h: c.box.h - G.TOP - (G.ROW - G.TOP - G.BODY),
    };
    if (n.type === "contact") {
      const v = observed.value(n.variable);
      const kind = CONTACTS.find((k) => k.kind === n.contact);
      const name = n.variable || t("lad.noVariable");
      const label = t(kind?.label ?? "lad.noContact", { variable: name });
      return (
        <button
          key={n.id}
          type="button"
          {...common(n.id, "node", label)}
          onDoubleClick={() => edit(n.id, "variable")}
          className={cls("lad-contact", n.id, "node", isOn(v))}
          style={at(body)}
          aria-label={describe(t, label, v, p)}
        >
          <span className="lad-name" data-field={`${n.id}:variable`}>
            {name}
          </span>
          <span className="lad-symbol">{kind?.symbol ?? "| |"}</span>
          {v && <span className="lad-value">{formatValue(v)}</span>}
        </button>
      );
    }
    if (n.type === "compare") {
      const side = (o: typeof n.left) => {
        if ("constant" in o) return o.constant;
        const v = observed.value(o.variable);
        return v ? `${o.variable} (${formatValue(v)})` : o.variable || "?";
      };
      const text = `${side(n.left)} ${opSymbol(n.op)} ${side(n.right)}`;
      return [
        <button
          key={n.id}
          type="button"
          {...common(n.id, "node", elementName(t, n))}
          onDoubleClick={() => edit(n.id, "left")}
          className={cls("lad-compare", n.id, "node")}
          style={at(body)}
          aria-label={describe(
            t,
            t("lad.compareNamed", { comparison: text }),
            null,
            p,
          )}
        >
          <span className="lad-op">{opSymbol(n.op)}</span>
          <span className="lad-kind">CMP {n.op.toUpperCase()}</span>
        </button>,
        pin(
          n,
          "left",
          operandText(n.left),
          fieldBox(layout, n.id, "left") as Box,
        ),
        pin(
          n,
          "right",
          operandText(n.right),
          fieldBox(layout, n.id, "right") as Box,
        ),
      ];
    }
    if (n.type !== "block") return null;
    const timer = isTimer(n.block);
    const out = observed.value(`${n.instance}.${timer ? "ET" : "CV"}`);
    const q = observed.value(`${n.instance}.Q`);
    const preset = operandText(n.preset) || "?";
    return [
      <button
        key={n.id}
        type="button"
        {...common(n.id, "node", elementName(t, n))}
        onDoubleClick={() => edit(n.id, "instance")}
        className={cls("lad-block", n.id, "node", isOn(q))}
        style={at(body)}
        aria-label={describe(
          t,
          t("lad.blockNamed", {
            block: n.block,
            instance: n.instance,
            preset,
            value: out ? formatValue(out) : "—",
          }),
          null,
          p,
        )}
      >
        <span className="lad-kind">{n.block}</span>
        <span className="lad-pin-label in">
          {timer ? "IN" : n.block === "CTU" ? "CU" : "CD"}
        </span>
        <span className="lad-pin-label out">Q</span>
        <span className="lad-pin-label in second">{timer ? "PT" : "PV"}</span>
        <span className="lad-pin-label out second">{timer ? "ET" : "CV"}</span>
        {!timer && (
          <span className="lad-pin-label in third">
            {n.block === "CTD" ? "LD" : "R"}
          </span>
        )}
        <span className="lad-value">
          {timer ? "ET" : "CV"} {out ? formatValue(out) : "—"}
        </span>
      </button>,
      pin(n, "instance", n.instance, fieldBox(layout, n.id, "instance") as Box),
      pin(
        n,
        "preset",
        operandText(n.preset),
        fieldBox(layout, n.id, "preset") as Box,
      ),
      ...(timer
        ? []
        : [
            pin(
              n,
              "reset",
              operandText(n.reset),
              fieldBox(layout, n.id, "reset") as Box,
            ),
          ]),
      pin(n, "output", n.output ?? "", fieldBox(layout, n.id, "output") as Box),
    ];
  });

  // Short wires inside contacts and coils, on both sides of the symbol.
  const stubs = layout.cells.flatMap((c) => {
    if (c.kind === "empty") return [];
    if (c.kind === "node" && c.node.type !== "contact") return [];
    const y = c.box.y + G.LINE;
    const mid = c.box.x + c.box.w / 2;
    const inFeed: Feed = c.kind === "coil" ? layout.root : c.feed;
    const outFeed: Feed = c.kind === "coil" ? layout.root : c.node.id;
    return [
      { x1: c.box.x, x2: mid - 11, y, power: inFeed },
      { x1: mid + 11, x2: c.box.x + c.box.w, y, power: outFeed },
    ];
  });

  return (
    <div
      className="lad-canvas"
      ref={(el) => register(rung.id, el)}
      data-rung={rung.id}
      style={{ width: layout.width * zoom, height: layout.height * zoom }}
    >
      <div
        className="lad-plane"
        style={{
          width: layout.width,
          height: layout.height,
          transform: `scale(${zoom})`,
        }}
      >
        <svg
          className="lad-wires"
          width={layout.width}
          height={layout.height}
          aria-hidden="true"
        >
          <line
            className={`lad-rail-line${scanned ? " on" : ""}`}
            x1={2}
            x2={2}
            y1={0}
            y2={layout.height - G.BOTTOM + 6}
          />
          <line
            className="lad-rail-line"
            x1={layout.railRight}
            x2={layout.railRight}
            y1={0}
            y2={layout.height - G.BOTTOM + 6}
          />
          {layout.wires.map((w, i) => (
            <line
              key={i}
              className={`lad-wire-line${power(w.power) ? " on" : ""}`}
              x1={w.x1}
              y1={w.y1}
              x2={w.x2}
              y2={w.y2}
            />
          ))}
          {stubs.map((s, i) => (
            <line
              key={`s${i}`}
              className={`lad-wire-line${power(s.power) ? " on" : ""}`}
              x1={s.x1}
              y1={s.y}
              x2={s.x2}
              y2={s.y}
            />
          ))}
          {preview && <PreviewShape layout={layout} preview={preview} />}
        </svg>
        {layout.groups.map((g) => {
          const isSelected = selected?.kind === "node" && selected.id === g.id;
          return (
            <button
              key={g.id}
              type="button"
              data-element={g.id}
              className={`lad-group-handle${isSelected ? " selected" : ""}${errors.get(g.id) ? " error" : ""}`}
              style={{ left: g.box.x - 9, top: g.box.y + G.LINE - 9 }}
              aria-pressed={isSelected}
              aria-label={t("lad.parallelBlock", { count: g.count })}
              title={errors.get(g.id)?.join("\n")}
              onClick={() => select({ kind: "node", id: g.id })}
              onPointerDown={(e) => {
                if (e.button !== 0) return;
                grab(
                  e,
                  { kind: "move", id: g.id, copy: e.ctrlKey },
                  t("lad.parallelBlock", { count: g.count }),
                );
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                select({ kind: "node", id: g.id });
                menu(e, g.id);
              }}
            >
              ∥
            </button>
          );
        })}
        {cells}
        {layout.points.map((p, i) => {
          const isSelected =
            selected?.kind === "point" &&
            selected.rung === rung.id &&
            JSON.stringify(selected.target) === JSON.stringify(p.target);
          // Named after the element the point follows (or precedes, at a series start).
          const neighbour =
            p.target.zone === "before" || p.target.zone === "after"
              ? p.target.id
              : null;
          const cell = layout.cells.find(
            (c) => c.kind === "node" && c.node.id === neighbour,
          );
          const name =
            cell?.kind === "node"
              ? elementName(t, cell.node)
              : t("lad.parallelBlock", {
                  count:
                    layout.groups.find((g) => g.id === neighbour)?.count ?? 2,
                });
          const label =
            p.target.zone === "start"
              ? t("lad.pointStart")
              : p.target.zone === "before"
                ? t("lad.pointBefore", { name })
                : t("lad.point", { name });
          return (
            <div
              key={`p${i}`}
              role="button"
              tabIndex={-1}
              className={`lad-point${isSelected ? " selected" : ""}`}
              style={at(p.box)}
              aria-pressed={isSelected}
              aria-label={label}
              onClick={() =>
                select({
                  kind: "point",
                  rung: rung.id,
                  target: p.target,
                  after: p.after,
                })
              }
            />
          );
        })}
        {editor}
      </div>
    </div>
  );
}

/** The insertion preview: where the element will sit and how it will be wired. */
function PreviewShape({
  layout,
  preview,
}: {
  layout: RungLayout;
  preview: Preview;
}) {
  if (preview.kind === "invalid")
    return (
      <circle
        className="lad-drop-invalid"
        cx={preview.x}
        cy={preview.y}
        r={14}
      />
    );
  const { target } = preview;
  const b = targetBox(layout, target);
  if (!b) return null;
  const cls = `lad-drop${preview.copy ? " copy" : ""}`;
  if (target.zone === "coil")
    return (
      <line
        className={cls}
        x1={layout.coilX - 8}
        x2={layout.coilX + G.COIL_W + 8}
        y1={b.y}
        y2={b.y}
      />
    );
  if (target.zone === "start")
    return (
      <rect
        className={cls}
        x={b.x}
        y={b.y + G.TOP}
        width={b.w}
        height={G.BODY}
        rx={6}
      />
    );
  const line = b.y + G.LINE;
  if (target.zone === "before" || target.zone === "after") {
    const x =
      target.zone === "before" ? b.x - G.GAP / 2 : b.x + b.w + G.GAP / 2;
    return (
      <g className={cls}>
        <line x1={x} x2={x} y1={line - 20} y2={line + 20} />
        <rect
          x={x - 22}
          y={line - 33}
          width={44}
          height={14}
          rx={3}
          className="lad-drop-tag"
        />
        <text x={x} y={line - 22}>
          AND
        </text>
      </g>
    );
  }
  // An OR branch: from the left of the target down, across, and back up on its right.
  const branch = b.y + b.h + G.LINE;
  const x1 = b.x - G.JX / 2;
  const x2 = b.x + b.w + G.JX / 2;
  return (
    <g className={cls}>
      <polyline
        points={`${x1},${line} ${x1},${branch} ${x2},${branch} ${x2},${line}`}
        fill="none"
      />
      <rect
        x={b.x + 8}
        y={branch - 16}
        width={Math.min(G.CONTACT_W, b.w) - 16}
        height={32}
        rx={5}
        className="lad-drop-ghost"
      />
      <rect
        x={x1 - 2}
        y={branch + 10}
        width={30}
        height={14}
        rx={3}
        className="lad-drop-tag"
      />
      <text x={x1 + 13} y={branch + 21}>
        OR
      </text>
    </g>
  );
}

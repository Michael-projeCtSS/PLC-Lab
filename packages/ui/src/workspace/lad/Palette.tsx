/**
 * The ladder toolbar (ADR-0052): palette elements are dragged onto a rung — beside an
 * element for AND, below it for OR — or clicked to insert at the selection with the
 * explicit position, which stays as the keyboard and accessibility fallback. Editing
 * commands, zoom and the shortcut reference sit next to it.
 */
import type { TranslationKey } from "../../locale/en";
import { BLOCKS, COILS, CONTACTS, ZOOMS, type T } from "./common";
import type { PaletteItem, Source } from "./placement";

export type Where = "before" | "after" | "parallel";

const SHORTCUTS: [string, TranslationKey][] = [
  ["F9", "lad.key.f9"],
  ["F10", "lad.key.f10"],
  ["Shift+F9", "lad.key.shiftF9"],
  ["F8", "lad.key.f8"],
  ["A…Z", "lad.key.type"],
  ["Enter / F2", "lad.key.enter"],
  ["Ctrl+Space", "lad.key.space"],
  ["Tab", "lad.key.tab"],
  ["Esc", "lad.key.esc"],
  ["Delete", "lad.key.delete"],
  ["Ctrl+C / Ctrl+V", "lad.key.copy"],
  ["Ctrl+Z / Ctrl+Y", "lad.key.undo"],
  ["← → ↑ ↓", "lad.key.arrows"],
  ["Alt+← →, Alt+↑ ↓", "lad.key.move"],
  ["Shift+F10", "lad.key.menu"],
  ["Ctrl + drag", "lad.key.ctrlDrag"],
];

export function Palette({
  t,
  where,
  setWhere,
  canInsert,
  insert,
  grab,
  justDropped,
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
  insert: (item: PaletteItem) => void;
  grab: (e: React.PointerEvent, source: Source, label: string) => void;
  justDropped: () => boolean;
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
  /** A palette element: drag it onto a rung, or click to insert at the selection. */
  const item = (
    key: string,
    element: PaletteItem,
    symbol: string,
    name: string,
    label: string,
  ) => (
    <button
      key={key}
      type="button"
      className="btn btn-sm mono lad-tool"
      // Dragging works without a selection; clicking needs one.
      aria-disabled={!canInsert}
      aria-label={label}
      title={`${name} — ${t("lad.dragHint")}`}
      onPointerDown={(e) => {
        if (e.button === 0)
          grab(e, { kind: "new", item: element }, `${symbol} ${name}`);
      }}
      onClick={() => {
        if (justDropped() || !canInsert) return;
        insert(element);
      }}
    >
      {symbol}
    </button>
  );
  return (
    <div className="lad-toolbar" role="toolbar" aria-label={t("lad.tools")}>
      <div
        className="row lad-palette"
        role="group"
        aria-label={t("lad.palette")}
      >
        {CONTACTS.map((c) => {
          const name = t(c.label, { variable: "" }).trim();
          return item(
            c.kind,
            { type: "contact", contact: c.kind },
            c.symbol,
            name,
            t("lad.insert", { element: name }),
          );
        })}
        {item(
          "cmp",
          { type: "compare", op: "ge" },
          "CMP",
          t("lad.compare"),
          t("lad.insert", { element: t("lad.compare") }),
        )}
        {BLOCKS.map((b) =>
          item(
            b,
            { type: "block", block: b },
            b,
            t(`lad.block.${b}`),
            t("lad.insert", { element: t(`lad.block.${b}`) }),
          ),
        )}
        {COILS.map((c) =>
          item(
            c.kind,
            { type: "coil", kind: c.kind },
            c.symbol,
            t(c.label),
            t(c.label),
          ),
        )}
      </div>
      <div className="segmented" role="group" aria-label={t("lad.where")}>
        {(["before", "after", "parallel"] as const).map((w) => (
          <button
            key={w}
            type="button"
            aria-pressed={where === w}
            onClick={() => setWhere(w)}
          >
            {t(`lad.where.${w}`)}
          </button>
        ))}
      </div>
      <div className="row" style={{ gap: 4 }}>
        <button
          type="button"
          className="btn btn-sm"
          disabled={!move}
          aria-label={t("lad.moveLeft")}
          onClick={() => move?.(-1)}
        >
          ◀
        </button>
        <button
          type="button"
          className="btn btn-sm"
          disabled={!move}
          aria-label={t("lad.moveRight")}
          onClick={() => move?.(1)}
        >
          ▶
        </button>
        <button
          type="button"
          className="btn btn-sm btn-ghost"
          disabled={!remove}
          onClick={() => remove?.()}
        >
          {t("lad.delete")}
        </button>
        <button
          type="button"
          className="btn btn-sm"
          disabled={!undo}
          onClick={() => undo?.()}
        >
          {t("lad.undo")}
        </button>
        <button
          type="button"
          className="btn btn-sm"
          disabled={!redo}
          onClick={() => redo?.()}
        >
          {t("lad.redo")}
        </button>
        <button
          type="button"
          className="btn btn-sm"
          disabled={!copy}
          onClick={() => copy?.()}
        >
          {t("lad.copyRung")}
        </button>
        <button
          type="button"
          className="btn btn-sm"
          disabled={!paste}
          onClick={() => paste?.()}
        >
          {t("lad.pasteRung")}
        </button>
        <span className="lad-zoom row" role="group" aria-label={t("lad.zoom")}>
          <button
            type="button"
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
            type="button"
            className="btn btn-sm"
            disabled={zi >= ZOOMS.length - 1}
            aria-label={t("lad.zoomIn")}
            onClick={() => setZoom(ZOOMS[zi + 1] ?? zoom)}
          >
            +
          </button>
        </span>
      </div>
      <details className="lad-shortcuts">
        <summary>{t("lad.shortcuts")}</summary>
        <dl>
          {SHORTCUTS.map(([keys, text]) => (
            <div key={keys} className="row">
              <dt className="mono">{keys}</dt>
              <dd>{t(text)}</dd>
            </div>
          ))}
        </dl>
      </details>
    </div>
  );
}

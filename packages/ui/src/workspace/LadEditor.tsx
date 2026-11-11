/**
 * Ladder Diagram editor (ADR-0050, ADR-0051, ADR-0052): direct manipulation of the ladder
 * document. Elements are dragged from the palette or moved on the diagram — beside an
 * element for AND, below it for OR — operands and box pins are edited in place with
 * completion, instructions are typed at an insertion point, and every command has a
 * keyboard path. The document stays the typed ladder tree the backend compiles; layout,
 * hit testing and placement are pure functions of it (./lad).
 *
 * Online, everything shown on the rungs comes from `ladder_state`: the power leaving each
 * element as the runtime computed it, the values its variables had in that scan, timer
 * elapsed times and counter values. The editor computes no power flow.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api, type DiagnosticDto } from "@plc-lab/ipc";
import { useInterface } from "../locale";
import { useLoad } from "../useLoad";
import type { Lab } from "./useLab";
import {
  addRung,
  addVariable,
  duplicateRung,
  findCoil,
  findNode,
  isLeaf,
  moveElement,
  moveRung,
  newLadDocument,
  parseLad,
  pasteRung,
  readingOrder,
  removeCoil,
  removeElement,
  removeRung,
  rungOf,
  serializeLad,
  setBlockPin,
  setCoil,
  setContactVariable,
  setRungComment,
  toggleContact,
  updateElement,
  type Coil,
  type LadDocument,
  type LadNode,
  type Operand,
  type Rung,
} from "./ladModel";
import {
  elementName,
  fieldLabel,
  fieldText,
  isTyping,
  useObserved,
  type Observed,
  type Selected,
  type T,
} from "./lad/common";
import {
  channelsOf,
  fieldsOf,
  instructions,
  type Field,
  type Instruction,
} from "./lad/complete";
import { ContextMenu, type MenuItem } from "./lad/ContextMenu";
import { InlineEditor, type Accepted } from "./lad/InlineEditor";
import { InstructionPicker } from "./lad/InstructionPicker";
import {
  coilColumn,
  fieldBox,
  layoutRung,
  targetBox,
  type RungLayout,
} from "./lad/layout";
import { Palette, type Where } from "./lad/Palette";
import {
  applyDrop,
  expression,
  type DropError,
  type PaletteItem,
  type Source,
  type Target,
} from "./lad/placement";
import { Properties } from "./lad/Properties";
import { RungCanvas, type Preview } from "./lad/RungCanvas";
import { useDrag, type DragState } from "./lad/useDrag";
import { Variables } from "./lad/Variables";

const HISTORY = 100;

type Editing = { element: number; field: Field; initial: string };
type Picking = { target: Target; rung: number; initial: string };
type Floating = { left: number; top: number };
type Menu = { x: number; y: number; id: number; items: MenuItem[] };
type Clip =
  | { kind: "rung"; rung: Rung }
  | { kind: "node"; node: LadNode }
  | { kind: "coil"; coil: Coil };

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
  const [clipboard, setClipboard] = useState<Clip | null>(null);
  const [history, setHistory] = useState<{ past: string[]; future: string[] }>({
    past: [],
    future: [],
  });
  const [editing, setEditing] = useState<Editing | null>(null);
  const [picking, setPicking] = useState<
    (Picking & { position: Floating }) | null
  >(null);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const canvases = useRef(new Map<number, HTMLDivElement>());
  const observed = useObserved(lab, document);
  const library = useLoad(() => api.library(), []);
  const standard = library.data?.standardBlocks;
  const instructionList = useMemo(
    () => instructions(standard ?? []),
    [standard],
  );
  const station = lab.project?.station;
  const channels = useMemo(() => channelsOf(station), [station]);
  const layouts = useMemo(() => {
    const map = new Map<number, RungLayout>();
    if (!document) return map;
    const x = coilColumn(document);
    for (const r of document.rungs) map.set(r.id, layoutRung(r, x));
    return map;
  }, [document]);

  const focusElement = useCallback((id: number) => {
    requestAnimationFrame(() =>
      root.current
        ?.querySelector<HTMLElement>(`[data-element="${id}"]`)
        ?.focus(),
    );
  }, []);
  const commit = (next: LadDocument) => {
    const text = serializeLad(next);
    if (text === lab.source) return;
    setHistory((h) => ({
      past: [...h.past, lab.source].slice(-HISTORY),
      future: [],
    }));
    lab.setSource(text);
  };
  const kindOf = (d: LadDocument, id: number): Selected => {
    if (findCoil(d, id)) return { kind: "coil", id };
    if (d.rungs.some((r) => r.id === id)) return { kind: "rung", id };
    return { kind: "node", id };
  };
  /** Open the inline editor on `field` of `element`, or on its first field. */
  const openEditor = (
    d: LadDocument,
    element: number,
    field?: Field,
    initial?: string,
  ) => {
    const coil = findCoil(d, element);
    const node = coil ? null : findNode(d, element);
    const fields = fieldsOf(node, !!coil);
    const f = field ?? fields[0];
    if (!f || (!coil && (!node || !isLeaf(node)))) return;
    setPicking(null);
    setMenu(null);
    setSelected(kindOf(d, element));
    setEditing({
      element,
      field: f,
      initial: initial ?? fieldText(node, f, coil ? coil.variable : undefined),
    });
  };
  /** After a new element is placed: select it and edit its first field. */
  const placed = (d: LadDocument, id: number, fresh: boolean) => {
    setNotice(null);
    setSelected(kindOf(d, id));
    if (fresh) openEditor(d, id);
    else focusElement(id);
  };
  const dropError = (e: DropError) => t(`lad.drop.${e}`);
  const apply = (source: Source, target: Target) => {
    if (!document) return;
    const r = applyDrop(document, source, target);
    if ("error" in r) {
      if (r.error !== "noChange") setNotice(dropError(r.error));
      return;
    }
    commit(r.document);
    placed(r.document, r.selected, source.kind !== "move");
  };
  const { drag, grab, justDropped } = useDrag({
    document,
    zoom,
    canvases,
    layouts,
    scroller: root,
    drop: apply,
  });

  // A diagnostic chosen in the problem list selects its element.
  const [chosen, setChosen] = useState(focused);
  if (focused !== chosen) {
    setChosen(focused);
    if (focused?.element != null && document)
      setSelected(kindOf(document, focused.element));
  }
  // The selected element keeps the keyboard focus, unless a field is being edited.
  const floating = editing !== null || picking !== null || menu !== null;
  useEffect(() => {
    if (!selected || selected.kind === "point" || floating) return;
    const el = root.current?.querySelector<HTMLElement>(
      `[data-element="${selected.id}"]`,
    );
    if (el && !el.contains(globalThis.document.activeElement)) el.focus();
  }, [selected, floating]);

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

  const selectedId = selected && selected.kind !== "point" ? selected.id : null;
  const rung =
    selected?.kind === "point"
      ? (document.rungs.find((r) => r.id === selected.rung) ?? null)
      : selectedId !== null
        ? rungOf(document, selectedId)
        : null;

  /** Where an insertion goes, from the selection: beside, below, or at a point. */
  const insertTarget = (how: "before" | "after" | "below"): Picking | null => {
    if (selected?.kind === "point")
      return { target: selected.target, rung: selected.rung, initial: "" };
    if (selected?.kind === "node" && rung)
      return {
        target: { zone: how, id: selected.id },
        rung: rung.id,
        initial: "",
      };
    const r = rung ?? document.rungs.at(-1);
    if (!r) return null;
    const items = r.network.type === "series" ? r.network.items : [r.network];
    const last = items.at(-1);
    const first = items[0];
    const target: Target = !last
      ? { zone: "start", rung: r.id }
      : how === "below"
        ? { zone: "below", id: r.network.id }
        : how === "before" && first
          ? { zone: "before", id: first.id }
          : { zone: "after", id: last.id };
    return { target, rung: r.id, initial: "" };
  };
  const insertItem = (item: PaletteItem, at: Picking | null) => {
    if (!at) return;
    const r = document.rungs.find((x) => x.id === at.rung);
    const target: Target =
      item.type === "coil"
        ? { zone: "coil", rung: at.rung, index: r?.coils.length ?? 0 }
        : at.target;
    apply({ kind: "new", item }, target);
  };
  const remove = () => {
    if (!selected || selected.kind === "point") return;
    if (selected.kind === "node") commit(removeElement(document, selected.id));
    if (selected.kind === "coil") commit(removeCoil(document, selected.id));
    if (selected.kind === "rung") commit(removeRung(document, selected.id));
    setSelected(null);
  };
  const copySelection = () => {
    if (selectedId === null) return;
    const coil = findCoil(document, selectedId);
    const node = findNode(document, selectedId);
    if (selected?.kind === "rung" && rung) setClipboard({ kind: "rung", rung });
    else if (coil) setClipboard({ kind: "coil", coil });
    else if (node) setClipboard({ kind: "node", node });
  };
  const pasteClipboard = () => {
    if (!clipboard) return;
    if (clipboard.kind === "rung") {
      commit(pasteRung(document, clipboard.rung, rung?.id));
      return;
    }
    const at = insertTarget("after");
    if (!at) return;
    if (clipboard.kind === "coil") {
      const r = document.rungs.find((x) => x.id === at.rung);
      apply(
        { kind: "pasteCoil", coil: clipboard.coil },
        { zone: "coil", rung: at.rung, index: r?.coils.length ?? 0 },
      );
    } else apply({ kind: "paste", node: clipboard.node }, at.target);
  };

  const order = readingOrder(document);
  const navigate = (key: string) => {
    const r = order.findIndex(
      (ids) => selectedId !== null && ids.includes(selectedId),
    );
    if (r < 0) {
      const first = order[0]?.[0];
      if (first !== undefined) setSelected(kindOf(document, first));
      return;
    }
    const row = order[r] ?? [];
    const i = selectedId !== null ? row.indexOf(selectedId) : 0;
    let target: number | undefined;
    if (key === "ArrowLeft") target = row[Math.max(0, i - 1)];
    if (key === "ArrowRight") target = row[Math.min(row.length - 1, i + 1)];
    if (key === "ArrowUp" || key === "ArrowDown") {
      const other = order[r + (key === "ArrowUp" ? -1 : 1)];
      if (other?.length) target = other[Math.min(i, other.length - 1)];
    }
    if (target !== undefined) setSelected(kindOf(document, target));
  };
  const editable = (id: number | null) => {
    if (id === null) return false;
    if (findCoil(document, id)) return true;
    const n = findNode(document, id);
    return !!n && isLeaf(n);
  };
  const openMenuAt = (id: number) => {
    const el = root.current?.querySelector<HTMLElement>(
      `[data-element="${id}"]`,
    );
    const r = el?.getBoundingClientRect();
    openMenu(r ? r.left + 8 : 80, r ? r.bottom + 4 : 80, id);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (isTyping(e.target)) return;
    // A key on the diagram while an editor is open elsewhere: that editor is abandoned.
    if (editing || picking || menu) {
      setEditing(null);
      setPicking(null);
      setMenu(null);
    }
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key;
    const lower = key.toLowerCase();
    if (key === "F9" && !e.shiftKey)
      insertItem(
        { type: "contact", contact: "normallyOpen" },
        insertTarget("after"),
      );
    else if (key === "F9" && e.shiftKey)
      insertItem(
        { type: "contact", contact: "normallyOpen" },
        insertTarget("below"),
      );
    else if (key === "F10" && !e.shiftKey)
      insertItem(
        { type: "contact", contact: "normallyClosed" },
        insertTarget("after"),
      );
    else if (key === "ContextMenu" || (key === "F10" && e.shiftKey)) {
      if (selectedId !== null) openMenuAt(selectedId);
    } else if (key === "F8") {
      const at = insertTarget("after");
      if (at) openPicker(at);
    } else if ((ctrl && key === " ") || key === "Enter" || key === "F2") {
      if (editable(selectedId) && selectedId !== null)
        openEditor(document, selectedId);
      else {
        const at = insertTarget("after");
        if (at) openPicker(at);
      }
    } else if (ctrl && lower === "z" && !e.shiftKey) undo();
    else if (ctrl && (lower === "y" || (lower === "z" && e.shiftKey))) redo();
    else if (ctrl && lower === "c") copySelection();
    else if (ctrl && lower === "v") pasteClipboard();
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
    else if (key === "Escape") {
      setSelected(null);
      setNotice(null);
    } else if (key.length === 1 && !ctrl && !e.altKey && key !== " ") {
      // Typing edits the selected element, or searches instructions at a point.
      if (editable(selectedId) && selectedId !== null)
        openEditor(document, selectedId, undefined, key);
      else {
        const at = insertTarget("after");
        if (at) openPicker({ ...at, initial: key });
      }
    } else return;
    e.preventDefault();
  };

  /** Apply what the inline editor accepted, as one undo step. */
  const acceptEdit = (choice: Accepted, advance: -1 | 0 | 1) => {
    if (!editing) return;
    const { element, field } = editing;
    let next = document;
    if (choice.kind === "create") {
      const declared = addVariable(
        next,
        choice.name,
        choice.direction,
        choice.type,
      );
      if (!declared) {
        setNotice(t("lad.suggest.instanceTaken", { name: choice.name }));
        return;
      }
      next = declared;
    }
    const value =
      choice.kind === "clear"
        ? ""
        : choice.kind === "literal"
          ? choice.text
          : choice.name;
    const operand: Operand =
      choice.kind === "literal" ? { constant: value } : { variable: value };
    const coil = findCoil(next, element);
    const node = findNode(next, element);
    if (coil) next = setCoil(next, element, { variable: value });
    else if (node?.type === "contact")
      next = setContactVariable(next, element, value);
    else if (node?.type === "compare")
      next = updateElement(next, element, (n) =>
        n.type === "compare"
          ? { ...n, [field === "left" ? "left" : "right"]: operand }
          : n,
      );
    else if (node?.type === "block") {
      if (field === "instance") {
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(value)) {
          setNotice(t("lad.suggest.instanceTaken", { name: value }));
          return;
        }
        next = updateElement(next, element, (n) =>
          n.type === "block" ? { ...n, instance: value } : n,
        );
      } else if (field === "preset")
        next = updateElement(next, element, (n) => {
          if (n.type !== "block") return n;
          const { preset: _p, ...rest } = n;
          void _p;
          return value ? { ...rest, preset: operand } : rest;
        });
      else if (field === "reset")
        next = setBlockPin(next, element, "reset", value ? operand : null);
      else next = setBlockPin(next, element, "output", value);
    }
    commit(next);
    setEditing(null);
    setNotice(null);
    if (advance === 0) {
      focusElement(element);
      return;
    }
    // Tab: the next field of this element, then the next element of the rung.
    const fields = fieldsOf(node, !!coil);
    const i = fields.indexOf(field) + advance;
    const nextField = fields[i];
    if (nextField) return openEditor(next, element, nextField);
    const row = order.find((ids) => ids.includes(element)) ?? [];
    const candidates =
      advance > 0
        ? row.slice(row.indexOf(element) + 1)
        : row.slice(0, row.indexOf(element)).reverse();
    const target = candidates.find((id) => editable(id));
    if (target === undefined) return focusElement(element);
    const tc = findCoil(next, target);
    const tf = fieldsOf(tc ? null : findNode(next, target), !!tc);
    openEditor(next, target, advance > 0 ? tf[0] : tf.at(-1));
  };
  const cancelEdit = () => {
    const id = editing?.element;
    setEditing(null);
    if (id !== undefined) focusElement(id);
  };

  const menuItems = (id: number): MenuItem[] => {
    const coil = findCoil(document, id);
    const node = findNode(document, id);
    const r = rungOf(document, id);
    const no: PaletteItem = { type: "contact", contact: "normallyOpen" };
    const del = { label: t("lad.menu.delete"), run: () => remove() };
    const props = {
      label: t("lad.menu.properties"),
      run: () =>
        root.current
          ?.querySelector<HTMLElement>(
            ".lad-properties select, .lad-properties input",
          )
          ?.focus(),
    };
    if (coil && r) {
      const index = r.coils.findIndex((c) => c.id === id);
      return [
        { label: t("lad.menu.edit"), run: () => openEditor(document, id) },
        {
          label: t("lad.menu.duplicate"),
          run: () =>
            apply(
              { kind: "move", id, copy: true },
              { zone: "coil", rung: r.id, index: index + 1 },
            ),
        },
        del,
        props,
      ];
    }
    if (!node || !r) return [];
    const items: MenuItem[] = [
      {
        label: t("lad.menu.insertBefore"),
        run: () => apply({ kind: "new", item: no }, { zone: "before", id }),
      },
      {
        label: t("lad.menu.insertAfter"),
        run: () => apply({ kind: "new", item: no }, { zone: "after", id }),
      },
      {
        label: t("lad.menu.addBranch"),
        run: () =>
          apply(
            { kind: "new", item: no },
            node.type === "parallel"
              ? { zone: "branch", id }
              : { zone: "below", id },
          ),
      },
      {
        label: t("lad.menu.instruction"),
        run: () =>
          openPicker({
            target: { zone: "after", id },
            rung: r.id,
            initial: "",
          }),
      },
    ];
    if (node.type === "contact")
      items.push({
        label: t(
          node.contact === "normallyOpen" ? "lad.menu.toNc" : "lad.menu.toNo",
        ),
        run: () => commit(toggleContact(document, id)),
      });
    if (isLeaf(node))
      items.push({
        label: t("lad.menu.edit"),
        run: () => openEditor(document, id),
      });
    items.push(
      {
        label: t("lad.menu.duplicate"),
        run: () =>
          apply({ kind: "move", id, copy: true }, { zone: "after", id }),
      },
      del,
    );
    if (isLeaf(node)) items.push(props);
    return items;
  };

  const preview = (r: Rung): Preview | null => {
    const over = drag?.over;
    if (!over || over.rung !== r.id) return null;
    if ("target" in over)
      return over.error
        ? { kind: "invalid", x: over.x, y: over.y }
        : {
            kind: "target",
            target: over.target,
            copy: drag.source.kind === "move" && drag.source.copy,
          };
    if ("wrong" in over) return { kind: "invalid", x: over.x, y: over.y };
    return null;
  };
  const placeName = (target: Target): string => {
    if (target.zone === "start" || target.zone === "coil") {
      const number = document.rungs.findIndex((x) => x.id === target.rung) + 1;
      return target.zone === "start"
        ? t("lad.drop.start", { number })
        : t("lad.drop.coil", { position: target.index + 1, number });
    }
    const node = findNode(document, target.id);
    const isRoot = document.rungs.some((x) => x.network.id === target.id);
    if (target.zone === "below" && isRoot) return t("lad.drop.belowAll");
    const name =
      node?.type === "parallel"
        ? t("lad.parallelBlock", { count: node.branches.length })
        : elementName(t, node);
    if (target.zone === "branch") return t("lad.drop.branch");
    return t(`lad.drop.${target.zone}`, { name });
  };
  const dragText = (d: DragState): string => {
    const over = d.over;
    if (!over) return t("lad.drop.none");
    if ("wrong" in over)
      return t(
        over.wrong === "coil"
          ? "lad.drop.elementInCoils"
          : "lad.drop.coilInNetwork",
      );
    if ("nothing" in over) return t("lad.drop.none");
    if (over.error) return dropError(over.error);
    // The resulting logic of the rung, so the topology is never a surprise.
    const result = applyDrop(document, d.source, over.target);
    const changed =
      "document" in result
        ? result.document.rungs.find((x) => x.id === over.rung)
        : null;
    const coilTarget = over.target.zone === "coil";
    return `${placeName(over.target)}${changed && !coilTarget ? ` → ${expression(changed.network)}` : ""}`;
  };

  const editorFor = (r: Rung) => {
    if (!editing) return null;
    const layout = layouts.get(r.id);
    if (!layout) return null;
    const box = fieldBox(layout, editing.element, editing.field);
    if (!box) return null;
    const coil = findCoil(document, editing.element);
    const node = coil ? null : findNode(document, editing.element);
    return (
      <div
        className="lad-inline-host"
        style={{
          left: box.x,
          top: box.y,
          width: Math.max(box.w, 150),
          height: box.h,
        }}
      >
        <InlineEditor
          key={`${editing.element}:${editing.field}`}
          t={t}
          document={document}
          element={editing.element}
          field={editing.field}
          label={t("lad.inlineEdit", {
            field: fieldLabel(t, node, editing.field, !!coil),
            element: elementName(t, node, coil ? coil.variable : undefined),
          })}
          initial={editing.initial}
          channels={channels}
          accept={acceptEdit}
          cancel={cancelEdit}
        />
      </div>
    );
  };

  /** Open the instruction picker under its insertion target (measured now, in the handler). */
  const openPicker = (p: Picking) => {
    setMenu(null);
    setPicking({ ...p, position: pickerPosition(p) });
  };
  const openMenu = (x: number, y: number, id: number) =>
    setMenu({ x, y, id, items: menuItems(id) });
  const pickerPosition = (p: Picking): Floating => {
    const canvas = canvases.current.get(p.rung);
    const layout = layouts.get(p.rung);
    const r = canvas?.getBoundingClientRect();
    const b = layout ? targetBox(layout, p.target) : null;
    if (!r || !b) return { left: 80, top: 120 };
    const x = p.target.zone === "after" ? b.x + b.w : b.x;
    return {
      left: Math.max(4, Math.min(r.left + x * zoom, window.innerWidth - 420)),
      top: Math.min(r.top + (b.y + b.h) * zoom + 4, window.innerHeight - 330),
    };
  };

  return (
    <div
      className={`lad${drag ? " dragging" : ""}`}
      data-testid="lad-editor"
      ref={root}
      onKeyDown={onKeyDown}
    >
      <Palette
        t={t}
        where={where}
        setWhere={setWhere}
        canInsert={!!selected || !!rung}
        insert={(item) =>
          insertItem(item, insertTarget(where === "parallel" ? "below" : where))
        }
        grab={grab}
        justDropped={justDropped}
        remove={selected && selected.kind !== "point" ? remove : null}
        move={
          selected?.kind === "node"
            ? (delta) => commit(moveElement(document, selected.id, delta))
            : null
        }
        undo={history.past.length ? undo : null}
        redo={history.future.length ? redo : null}
        copy={rung ? () => setClipboard({ kind: "rung", rung }) : null}
        paste={
          clipboard?.kind === "rung"
            ? () => commit(pasteRung(document, clipboard.rung, rung?.id))
            : null
        }
        zoom={zoom}
        setZoom={setZoom}
      />
      <Status t={t} lab={lab} observed={observed} fault={fault} />
      <p className="lad-notice" role="status" aria-live="polite">
        {drag ? dragText(drag) : (notice ?? "")}
      </p>
      <div className="lad-rungs" role="list" aria-label={t("lad.rungs")}>
        {document.rungs.map((r, index) => {
          const layout = layouts.get(r.id);
          return (
            <RungFrame
              key={r.id}
              t={t}
              rung={r}
              number={index + 1}
              count={document.rungs.length}
              selected={selected}
              select={setSelected}
              errors={errors}
              change={(edit) => commit(edit(document))}
            >
              {layout && (
                <RungCanvas
                  t={t}
                  rung={r}
                  layout={layout}
                  zoom={zoom}
                  selected={selected}
                  select={(s) => {
                    setSelected(s);
                    setMenu(null);
                  }}
                  observed={observed}
                  errors={errors}
                  preview={preview(r)}
                  editor={
                    editing && rungOf(document, editing.element)?.id === r.id
                      ? editorFor(r)
                      : null
                  }
                  register={(id, el) => {
                    if (el) canvases.current.set(id, el);
                    else canvases.current.delete(id);
                  }}
                  grab={grab}
                  edit={(element, field) =>
                    openEditor(document, element, field)
                  }
                  menu={(e, id) => openMenu(e.clientX, e.clientY, id)}
                />
              )}
            </RungFrame>
          );
        })}
        <button
          type="button"
          className="btn lad-add-rung"
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
      {picking &&
        createPortal(
          <div className="lad-floating" style={picking.position}>
            <InstructionPicker
              t={t}
              list={instructionList}
              initial={picking.initial}
              where={placeName(picking.target)}
              choose={(i: Instruction) => {
                setPicking(null);
                insertItem(i.item, picking);
              }}
              cancel={() => {
                setPicking(null);
                root.current?.focus();
              }}
            />
          </div>,
          globalThis.document.body,
        )}
      {menu &&
        createPortal(
          <ContextMenu
            t={t}
            x={menu.x}
            y={menu.y}
            items={menu.items}
            close={() => {
              setMenu(null);
              focusElement(menu.id);
            }}
          />,
          globalThis.document.body,
        )}
      {drag &&
        createPortal(
          <div
            className={`lad-ghost${drag.over && "target" in drag.over && !drag.over.error ? " valid" : drag.over && ("wrong" in drag.over || ("target" in drag.over && drag.over.error && drag.over.error !== "noChange")) ? " invalid" : ""}`}
            style={{ left: drag.x + 14, top: drag.y + 14 }}
            aria-hidden="true"
          >
            <strong className="mono">
              {drag.label}
              {drag.source.kind === "move" && drag.source.copy
                ? ` + ${t("lad.drop.copy")}`
                : ""}
            </strong>
            <span>{dragText(drag)}</span>
          </div>,
          globalThis.document.body,
        )}
    </div>
  );
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

/** A rung: its header (handle, comment, order, duplicate, delete) and its canvas. */
function RungFrame({
  t,
  rung,
  number,
  count,
  selected,
  select,
  errors,
  change,
  children,
}: {
  t: T;
  rung: Rung;
  number: number;
  count: number;
  selected: Selected;
  select: (s: Selected) => void;
  errors: Map<number, string[]>;
  change: (edit: (d: LadDocument) => LadDocument) => void;
  children: React.ReactNode;
}) {
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
          type="button"
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
          type="button"
          className="btn btn-sm"
          disabled={number === 1}
          aria-label={t("lad.rungUp", { number })}
          onClick={() => change((d) => moveRung(d, rung.id, -1))}
        >
          ↑
        </button>
        <button
          type="button"
          className="btn btn-sm"
          disabled={number === count}
          aria-label={t("lad.rungDown", { number })}
          onClick={() => change((d) => moveRung(d, rung.id, 1))}
        >
          ↓
        </button>
        <button
          type="button"
          className="btn btn-sm"
          onClick={() => change((d) => duplicateRung(d, rung.id))}
        >
          {t("lad.duplicateRung")}
        </button>
        <button
          type="button"
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
      {children}
    </section>
  );
}

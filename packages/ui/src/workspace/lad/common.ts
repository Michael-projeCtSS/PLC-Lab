/** Shared vocabulary of the ladder editor's components (ADR-0050, ADR-0051, ADR-0052). */
import { useMemo } from "react";
import { api, type LadderStateDto, type SignalValueDto } from "@plc-lab/ipc";
import { formatValue } from "../../format";
import type { TranslationKey } from "../../locale/en";
import { useFrameQuery } from "../useFrameQuery";
import type { Lab } from "../useLab";
import type {
  BlockKind,
  CoilKind,
  CompareOp,
  ContactKind,
  LadDocument,
  LadNode,
  Operand,
} from "../ladModel";
import type { Field, Instruction } from "./complete";
import type { Target } from "./placement";

export type T = (
  key: TranslationKey,
  values?: Record<string, string | number>,
) => string;

/** What is selected: an element, a rung, or an insertion point on a series wire. */
export type Selected =
  | { kind: "node" | "coil" | "rung"; id: number }
  | { kind: "point"; rung: number; target: Target; after: number | null }
  | null;

export const CONTACTS: {
  kind: ContactKind;
  symbol: string;
  label: TranslationKey;
}[] = [
  { kind: "normallyOpen", symbol: "| |", label: "lad.noContact" },
  { kind: "normallyClosed", symbol: "|/|", label: "lad.ncContact" },
  { kind: "positiveEdge", symbol: "|P|", label: "lad.pContact" },
  { kind: "negativeEdge", symbol: "|N|", label: "lad.nContact" },
];
export const OPS: { op: CompareOp; symbol: string }[] = [
  { op: "eq", symbol: "=" },
  { op: "ne", symbol: "<>" },
  { op: "gt", symbol: ">" },
  { op: "ge", symbol: ">=" },
  { op: "lt", symbol: "<" },
  { op: "le", symbol: "<=" },
];
export const BLOCKS: BlockKind[] = ["TON", "TOF", "TP", "CTU", "CTD"];
export const COILS: {
  kind: CoilKind;
  symbol: string;
  label: TranslationKey;
}[] = [
  { kind: "output", symbol: "( )", label: "lad.addCoil" },
  { kind: "set", symbol: "(S)", label: "lad.addSetCoil" },
  { kind: "reset", symbol: "(R)", label: "lad.addResetCoil" },
];
export const ZOOMS = [0.6, 0.7, 0.8, 0.9, 1, 1.1, 1.25, 1.5];

export const symbolOf = (kind: ContactKind) =>
  CONTACTS.find((c) => c.kind === kind)?.symbol ?? "| |";
export const coilSymbol = (kind: CoilKind | undefined) =>
  kind === "set" ? "(S)" : kind === "reset" ? "(R)" : "( )";
export const opSymbol = (op: CompareOp) =>
  OPS.find((o) => o.op === op)?.symbol ?? op;
export const operandText = (o: Operand | undefined) =>
  !o ? "" : "variable" in o ? o.variable : o.constant;
/** Focus is in a text field, select or text area: the editor's shortcuts stay quiet. */
export const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName) ||
    target.isContentEditable);

/** What the runtime reported for this ladder program in its latest recorded cycle. */
export type Observed = {
  online: boolean;
  scanned: boolean;
  cycle: string | null;
  power: (element: number) => boolean | null;
  value: (name: string) => SignalValueDto | null;
};

export function useObserved(lab: Lab, document: LadDocument | null): Observed {
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

export const isOn = (value: SignalValueDto | null): boolean =>
  value?.type === "BOOL" && value.value;

/** "Normally open contact X: TRUE, power flows" — values and power from the runtime. */
export function describe(
  t: T,
  label: string,
  value: SignalValueDto | null,
  power: boolean | null,
): string {
  const parts = [value ? `${label}: ${formatValue(value)}` : label];
  if (power !== null) parts.push(t(power ? "lad.powered" : "lad.unpowered"));
  return parts.join(", ");
}

/** The accessible name of an element without its online state: "TON RunDelay". */
export function elementName(
  t: T,
  node: LadNode | null,
  coilVariable?: string,
): string {
  if (coilVariable !== undefined)
    return t("lad.coilNamed", {
      variable: coilVariable || t("lad.noVariable"),
    });
  if (!node) return "";
  if (node.type === "contact") {
    const kind = CONTACTS.find((c) => c.kind === node.contact);
    return t(kind?.label ?? "lad.noContact", {
      variable: node.variable || t("lad.noVariable"),
    });
  }
  if (node.type === "compare")
    return `${t("lad.compare")} ${operandText(node.left) || "?"} ${opSymbol(node.op)} ${operandText(node.right) || "?"}`;
  if (node.type === "block") return `${node.block} ${node.instance}`;
  return "";
}

/** The label of a field: "Preset time PT", "Variable of the coil", … */
export function fieldLabel(
  t: T,
  node: LadNode | null,
  field: Field,
  coil: boolean,
): string {
  if (coil) return t("lad.coilVariable");
  switch (field) {
    case "variable":
      return t("lad.contactVariable");
    case "left":
      return t("lad.left");
    case "right":
      return t("lad.right");
    case "instance":
      return t("lad.instance");
    case "preset":
      return node?.type === "block" &&
        (node.block === "CTU" || node.block === "CTD")
        ? t("lad.presetPV")
        : t("lad.presetPT");
    case "reset":
      return node?.type === "block" && node.block === "CTD"
        ? t("lad.loadLD")
        : t("lad.resetR");
    case "output":
      return node?.type === "block" &&
        (node.block === "CTU" || node.block === "CTD")
        ? t("lad.outputCV")
        : t("lad.outputET");
  }
}

/** The current text of a field, as the inline editor starts with it. */
export function fieldText(
  node: LadNode | null,
  field: Field,
  coilVariable?: string,
): string {
  if (coilVariable !== undefined) return coilVariable;
  if (!node) return "";
  if (node.type === "contact") return node.variable;
  if (node.type === "compare")
    return operandText(field === "left" ? node.left : node.right);
  if (node.type === "block") {
    if (field === "instance") return node.instance;
    if (field === "preset") return operandText(node.preset);
    if (field === "reset") return operandText(node.reset);
    if (field === "output") return node.output ?? "";
  }
  return "";
}

/** The localized description of an instruction; boxes use the standard library text. */
export function describeInstruction(t: T, i: Instruction): string {
  const key = (
    i.item.type === "block" ? `lib.std.${i.name}` : `lad.instr.${i.name}`
  ) as TranslationKey;
  return t(key);
}

/**
 * Deterministic, local completion for the ladder editor (ADR-0052): variables of the
 * program filtered by what the field accepts, IEC literals, instance-name proposals and
 * the instructions LAD supports. Ranking is plain string matching; nothing is guessed and
 * nothing is created without the student confirming it.
 */
import type { StandardBlockDto, StationDto } from "@plc-lab/ipc";
import {
  findCoil,
  findNode,
  isTimer,
  nextInstance,
  type BlockKind,
  type CompareOp,
  type ContactKind,
  type CoilKind,
  type Direction,
  type LadDocument,
  type LadNode,
  type LadType,
  type Operand,
} from "../ladModel";
import type { PaletteItem } from "./placement";

/** The editable fields of ladder elements, in Tab order per element kind. */
export type Field =
  "variable" | "left" | "right" | "instance" | "preset" | "reset" | "output";

export function fieldsOf(node: LadNode | null, coil: boolean): Field[] {
  if (coil) return ["variable"];
  if (!node) return [];
  if (node.type === "contact") return ["variable"];
  if (node.type === "compare") return ["left", "right"];
  if (node.type === "block")
    return isTimer(node.block)
      ? ["instance", "preset", "output"]
      : ["instance", "preset", "reset", "output"];
  return [];
}

/** What a field accepts. */
export type Expectation = {
  /** Variable types offered (in preference order); empty for an instance name. */
  types: LadType[];
  /** Only variables a coil or output may write (no inputs). */
  writable: boolean;
  /** IEC literals of these types are accepted. */
  literals: LadType[];
  instance: boolean;
};

const typeOfOperand = (document: LadDocument, o: Operand): LadType | null => {
  if ("variable" in o)
    return (
      document.variables.find(
        (v) => v.name.toLowerCase() === o.variable.toLowerCase(),
      )?.type ?? null
    );
  return literalType(o.constant);
};

/** The type of an IEC literal as the Rust compiler reads it, or null. */
export function literalType(text: string): LadType | null {
  const t = text.trim();
  if (/^(TRUE|FALSE)$/i.test(t)) return "BOOL";
  if (/^(T|TIME)#(\d+(\.\d+)?(d|h|ms|m|s|us)_?)+$/i.test(t)) return "TIME";
  if (/^[+-]?\d+$/.test(t)) return "INT";
  if (/^[+-]?\d+\.\d+([eE][+-]?\d+)?$/.test(t)) return "REAL";
  return null;
}

export function expectationFor(
  document: LadDocument,
  element: number,
  field: Field,
): Expectation {
  if (findCoil(document, element))
    return { types: ["BOOL"], writable: true, literals: [], instance: false };
  const node = findNode(document, element);
  if (node?.type === "compare") {
    const other = field === "left" ? node.right : node.left;
    const known = typeOfOperand(document, other);
    const any: LadType[] =
      node.op === "eq" || node.op === "ne"
        ? ["INT", "REAL", "TIME", "BOOL"]
        : ["INT", "REAL", "TIME"];
    const types = known && any.includes(known) ? [known] : any;
    return { types, writable: false, literals: types, instance: false };
  }
  if (node?.type === "block") {
    const timer = isTimer(node.block);
    if (field === "instance")
      return { types: [], writable: false, literals: [], instance: true };
    if (field === "preset") {
      const t: LadType = timer ? "TIME" : "INT";
      return { types: [t], writable: false, literals: [t], instance: false };
    }
    if (field === "reset")
      return {
        types: ["BOOL"],
        writable: false,
        literals: ["BOOL"],
        instance: false,
      };
    return {
      types: [timer ? "TIME" : "INT"],
      writable: true,
      literals: [],
      instance: false,
    };
  }
  return { types: ["BOOL"], writable: false, literals: [], instance: false };
}

export type Suggestion =
  | {
      kind: "variable";
      name: string;
      type: LadType;
      direction: Direction;
      comment: string | null;
      channel: string | null;
    }
  | { kind: "literal"; text: string; type: LadType }
  | { kind: "instance"; name: string }
  | { kind: "create"; name: string; type: LadType; direction: Direction };

/** Match score of `name` for `query`; null when it does not match. */
export function score(name: string, query: string): number | null {
  const q = query.trim().toLowerCase();
  if (!q) return 1;
  const n = name.toLowerCase();
  if (n === q) return 100;
  if (n.startsWith(q)) return 80 - Math.min(20, n.length - q.length) / 2;
  // Word starts: "rd" finds RunDelay, "run_d" finds Run_Delay.
  const words = name
    .split(/(?=[A-Z])|_/)
    .filter(Boolean)
    .map((w) => w.toLowerCase());
  const initials = words.map((w) => w[0]).join("");
  if (initials.startsWith(q)) return 60;
  if (words.some((w) => w.startsWith(q))) return 55;
  const at = n.indexOf(q);
  if (at >= 0) return 40 - Math.min(20, at) / 2;
  return null;
}

const TIME_LITERALS = [
  "T#100ms",
  "T#500ms",
  "T#1s",
  "T#2s",
  "T#3s",
  "T#5s",
  "T#10s",
  "T#30s",
  "T#1m",
];

function literals(query: string, types: LadType[]): Suggestion[] {
  const q = query.trim();
  const out: Suggestion[] = [];
  const add = (text: string, type: LadType) => {
    if (!out.some((s) => s.kind === "literal" && s.text === text))
      out.push({ kind: "literal", text, type });
  };
  const typed = literalType(q);
  if (typed && types.includes(typed))
    add(
      typed === "TIME"
        ? `T#${q.replace(/^(T|TIME)#/i, "").toLowerCase()}`
        : typed === "BOOL"
          ? q.toUpperCase()
          : q,
      typed,
    );
  if (types.includes("TIME")) {
    if (/^\d+$/.test(q)) {
      add(`T#${q}s`, "TIME");
      add(`T#${q}ms`, "TIME");
    }
    for (const t of TIME_LITERALS)
      if (!q || t.toLowerCase().startsWith(q.toLowerCase())) add(t, "TIME");
  }
  if (types.includes("BOOL"))
    for (const t of ["TRUE", "FALSE"])
      if (!q || t.startsWith(q.toUpperCase())) add(t, "BOOL");
  if (types.includes("REAL") && /^[+-]?\d+$/.test(q)) add(`${q}.0`, "REAL");
  return out;
}

/** "Slot 1, channel 2" for a tag wired in the station's I/O map. */
export function channelsOf(station: StationDto | null | undefined) {
  const map = new Map<string, string>();
  for (const io of station?.ioMap ?? [])
    map.set(
      io.tag.toLowerCase(),
      `${io.channel.rack}.${io.channel.slot}.${io.channel.channel}`,
    );
  return map;
}

/** Instance names that are free: the next T#/C# name and one derived from the rung. */
function instanceProposals(
  document: LadDocument,
  element: number,
  query: string,
): Suggestion[] {
  const node = findNode(document, element);
  if (node?.type !== "block") return [];
  const out: Suggestion[] = [];
  const taken = takenNames(document, element);
  const free = (n: string) =>
    /^[A-Za-z_][A-Za-z0-9_]*$/.test(n) && !taken.has(n.toLowerCase());
  const q = query.trim();
  if (q && free(q)) out.push({ kind: "instance", name: q });
  const generated = nextInstance(document, node.block);
  if (free(generated) && generated !== q)
    out.push({ kind: "instance", name: generated });
  if (node.instance && node.instance !== q && free(node.instance))
    out.push({ kind: "instance", name: node.instance });
  return out;
}

/** Names a box instance may not take: variables and the other boxes' instances. */
export function takenNames(
  document: LadDocument,
  element: number,
): Set<string> {
  const names = new Set(document.variables.map((v) => v.name.toLowerCase()));
  const walk = (n: LadNode) => {
    if (n.type === "block" && n.id !== element)
      names.add(n.instance.toLowerCase());
    if (n.type === "series") n.items.forEach(walk);
    if (n.type === "parallel") n.branches.forEach(walk);
  };
  document.rungs.forEach((r) => walk(r.network));
  return names;
}

/** Suggestions for `field` of `element` while the student has typed `query`. */
export function suggest(
  document: LadDocument,
  element: number,
  field: Field,
  query: string,
  channels: Map<string, string> = new Map(),
): Suggestion[] {
  const expect = expectationFor(document, element, field);
  if (expect.instance) return instanceProposals(document, element, query);
  const variables = document.variables
    .filter(
      (v) =>
        expect.types.includes(v.type) &&
        (!expect.writable || v.direction !== "input"),
    )
    .map((v) => ({ v, s: score(v.name, query) }))
    .filter((x): x is { v: (typeof x)["v"]; s: number } => x.s !== null)
    .sort(
      (a, b) =>
        b.s - a.s ||
        expect.types.indexOf(a.v.type) - expect.types.indexOf(b.v.type) ||
        a.v.name.localeCompare(b.v.name),
    )
    .map(({ v }): Suggestion => ({
      kind: "variable",
      name: v.name,
      type: v.type,
      direction: v.direction,
      comment: v.comment ?? null,
      channel: channels.get(v.name.toLowerCase()) ?? null,
    }));
  const out: Suggestion[] = [...variables, ...literals(query, expect.literals)];
  // A name nobody declared: offer to declare it, never declare it silently.
  const q = query.trim();
  const declared = document.variables.some(
    (v) => v.name.toLowerCase() === q.toLowerCase(),
  );
  if (
    q &&
    !declared &&
    literalType(q) === null &&
    /^[A-Za-z_][A-Za-z0-9_]*$/.test(q) &&
    !takenNames(document, 0).has(q.toLowerCase())
  )
    out.push({
      kind: "create",
      name: q,
      type: expect.types[0] ?? "BOOL",
      direction: expect.writable ? "output" : "local",
    });
  return out;
}

/** An instruction offered by the instruction completion and the palette. */
export type Instruction = {
  /** What the student types: TON, NO, GE, S, … */
  name: string;
  item: PaletteItem;
  /** Other names that find it. */
  aliases: string[];
  /** Pins as the runtime declares them (boxes) or as the element has them. */
  pins: { name: string; type: string; direction: "in" | "out" }[];
};

const CONTACT_ITEMS: [string, ContactKind, string[]][] = [
  ["NO", "normallyOpen", ["contact", "| |", "-| |-", "XIC"]],
  ["NC", "normallyClosed", ["contact", "|/|", "-|/|-", "not", "XIO"]],
  ["P", "positiveEdge", ["edge", "rising", "|P|", "R_TRIG"]],
  ["N", "negativeEdge", ["edge", "falling", "|N|", "F_TRIG"]],
];
const COIL_ITEMS: [string, CoilKind, string[]][] = [
  ["COIL", "output", ["( )", "-( )-", "output", "OTE"]],
  ["S", "set", ["SET", "(S)", "latch", "OTL"]],
  ["R", "reset", ["RESET", "(R)", "unlatch", "OTU"]],
];
const COMPARES: [string, CompareOp, string[]][] = [
  ["EQ", "eq", ["=", "==", "CMP", "equal"]],
  ["NE", "ne", ["<>", "!=", "CMP", "not equal"]],
  ["GT", "gt", [">", "CMP", "greater"]],
  ["GE", "ge", [">=", "CMP", "greater or equal"]],
  ["LT", "lt", ["<", "CMP", "less"]],
  ["LE", "le", ["<=", "CMP", "less or equal"]],
];
/** The boxes the LAD compiler supports (ADR-0051); signatures come from the runtime. */
export const LAD_BLOCKS: readonly BlockKind[] = [
  "TON",
  "TOF",
  "TP",
  "CTU",
  "CTD",
];

export function instructions(
  standard: readonly StandardBlockDto[],
): Instruction[] {
  const blocks = LAD_BLOCKS.map((block): Instruction => {
    const signature = standard.find((b) => b.name === block);
    return {
      name: block,
      item: { type: "block", block },
      aliases: isTimer(block) ? ["timer"] : ["counter"],
      pins: (signature?.parameters ?? []).map((p) => ({
        name: p.name,
        type: p.dataType,
        direction: p.direction === "Output" ? "out" : "in",
      })),
    };
  });
  return [
    ...CONTACT_ITEMS.map(([name, contact, aliases]): Instruction => ({
      name,
      item: { type: "contact", contact },
      aliases,
      pins: [{ name: "operand", type: "BOOL", direction: "in" }],
    })),
    ...blocks,
    ...COMPARES.map(([name, op, aliases]): Instruction => ({
      name,
      item: { type: "compare", op },
      aliases,
      pins: [
        { name: "IN1", type: "INT | REAL | TIME", direction: "in" },
        { name: "IN2", type: "INT | REAL | TIME", direction: "in" },
      ],
    })),
    ...COIL_ITEMS.map(([name, kind, aliases]): Instruction => ({
      name,
      item: { type: "coil", kind },
      aliases,
      pins: [{ name: "operand", type: "BOOL", direction: "out" }],
    })),
  ];
}

/** Instructions matching `query`, best first; `describe` gives the localized text. */
export function matchInstructions(
  list: readonly Instruction[],
  query: string,
  describe: (i: Instruction) => string,
): Instruction[] {
  const q = query.trim();
  if (!q) return [...list];
  return list
    .map((i) => {
      const own = score(i.name, q);
      const s = Math.max(
        own === null ? -1 : own + 20,
        ...i.aliases.map((a) => score(a, q) ?? -1),
        describe(i).toLowerCase().includes(q.toLowerCase()) ? 10 : -1,
      );
      return { i, s };
    })
    .filter((x) => x.s >= 0)
    .sort((a, b) => b.s - a.s)
    .map((x) => x.i);
}

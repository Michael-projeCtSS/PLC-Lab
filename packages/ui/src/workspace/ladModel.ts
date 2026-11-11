/**
 * The ladder document edited in the LAD editor (ADR-0050, ADR-0051), the same JSON the
 * Rust LAD frontend compiles. Edits are pure functions that return a new document and
 * keep every network in the shape the compiler accepts: a parallel block has at least
 * two branches, only the rung's whole network may be an empty series, and element ids
 * are persistent (new elements take the next free id, existing ids never change).
 *
 * Nothing here evaluates the ladder: power flow, coil states, timer and counter values
 * come from the runtime (`ladder_state`).
 */
import type { InterfaceSignalDto } from "@plc-lab/ipc";

export const LAD_FORMAT = "plclab-lad";
/** The version written; version 1 documents are read and upgraded on the next edit. */
export const LAD_VERSION = 2;
export type Direction = "input" | "output" | "local";
export type LadType = "BOOL" | "INT" | "REAL" | "TIME";
export const LAD_TYPES: readonly LadType[] = ["BOOL", "INT", "REAL", "TIME"];
export type ContactKind =
  "normallyOpen" | "normallyClosed" | "positiveEdge" | "negativeEdge";
export type CompareOp = "eq" | "ne" | "gt" | "ge" | "lt" | "le";
export type BlockKind = "TON" | "TOF" | "TP" | "CTU" | "CTD";
export type CoilKind = "output" | "set" | "reset";
/** A variable or a literal in IEC syntax (`TRUE`, `10`, `2.5`, `T#5s`). */
export type Operand = { variable: string } | { constant: string };
export type LadVariable = {
  name: string;
  type: LadType;
  direction: Direction;
  comment?: string;
};
export type LadNode =
  | { type: "contact"; id: number; contact: ContactKind; variable: string }
  | {
      type: "compare";
      id: number;
      op: CompareOp;
      left: Operand;
      right: Operand;
    }
  | {
      type: "block";
      id: number;
      block: BlockKind;
      instance: string;
      preset?: Operand;
      reset?: Operand;
      output?: string;
    }
  | { type: "series"; id: number; items: LadNode[] }
  | { type: "parallel"; id: number; branches: LadNode[] };
export type Coil = { id: number; variable: string; kind?: "set" | "reset" };
export type Rung = {
  id: number;
  comment?: string;
  network: LadNode;
  coils: Coil[];
};
export type LadDocument = {
  format: typeof LAD_FORMAT;
  version: 1 | 2;
  name: string;
  variables: LadVariable[];
  rungs: Rung[];
};

/** What the palette inserts. */
export type ElementSpec =
  | { type: "contact"; contact: ContactKind }
  | { type: "compare"; op: CompareOp }
  | { type: "block"; block: BlockKind };

export const isTimer = (kind: BlockKind) =>
  kind === "TON" || kind === "TOF" || kind === "TP";

/** Compiler-owned power and edge symbols (`LAD#12`, `EDGE#12`): hidden from students. */
export function isSynthetic(name: string): boolean {
  return /^(LAD|EDGE)#\d+$/i.test(name);
}

/** The document as it is saved: stable, readable JSON of the current version. */
export function serializeLad(document: LadDocument): string {
  return `${JSON.stringify({ ...document, version: LAD_VERSION }, null, 2)}\n`;
}

/**
 * The ladder document in `text`, or why it is not one. Only the shape the editor needs
 * is checked here; the backend compiler validates the document completely.
 */
export function parseLad(
  text: string,
): { document: LadDocument } | { error: string } {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
  const d = value as Partial<LadDocument> | null;
  if (
    !d ||
    typeof d !== "object" ||
    d.format !== LAD_FORMAT ||
    (d.version !== 1 && d.version !== 2) ||
    typeof d.name !== "string" ||
    !Array.isArray(d.variables) ||
    !Array.isArray(d.rungs) ||
    !d.rungs.every(
      (r) =>
        r &&
        typeof r.id === "number" &&
        Array.isArray(r.coils) &&
        validNode(r.network),
    )
  )
    return { error: "not a PLC Lab ladder document of version 1 or 2" };
  return { document: d as LadDocument };
}

function validNode(node: unknown): boolean {
  const n = node as LadNode | null;
  if (!n || typeof n !== "object" || typeof n.id !== "number") return false;
  switch (n.type) {
    case "contact":
      return typeof n.variable === "string";
    case "compare":
      return !!n.left && !!n.right && typeof n.op === "string";
    case "block":
      return typeof n.instance === "string" && typeof n.block === "string";
    case "series":
      return Array.isArray(n.items) && n.items.every(validNode);
    case "parallel":
      return Array.isArray(n.branches) && n.branches.every(validNode);
    default:
      return false;
  }
}

/** A new ladder program with the scene's signals declared and no rungs yet. */
export function newLadDocument(
  name: string,
  signals: readonly InterfaceSignalDto[],
): LadDocument {
  return {
    format: LAD_FORMAT,
    version: LAD_VERSION,
    name,
    variables: signals
      .filter((s) => (LAD_TYPES as readonly string[]).includes(s.dataType))
      .map((s) => ({
        name: s.tag,
        type: s.dataType as LadType,
        direction: s.direction === "output" ? "output" : "input",
        ...(s.description ? { comment: s.description } : {}),
      })),
    rungs: [],
  };
}

export const isLeaf = (n: LadNode) =>
  n.type === "contact" || n.type === "compare" || n.type === "block";
function children(node: LadNode): LadNode[] {
  if (node.type === "series") return node.items;
  if (node.type === "parallel") return node.branches;
  return [];
}
function ids(node: LadNode): number[] {
  return [node.id, ...children(node).flatMap(ids)];
}
/** Every element id of the document: rungs, network elements and coils. */
export function elementIds(document: LadDocument): number[] {
  return document.rungs.flatMap((r) => [
    r.id,
    ...ids(r.network),
    ...r.coils.map((c) => c.id),
  ]);
}
/** The element added last: the highest id. */
export function newestElement(document: LadDocument): number {
  return Math.max(0, ...elementIds(document));
}
function allocator(document: LadDocument) {
  let next = Math.max(0, ...elementIds(document)) + 1;
  return () => next++;
}
function instances(document: LadDocument): string[] {
  const all: string[] = [];
  const walk = (n: LadNode) => {
    if (n.type === "block") all.push(n.instance.toLowerCase());
    children(n).forEach(walk);
  };
  document.rungs.forEach((r) => walk(r.network));
  return all;
}
/** A free instance name: T1, T2, … for timers, C1, … for counters. */
export function nextInstance(document: LadDocument, kind: BlockKind): string {
  const taken = new Set([
    ...instances(document),
    ...document.variables.map((v) => v.name.toLowerCase()),
  ]);
  const prefix = isTimer(kind) ? "T" : "C";
  let n = 1;
  while (taken.has(`${prefix}${n}`.toLowerCase())) n += 1;
  return `${prefix}${n}`;
}

/** A new element of `spec` with id `id`; boxes get a free instance name and a preset. */
export function createElement(
  document: LadDocument,
  spec: ElementSpec | ContactKind,
  id: number,
): LadNode {
  const s: ElementSpec =
    typeof spec === "string" ? { type: "contact", contact: spec } : spec;
  if (s.type === "contact")
    return { type: "contact", id, contact: s.contact, variable: "" };
  if (s.type === "compare")
    return {
      type: "compare",
      id,
      op: s.op,
      left: { variable: "" },
      right: { constant: "0" },
    };
  return {
    type: "block",
    id,
    block: s.block,
    instance: nextInstance(document, s.block),
    preset: { constant: isTimer(s.block) ? "T#1s" : "3" },
  };
}

/** Add a rung with one empty contact and one empty coil at the end (or after a rung). */
export function addRung(document: LadDocument, after?: number): LadDocument {
  const id = allocator(document);
  const rung = id();
  const series = id();
  const added: Rung = {
    id: rung,
    network: {
      type: "series",
      id: series,
      items: [createElement(document, "normallyOpen", id())],
    },
    coils: [{ id: id(), variable: "" }],
  };
  const at = document.rungs.findIndex((r) => r.id === after);
  const rungs = [...document.rungs];
  rungs.splice(at < 0 ? rungs.length : at + 1, 0, added);
  return { ...document, rungs };
}

export function removeRung(document: LadDocument, rungId: number): LadDocument {
  return { ...document, rungs: document.rungs.filter((r) => r.id !== rungId) };
}

export function setRungComment(
  document: LadDocument,
  rungId: number,
  comment: string,
): LadDocument {
  return {
    ...document,
    rungs: document.rungs.map((r) => {
      if (r.id !== rungId) return r;
      const { comment: _old, ...rest } = r;
      void _old;
      return comment.trim() ? { ...rest, comment } : rest;
    }),
  };
}

/** Move a rung up (-1) or down (+1); at the ends nothing changes. */
export function moveRung(
  document: LadDocument,
  rungId: number,
  delta: -1 | 1,
): LadDocument {
  const at = document.rungs.findIndex((r) => r.id === rungId);
  const to = at + delta;
  if (at < 0 || to < 0 || to >= document.rungs.length) return document;
  const rungs = [...document.rungs];
  const [rung] = rungs.splice(at, 1);
  if (rung) rungs.splice(to, 0, rung);
  return { ...document, rungs };
}

/**
 * Insert a copy of `rung` after rung `after` (or at the end) with fresh element ids and,
 * for its boxes, fresh instance names: the copy never shares state with the original.
 */
export function pasteRung(
  document: LadDocument,
  rung: Rung,
  after?: number,
): LadDocument {
  const next = allocator(document);
  let working = document;
  const copy = (node: LadNode): LadNode => {
    const id = next();
    if (node.type === "series")
      return { ...node, id, items: node.items.map(copy) };
    if (node.type === "parallel")
      return { ...node, id, branches: node.branches.map(copy) };
    if (node.type === "block") {
      const instance = nextInstance(working, node.block);
      const renamed = { ...node, id, instance };
      // Reserve the name before the next box of the same copy takes one.
      working = {
        ...working,
        rungs: [...working.rungs, { id: 0, network: renamed, coils: [] }],
      };
      return renamed;
    }
    return { ...node, id };
  };
  const pasted: Rung = {
    ...rung,
    id: next(),
    network: copy(rung.network),
    coils: rung.coils.map((c) => ({ ...c, id: next() })),
  };
  const at = document.rungs.findIndex((r) => r.id === after);
  const rungs = [...document.rungs];
  rungs.splice(at < 0 ? rungs.length : at + 1, 0, pasted);
  return { ...document, rungs };
}

export function duplicateRung(
  document: LadDocument,
  rungId: number,
): LadDocument {
  const rung = document.rungs.find((r) => r.id === rungId);
  return rung ? pasteRung(document, rung, rungId) : document;
}

/** Rewrite the network node `id` (and only it) with `change`. */
function mapNode(
  node: LadNode,
  id: number,
  change: (node: LadNode) => LadNode,
): LadNode {
  if (node.id === id) return change(node);
  if (node.type === "series")
    return { ...node, items: node.items.map((n) => mapNode(n, id, change)) };
  if (node.type === "parallel")
    return {
      ...node,
      branches: node.branches.map((n) => mapNode(n, id, change)),
    };
  return node;
}
function mapNetworks(
  document: LadDocument,
  change: (network: LadNode) => LadNode,
): LadDocument {
  return {
    ...document,
    rungs: document.rungs.map((r) => ({ ...r, network: change(r.network) })),
  };
}

/** The node `id` in any rung. */
export function findNode(document: LadDocument, id: number): LadNode | null {
  const find = (node: LadNode): LadNode | null => {
    if (node.id === id) return node;
    for (const child of children(node)) {
      const found = find(child);
      if (found) return found;
    }
    return null;
  };
  for (const r of document.rungs) {
    const found = find(r.network);
    if (found) return found;
  }
  return null;
}
export function findCoil(document: LadDocument, id: number): Coil | null {
  for (const r of document.rungs)
    for (const c of r.coils) if (c.id === id) return c;
  return null;
}
/** The rung that holds element `id` (network element, coil or the rung itself). */
export function rungOf(document: LadDocument, id: number): Rung | null {
  return (
    document.rungs.find(
      (r) =>
        r.id === id ||
        ids(r.network).includes(id) ||
        r.coils.some((c) => c.id === id),
    ) ?? null
  );
}

/** The series that directly contains `id`, if any. */
function seriesOf(node: LadNode, id: number): LadNode | null {
  if (node.type === "series" && node.items.some((c) => c.id === id))
    return node;
  for (const child of children(node)) {
    const found = seriesOf(child, id);
    if (found) return found;
  }
  return null;
}
function parallelOf(node: LadNode, id: number): number | null {
  if (node.type === "parallel" && node.branches.some((c) => c.id === id))
    return node.id;
  for (const child of children(node)) {
    const found = parallelOf(child, id);
    if (found !== null) return found;
  }
  return null;
}

/**
 * A new element in series right after (or before) element `id`: inside the series that
 * holds it, or, for an element that is a parallel branch or the whole network, in a new
 * series with it. The added element is the newest element.
 */
export function insertSeries(
  document: LadDocument,
  id: number,
  spec: ElementSpec | ContactKind,
  where: "after" | "before" = "after",
): LadDocument {
  const next = allocator(document);
  // A wrapping series takes its id first: the added element is the newest.
  const wrapper = next();
  const added = createElement(document, spec, next());
  return mapNetworks(document, (network) => {
    if (network.id === id && network.type === "series" && !network.items.length)
      return { ...network, items: [added] };
    const parent = seriesOf(network, id);
    if (parent && parent.type === "series")
      return mapNode(network, parent.id, (s) => {
        if (s.type !== "series") return s;
        const at =
          s.items.findIndex((c) => c.id === id) + (where === "after" ? 1 : 0);
        return {
          ...s,
          items: [...s.items.slice(0, at), added, ...s.items.slice(at)],
        };
      });
    return mapNode(network, id, (node) => ({
      type: "series",
      id: wrapper,
      items: where === "after" ? [node, added] : [added, node],
    }));
  });
}

/**
 * A new element in parallel to element `id`: one more branch of the parallel block that
 * holds it as a branch, or a new parallel block of the element and the new one.
 */
export function insertParallel(
  document: LadDocument,
  id: number,
  spec: ElementSpec | ContactKind,
): LadDocument {
  const next = allocator(document);
  // A wrapping parallel block takes its id first: the added element is the newest.
  const wrapper = next();
  const added = createElement(document, spec, next());
  return mapNetworks(document, (network) => {
    const holder = parallelOf(network, id);
    if (holder)
      return mapNode(network, holder, (p) =>
        p.type === "parallel" ? { ...p, branches: [...p.branches, added] } : p,
      );
    // The whole network of an empty rung is replaced by the element itself.
    if (network.id === id && network.type === "series" && !network.items.length)
      return { ...network, items: [added] };
    return mapNode(network, id, (node) => ({
      type: "parallel",
      id: wrapper,
      branches: [node, added],
    }));
  });
}

/** Move element `id` one place left (-1) or right (+1) inside its series. */
export function moveElement(
  document: LadDocument,
  id: number,
  delta: -1 | 1,
): LadDocument {
  return mapNetworks(document, (network) => {
    const parent = seriesOf(network, id);
    if (!parent || parent.type !== "series") return network;
    return mapNode(network, parent.id, (s) => {
      if (s.type !== "series") return s;
      const at = s.items.findIndex((c) => c.id === id);
      const to = at + delta;
      if (to < 0 || to >= s.items.length) return s;
      const items = [...s.items];
      const [item] = items.splice(at, 1);
      if (item) items.splice(to, 0, item);
      return { ...s, items };
    });
  });
}

/** Remove network element `id` and keep the network valid. */
export function removeElement(document: LadDocument, id: number): LadDocument {
  return mapNetworks(document, (network) => {
    if (network.id === id) return { type: "series", id, items: [] };
    return normalize(prune(network, id), true);
  });
}
function prune(node: LadNode, id: number): LadNode {
  if (node.type === "series")
    return {
      ...node,
      items: node.items.filter((c) => c.id !== id).map((c) => prune(c, id)),
    };
  if (node.type === "parallel")
    return {
      ...node,
      branches: node.branches
        .filter((c) => c.id !== id)
        .map((c) => prune(c, id)),
    };
  return node;
}
/** One-child blocks collapse into their child; only the root may stay an empty series. */
function normalize(node: LadNode, root: boolean): LadNode {
  if (isLeaf(node)) return node;
  if (node.type === "series") {
    const items = node.items
      .map((n) => normalize(n, false))
      .filter((n) => !(n.type === "series" && n.items.length === 0));
    const only = items[0];
    if (!root && items.length === 1 && only) return only;
    return { ...node, items };
  }
  if (node.type !== "parallel") return node;
  const branches = node.branches
    .map((n) => normalize(n, false))
    .filter((n) => !(n.type === "series" && n.items.length === 0));
  const only = branches[0];
  if (branches.length === 1 && only)
    return root ? { type: "series", id: node.id, items: [only] } : only;
  if (branches.length === 0) return { type: "series", id: node.id, items: [] };
  return { ...node, branches };
}

/** Change the leaf element `id`; the change cannot alter its kind or id. */
export function updateElement(
  document: LadDocument,
  id: number,
  change: (node: LadNode) => LadNode,
): LadDocument {
  return mapNetworks(document, (network) =>
    mapNode(network, id, (n) => {
      const next = change(n);
      return next.type === n.type ? { ...next, id: n.id } : n;
    }),
  );
}

/**
 * Set or clear an optional pin of the box `id`: the reset input R/LD (`null` clears it,
 * like an unwired input) or the ET/CV output variable (`""` clears it).
 */
export function setBlockPin(
  document: LadDocument,
  id: number,
  pin: "reset" | "output",
  value: Operand | string | null,
): LadDocument {
  return updateElement(document, id, (n) => {
    if (n.type !== "block") return n;
    const { reset, output, ...rest } = n;
    const next = {
      ...rest,
      ...(pin === "reset"
        ? value && typeof value !== "string"
          ? { reset: value }
          : {}
        : reset
          ? { reset }
          : {}),
      ...(pin === "output"
        ? typeof value === "string" && value
          ? { output: value }
          : {}
        : output
          ? { output }
          : {}),
    };
    return next;
  });
}

export function setContactVariable(
  document: LadDocument,
  id: number,
  variable: string,
): LadDocument {
  return updateElement(document, id, (n) =>
    n.type === "contact" ? { ...n, variable } : n,
  );
}

export function setContactKind(
  document: LadDocument,
  id: number,
  contact: ContactKind,
): LadDocument {
  return updateElement(document, id, (n) =>
    n.type === "contact" ? { ...n, contact } : n,
  );
}

/** Normally open ↔ normally closed (an edge contact becomes normally open). */
export function toggleContact(document: LadDocument, id: number): LadDocument {
  return updateElement(document, id, (n) =>
    n.type === "contact"
      ? {
          ...n,
          contact:
            n.contact === "normallyOpen" ? "normallyClosed" : "normallyOpen",
        }
      : n,
  );
}

export function addCoil(
  document: LadDocument,
  rungId: number,
  kind: CoilKind = "output",
): LadDocument {
  const next = allocator(document);
  const coil: Coil =
    kind === "output"
      ? { id: next(), variable: "" }
      : { id: next(), variable: "", kind };
  return {
    ...document,
    rungs: document.rungs.map((r) =>
      r.id === rungId ? { ...r, coils: [...r.coils, coil] } : r,
    ),
  };
}

export function setCoil(
  document: LadDocument,
  coilId: number,
  change: { variable?: string; kind?: CoilKind },
): LadDocument {
  return {
    ...document,
    rungs: document.rungs.map((r) => ({
      ...r,
      coils: r.coils.map((c) => {
        if (c.id !== coilId) return c;
        const variable = change.variable ?? c.variable;
        const kind = change.kind ?? c.kind ?? "output";
        return kind === "output"
          ? { id: c.id, variable }
          : { id: c.id, variable, kind };
      }),
    })),
  };
}

export function setCoilVariable(
  document: LadDocument,
  coilId: number,
  variable: string,
): LadDocument {
  return setCoil(document, coilId, { variable });
}

export function removeCoil(document: LadDocument, coilId: number): LadDocument {
  return {
    ...document,
    rungs: document.rungs.map((r) => ({
      ...r,
      coils: r.coils.filter((c) => c.id !== coilId),
    })),
  };
}

/**
 * The selectable elements of each rung in reading order (left to right, top branch
 * first; coils last), for keyboard navigation.
 */
export function readingOrder(document: LadDocument): number[][] {
  const walk = (node: LadNode): number[] => {
    if (isLeaf(node)) return [node.id];
    if (node.type === "series")
      return node.items.length ? node.items.flatMap(walk) : [node.id];
    return [node.id, ...children(node).flatMap(walk)];
  };
  return document.rungs.map((r) => [
    ...walk(r.network),
    ...r.coils.map((c) => c.id),
  ]);
}

/** Declare a variable; `null` when the name is not an identifier or is taken. */
export function addVariable(
  document: LadDocument,
  name: string,
  direction: Direction,
  type: LadType = "BOOL",
): LadDocument | null {
  const trimmed = name.trim();
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(trimmed)) return null;
  const taken = new Set([
    ...document.variables.map((v) => v.name.toLowerCase()),
    ...instances(document),
  ]);
  if (taken.has(trimmed.toLowerCase())) return null;
  return {
    ...document,
    variables: [...document.variables, { name: trimmed, type, direction }],
  };
}

export function removeVariable(
  document: LadDocument,
  name: string,
): LadDocument {
  return {
    ...document,
    variables: document.variables.filter((v) => v.name !== name),
  };
}

export function setDirection(
  document: LadDocument,
  name: string,
  direction: Direction,
): LadDocument {
  return {
    ...document,
    variables: document.variables.map((v) =>
      v.name === name ? { ...v, direction } : v,
    ),
  };
}

export function setVariableType(
  document: LadDocument,
  name: string,
  type: LadType,
): LadDocument {
  return {
    ...document,
    variables: document.variables.map((v) =>
      v.name === name ? { ...v, type } : v,
    ),
  };
}

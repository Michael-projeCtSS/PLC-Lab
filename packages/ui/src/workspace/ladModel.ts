/**
 * The ladder document edited in the LAD editor (ADR-0050), the same JSON the Rust LAD
 * frontend compiles. Edits are pure functions that return a new document and keep every
 * network in the shape the compiler accepts: a parallel block has at least two branches,
 * only the rung's whole network may be an empty series, and element ids are persistent
 * (new elements take the next free id, existing ids never change).
 *
 * Nothing here evaluates the ladder: power flow and coil values come from the runtime.
 */
import type { InterfaceSignalDto } from "@plc-lab/ipc";

export const LAD_FORMAT = "plclab-lad";
export type Direction = "input" | "output" | "local";
export type ContactKind = "normallyOpen" | "normallyClosed";
export type LadVariable = {
  name: string;
  type: "BOOL";
  direction: Direction;
  comment?: string;
};
export type LadNode =
  | { type: "contact"; id: number; contact: ContactKind; variable: string }
  | { type: "series"; id: number; items: LadNode[] }
  | { type: "parallel"; id: number; branches: LadNode[] };
export type Coil = { id: number; variable: string };
export type Rung = {
  id: number;
  comment?: string;
  network: LadNode;
  coils: Coil[];
};
export type LadDocument = {
  format: typeof LAD_FORMAT;
  version: 1;
  name: string;
  variables: LadVariable[];
  rungs: Rung[];
};

/** The document as it is saved: stable, readable JSON. */
export function serializeLad(document: LadDocument): string {
  return `${JSON.stringify(document, null, 2)}\n`;
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
    d.version !== 1 ||
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
    return { error: "not a PLC Lab ladder document of version 1" };
  return { document: d as LadDocument };
}

function validNode(node: unknown): boolean {
  const n = node as LadNode | null;
  if (!n || typeof n !== "object" || typeof n.id !== "number") return false;
  if (n.type === "contact") return typeof n.variable === "string";
  if (n.type === "series") return n.items.every(validNode);
  if (n.type === "parallel") return n.branches.every(validNode);
  return false;
}

/** A new ladder program with the scene's BOOL signals declared and no rungs yet. */
export function newLadDocument(
  name: string,
  signals: readonly InterfaceSignalDto[],
): LadDocument {
  return {
    format: LAD_FORMAT,
    version: 1,
    name,
    variables: signals
      .filter((s) => s.dataType === "BOOL")
      .map((s) => ({
        name: s.tag,
        type: "BOOL" as const,
        direction: s.direction === "output" ? "output" : "input",
        ...(s.description ? { comment: s.description } : {}),
      })),
    rungs: [],
  };
}

function ids(node: LadNode): number[] {
  if (node.type === "contact") return [node.id];
  const children = node.type === "series" ? node.items : node.branches;
  return [node.id, ...children.flatMap(ids)];
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
function contact(id: number, kind: ContactKind, variable = ""): LadNode {
  return { type: "contact", id, contact: kind, variable };
}

/** Add a rung with one empty contact and one empty coil at the end. */
export function addRung(document: LadDocument): LadDocument {
  const id = allocator(document);
  const rung = id();
  const series = id();
  return {
    ...document,
    rungs: [
      ...document.rungs,
      {
        id: rung,
        network: {
          type: "series",
          id: series,
          items: [contact(id(), "normallyOpen")],
        },
        coils: [{ id: id(), variable: "" }],
      },
    ],
  };
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

/** The series that directly contains `id`, if any. */
function seriesOf(node: LadNode, id: number): LadNode | null {
  if (node.type === "contact") return null;
  const children = node.type === "series" ? node.items : node.branches;
  if (node.type === "series" && children.some((c) => c.id === id)) return node;
  for (const child of children) {
    const found = seriesOf(child, id);
    if (found) return found;
  }
  return null;
}

/**
 * A new contact in series right after element `id`: inside the series that holds it,
 * or, for an element that is a parallel branch or the whole network, in a new series
 * with it.
 */
export function insertSeries(
  document: LadDocument,
  id: number,
  kind: ContactKind,
): LadDocument {
  const next = allocator(document);
  // A wrapping series takes its id first: the added contact is the newest element.
  const wrapper = next();
  const added = contact(next(), kind);
  return mapNetworks(document, (network) => {
    const parent = seriesOf(network, id);
    if (parent && parent.type === "series")
      return mapNode(network, parent.id, (s) => {
        if (s.type !== "series") return s;
        const at = s.items.findIndex((c) => c.id === id);
        return {
          ...s,
          items: [...s.items.slice(0, at + 1), added, ...s.items.slice(at + 1)],
        };
      });
    return mapNode(network, id, (node) => ({
      type: "series",
      id: wrapper,
      items: [node, added],
    }));
  });
}

/**
 * A new contact in parallel to element `id`: one more branch of the parallel block
 * that holds it as a branch, or a new parallel block of the element and the contact.
 */
export function insertParallel(
  document: LadDocument,
  id: number,
  kind: ContactKind,
): LadDocument {
  const next = allocator(document);
  // A wrapping parallel block takes its id first: the added contact is the newest.
  const wrapper = next();
  const added = contact(next(), kind);
  return mapNetworks(document, (network) => {
    const holder = parallelOf(network, id);
    if (holder)
      return mapNode(network, holder, (p) =>
        p.type === "parallel" ? { ...p, branches: [...p.branches, added] } : p,
      );
    // The whole network of an empty rung is replaced by the contact itself.
    if (network.id === id && network.type === "series" && !network.items.length)
      return { ...network, items: [added] };
    return mapNode(network, id, (node) => ({
      type: "parallel",
      id: wrapper,
      branches: [node, added],
    }));
  });
}
function parallelOf(node: LadNode, id: number): number | null {
  if (node.type === "contact") return null;
  const children = node.type === "series" ? node.items : node.branches;
  if (node.type === "parallel" && children.some((c) => c.id === id))
    return node.id;
  for (const child of children) {
    const found = parallelOf(child, id);
    if (found !== null) return found;
  }
  return null;
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
  if (node.type === "contact") return node;
  if (node.type === "series") {
    const items = node.items
      .map((n) => normalize(n, false))
      .filter((n) => !(n.type === "series" && n.items.length === 0));
    const only = items[0];
    if (!root && items.length === 1 && only) return only;
    return { ...node, items };
  }
  const branches = node.branches
    .map((n) => normalize(n, false))
    .filter((n) => !(n.type === "series" && n.items.length === 0));
  const only = branches[0];
  if (branches.length === 1 && only)
    return root ? { type: "series", id: node.id, items: [only] } : only;
  if (branches.length === 0) return { type: "series", id: node.id, items: [] };
  return { ...node, branches };
}

export function setContactVariable(
  document: LadDocument,
  id: number,
  variable: string,
): LadDocument {
  return mapNetworks(document, (network) =>
    mapNode(network, id, (n) =>
      n.type === "contact" ? { ...n, variable } : n,
    ),
  );
}

export function toggleContact(document: LadDocument, id: number): LadDocument {
  return mapNetworks(document, (network) =>
    mapNode(network, id, (n) =>
      n.type === "contact"
        ? {
            ...n,
            contact:
              n.contact === "normallyOpen" ? "normallyClosed" : "normallyOpen",
          }
        : n,
    ),
  );
}

export function addCoil(document: LadDocument, rungId: number): LadDocument {
  const next = allocator(document);
  return {
    ...document,
    rungs: document.rungs.map((r) =>
      r.id === rungId
        ? { ...r, coils: [...r.coils, { id: next(), variable: "" }] }
        : r,
    ),
  };
}

export function setCoilVariable(
  document: LadDocument,
  coilId: number,
  variable: string,
): LadDocument {
  return {
    ...document,
    rungs: document.rungs.map((r) => ({
      ...r,
      coils: r.coils.map((c) => (c.id === coilId ? { ...c, variable } : c)),
    })),
  };
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

/** Declare a BOOL variable; `null` when the name is not an identifier or is taken. */
export function addVariable(
  document: LadDocument,
  name: string,
  direction: Direction,
): LadDocument | null {
  const trimmed = name.trim();
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(trimmed)) return null;
  if (
    document.variables.some(
      (v) => v.name.toLowerCase() === trimmed.toLowerCase(),
    )
  )
    return null;
  return {
    ...document,
    variables: [
      ...document.variables,
      { name: trimmed, type: "BOOL", direction },
    ],
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

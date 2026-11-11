/**
 * The placement engine of the ladder editor (ADR-0052): where a dragged, pasted or moved
 * element goes is a semantic drop target — "in series after element 4", "a new OR branch
 * beside element 4", "a new branch of parallel block 7" — never a screen position. Every
 * operation is one pure transformation of the ladder document; an operation that cannot
 * be applied returns a reason and no document, so nothing changes.
 */
import {
  createElement,
  elementIds,
  findCoil,
  findNode,
  isLeaf,
  nextInstance,
  type BlockKind,
  type Coil,
  type CoilKind,
  type CompareOp,
  type ContactKind,
  type ElementSpec,
  type LadDocument,
  type LadNode,
  type Rung,
} from "../ladModel";

/** What the palette offers: network elements and coils. */
export type PaletteItem =
  | { type: "contact"; contact: ContactKind }
  | { type: "compare"; op: CompareOp }
  | { type: "block"; block: BlockKind }
  | { type: "coil"; kind: CoilKind };

/** What is being placed. */
export type Source =
  | { kind: "new"; item: PaletteItem }
  | { kind: "move"; id: number; copy: boolean }
  | { kind: "paste"; node: LadNode }
  | { kind: "pasteCoil"; coil: Coil };

/**
 * Where it goes, relative to the ladder tree:
 * - `before`/`after` element `id`: in series (AND) with it;
 * - `below` element `id`: a new branch in parallel (OR) with exactly that element; for the
 *   rung's whole network, in parallel with all of it;
 * - `branch` of parallel block `id`: one more branch of that block;
 * - `start` of rung `rung`: the empty network of that rung;
 * - `coil` position `index` of rung `rung`.
 */
export type Target =
  | { zone: "before" | "after" | "below"; id: number }
  | { zone: "branch"; id: number }
  | { zone: "start"; rung: number }
  | { zone: "coil"; rung: number; index: number };

export type DropError =
  "coilInNetwork" | "elementInCoils" | "intoItself" | "unknown" | "noChange";

export type DropResult =
  { document: LadDocument; selected: number } | { error: DropError };

export const isCoilItem = (item: PaletteItem) => item.type === "coil";

/** Is `source` a coil (it belongs in a coil list, not in a network)? */
export function placesCoil(document: LadDocument, source: Source): boolean {
  switch (source.kind) {
    case "new":
      return isCoilItem(source.item);
    case "move":
      return !!findCoil(document, source.id);
    case "paste":
      return false;
    case "pasteCoil":
      return true;
  }
}

function specOf(item: PaletteItem): ElementSpec {
  if (item.type === "contact")
    return { type: "contact", contact: item.contact };
  if (item.type === "compare") return { type: "compare", op: item.op };
  if (item.type === "block") return { type: "block", block: item.block };
  throw new Error("a coil is not a network element");
}

const children = (n: LadNode): LadNode[] =>
  n.type === "series" ? n.items : n.type === "parallel" ? n.branches : [];
const subtreeIds = (n: LadNode): number[] => [
  n.id,
  ...children(n).flatMap(subtreeIds),
];

function allocator(document: LadDocument, extra: number[] = []) {
  let next = Math.max(0, ...elementIds(document), ...extra) + 1;
  return () => next++;
}

/**
 * A copy of `node` with fresh ids and, for boxes, fresh instance names, so that it never
 * shares an instance (its timer or counter state) with the original.
 */
export function freshCopy(document: LadDocument, node: LadNode): LadNode {
  const next = allocator(document);
  let working = document;
  const copy = (n: LadNode): LadNode => {
    const id = next();
    if (n.type === "series") return { ...n, id, items: n.items.map(copy) };
    if (n.type === "parallel")
      return { ...n, id, branches: n.branches.map(copy) };
    if (n.type === "block") {
      const renamed = { ...n, id, instance: nextInstance(working, n.block) };
      working = {
        ...working,
        rungs: [...working.rungs, { id: 0, network: renamed, coils: [] }],
      };
      return renamed;
    }
    return { ...n, id };
  };
  return copy(node);
}

/** Apply `source` at `target`: the new document and the element to select, or why not. */
export function applyDrop(
  document: LadDocument,
  source: Source,
  target: Target,
): DropResult {
  const coil = placesCoil(document, source);
  if (coil && target.zone !== "coil") return { error: "coilInNetwork" };
  if (!coil && target.zone === "coil") return { error: "elementInCoils" };
  const result =
    target.zone === "coil"
      ? dropCoil(document, source, target)
      : dropNode(document, source, target);
  if ("error" in result) return result;
  return JSON.stringify(result.document) === JSON.stringify(document)
    ? { error: "noChange" }
    : result;
}

function dropCoil(
  document: LadDocument,
  source: Source,
  target: { rung: number; index: number },
): DropResult {
  if (!document.rungs.some((r) => r.id === target.rung))
    return { error: "unknown" };
  let coil: Coil;
  let working = document;
  let index = target.index;
  if (source.kind === "new" && source.item.type === "coil") {
    const id = allocator(document)();
    coil =
      source.item.kind === "output"
        ? { id, variable: "" }
        : { id, variable: "", kind: source.item.kind };
  } else if (source.kind === "pasteCoil") {
    coil = { ...source.coil, id: allocator(document)() };
  } else if (source.kind === "move") {
    const original = findCoil(document, source.id);
    if (!original) return { error: "unknown" };
    if (source.copy) coil = { ...original, id: allocator(document)() };
    else {
      coil = original;
      const from = document.rungs.find((r) =>
        r.coils.some((c) => c.id === original.id),
      );
      const at = from?.coils.findIndex((c) => c.id === original.id) ?? -1;
      // Taking the coil out shifts the later positions of its own rung.
      if (from?.id === target.rung && at < index) index -= 1;
      working = {
        ...document,
        rungs: document.rungs.map((r) => ({
          ...r,
          coils: r.coils.filter((c) => c.id !== original.id),
        })),
      };
    }
  } else return { error: "unknown" };
  return {
    document: {
      ...working,
      rungs: working.rungs.map((r) => {
        if (r.id !== target.rung) return r;
        const coils = [...r.coils];
        coils.splice(Math.max(0, Math.min(index, coils.length)), 0, coil);
        return { ...r, coils };
      }),
    },
    selected: coil.id,
  };
}

function dropNode(
  document: LadDocument,
  source: Source,
  target: Exclude<Target, { zone: "coil" }>,
): DropResult {
  let working = document;
  let node: LadNode;
  if (source.kind === "new") {
    node = createElement(document, specOf(source.item), allocator(document)());
  } else if (source.kind === "paste") {
    node = freshCopy(document, source.node);
  } else if (source.kind === "move") {
    const original = findNode(document, source.id);
    if (!original || document.rungs.some((r) => r.network.id === source.id))
      return { error: "unknown" };
    const inside = subtreeIds(original);
    const anchor = "id" in target ? target.id : null;
    if (anchor === source.id) return { error: "noChange" };
    if (anchor !== null && inside.includes(anchor))
      return { error: "intoItself" };
    if (source.copy) node = freshCopy(document, original);
    else {
      node = original;
      // Detach without tidying: the target's container still exists for the insert.
      working = {
        ...document,
        rungs: document.rungs.map((r) =>
          contains(r.network, source.id)
            ? { ...r, network: detach(r.network, source.id) }
            : r,
        ),
      };
    }
  } else return { error: "unknown" };
  const next = allocator(working, subtreeIds(node));
  const inserted = insert(working, node, target, next);
  if (!inserted) return { error: "unknown" };
  // Only the rungs this operation changed are tidied; other rungs stay byte-identical.
  return {
    document: {
      ...inserted,
      rungs: inserted.rungs.map((r, i) =>
        r === document.rungs[i]
          ? r
          : { ...r, network: tidyRoot(r.network, next) },
      ),
    },
    selected: node.id,
  };
}

function detach(node: LadNode, id: number): LadNode {
  if (node.type === "series")
    return {
      ...node,
      items: node.items.filter((c) => c.id !== id).map((c) => detach(c, id)),
    };
  if (node.type === "parallel")
    return {
      ...node,
      branches: node.branches
        .filter((c) => c.id !== id)
        .map((c) => detach(c, id)),
    };
  return node;
}

function contains(node: LadNode, id: number): boolean {
  return node.id === id || children(node).some((c) => contains(c, id));
}

/** Insert `node` at `target`; null when the target does not exist. */
function insert(
  document: LadDocument,
  node: LadNode,
  target: Exclude<Target, { zone: "coil" }>,
  next: () => number,
): LadDocument | null {
  if (target.zone === "start") {
    const rung = document.rungs.find((r) => r.id === target.rung);
    if (!rung) return null;
    return replaceRung(document, {
      ...rung,
      network:
        rung.network.type === "series"
          ? { ...rung.network, items: [...rung.network.items, node] }
          : { type: "series", id: next(), items: [rung.network, node] },
    });
  }
  const rung = document.rungs.find((r) => contains(r.network, target.id));
  if (!rung) return null;
  const root = rung.network;
  if (target.zone === "branch") {
    const changed = mapNode(root, target.id, (p) =>
      p.type === "parallel" ? { ...p, branches: [...p.branches, node] } : p,
    );
    return changed === root
      ? null
      : replaceRung(document, { ...rung, network: changed });
  }
  if (target.zone === "below" && target.id === root.id) {
    // In parallel with the whole network.
    const items = root.type === "series" ? root.items : [root];
    const whole: LadNode | undefined =
      items.length === 1 ? items[0] : { type: "series", id: next(), items };
    const network: LadNode = whole
      ? {
          type: "series",
          id: root.type === "series" ? root.id : next(),
          items: [{ type: "parallel", id: next(), branches: [whole, node] }],
        }
      : { type: "series", id: root.id, items: [node] };
    return replaceRung(document, { ...rung, network });
  }
  const parent = parentOf(root, target.id);
  let network: LadNode;
  if (target.zone === "below") {
    network =
      parent?.type === "parallel"
        ? // A branch of a parallel block: one more branch right below it.
          mapNode(root, parent.id, (p) =>
            p.type === "parallel"
              ? { ...p, branches: spliceAfter(p.branches, target.id, node) }
              : p,
          )
        : // Anything else: a new parallel block of exactly this element and the new one.
          mapNode(root, target.id, (x) => ({
            type: "parallel",
            id: next(),
            branches: [x, node],
          }));
  } else {
    const after = target.zone === "after";
    const self = findIn(root, target.id);
    if (self?.type === "series")
      network = mapNode(root, self.id, (s) =>
        s.type === "series"
          ? { ...s, items: after ? [...s.items, node] : [node, ...s.items] }
          : s,
      );
    else if (parent?.type === "series")
      network = mapNode(root, parent.id, (s) =>
        s.type === "series"
          ? {
              ...s,
              items: after
                ? spliceAfter(s.items, target.id, node)
                : spliceBefore(s.items, target.id, node),
            }
          : s,
      );
    else
      network = mapNode(root, target.id, (x) => ({
        type: "series",
        id: next(),
        items: after ? [x, node] : [node, x],
      }));
  }
  return replaceRung(document, { ...rung, network });
}

const spliceAfter = (list: LadNode[], id: number, node: LadNode) => {
  const at = list.findIndex((c) => c.id === id);
  return [...list.slice(0, at + 1), node, ...list.slice(at + 1)];
};
const spliceBefore = (list: LadNode[], id: number, node: LadNode) => {
  const at = list.findIndex((c) => c.id === id);
  return [...list.slice(0, at), node, ...list.slice(at)];
};

function replaceRung(document: LadDocument, rung: Rung): LadDocument {
  return {
    ...document,
    rungs: document.rungs.map((r) => (r.id === rung.id ? rung : r)),
  };
}
function mapNode(
  node: LadNode,
  id: number,
  change: (n: LadNode) => LadNode,
): LadNode {
  if (node.id === id) return change(node);
  if (node.type === "series") {
    const items = node.items.map((c) => mapNode(c, id, change));
    return items.every((c, i) => c === node.items[i])
      ? node
      : { ...node, items };
  }
  if (node.type === "parallel") {
    const branches = node.branches.map((c) => mapNode(c, id, change));
    return branches.every((c, i) => c === node.branches[i])
      ? node
      : { ...node, branches };
  }
  return node;
}
function findIn(node: LadNode, id: number): LadNode | null {
  if (node.id === id) return node;
  for (const c of children(node)) {
    const found = findIn(c, id);
    if (found) return found;
  }
  return null;
}
/** The container that holds `id` directly. */
export function parentOf(node: LadNode, id: number): LadNode | null {
  if (children(node).some((c) => c.id === id)) return node;
  for (const c of children(node)) {
    const found = parentOf(c, id);
    if (found) return found;
  }
  return null;
}

/**
 * Remove what carries no logic: empty series, containers with one child, a series inside
 * a series and a parallel block directly inside a parallel block (AND and OR are
 * associative). The rung's network stays a series. Leaves keep their ids.
 */
export function tidyRoot(network: LadNode, next: () => number): LadNode {
  const t = tidy(network);
  if (t === null) return { type: "series", id: network.id, items: [] };
  if (t.type === "series")
    return t.id === network.id ? t : { ...t, id: network.id };
  return {
    type: "series",
    id: network.type === "series" ? network.id : next(),
    items: [t],
  };
}
function tidy(node: LadNode): LadNode | null {
  if (isLeaf(node)) return node;
  if (node.type === "series") {
    const items = node.items
      .map(tidy)
      .filter((c): c is LadNode => c !== null)
      .flatMap((c) => (c.type === "series" ? c.items : [c]));
    if (items.length === 0) return null;
    if (items.length === 1 && items[0]) return items[0];
    return { ...node, items };
  }
  if (node.type !== "parallel") return node;
  const branches = node.branches
    .map(tidy)
    .filter((c): c is LadNode => c !== null)
    .flatMap((c) => (c.type === "parallel" ? c.branches : [c]));
  if (branches.length === 0) return null;
  if (branches.length === 1 && branches[0]) return branches[0];
  return { ...node, branches };
}

/** The boolean expression of a network, for previews and tests: `(A OR C) AND B`. */
export function expression(node: LadNode, nested = false): string {
  switch (node.type) {
    case "contact": {
      const v = node.variable || "?";
      if (node.contact === "normallyClosed") return `NOT ${v}`;
      if (node.contact === "positiveEdge") return `P(${v})`;
      if (node.contact === "negativeEdge") return `N(${v})`;
      return v;
    }
    case "compare":
      return `CMP(${node.op})`;
    case "block":
      return `${node.block}(${node.instance})`;
    case "series": {
      if (!node.items.length) return "TRUE";
      const text = node.items.map((c) => expression(c, true)).join(" AND ");
      return nested && node.items.length > 1 ? `(${text})` : text;
    }
    case "parallel": {
      const text = node.branches.map((c) => expression(c, true)).join(" OR ");
      return nested ? `(${text})` : text;
    }
  }
}

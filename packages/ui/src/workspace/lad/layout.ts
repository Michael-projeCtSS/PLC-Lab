/**
 * Layout of a rung, derived from the ladder tree alone (ADR-0052): element boxes, the
 * wires between them, insertion points on the wires and the drop zones that turn a
 * pointer position into a semantic placement target. Coordinates are logical pixels at
 * zoom 100 %; nothing here is saved, and the same document always gives the same layout.
 */
import type { Coil, LadDocument, LadNode, Rung } from "../ladModel";
import type { Field } from "./complete";
import type { Target } from "./placement";

export const G = {
  ROW: 72,
  /** The power line of a row, from the row's top. */
  LINE: 30,
  /** An element body starts TOP below its row's top and is BODY tall (one-row elements). */
  TOP: 6,
  BODY: 48,
  GAP: 30,
  JX: 18,
  RAIL: 18,
  BOTTOM: 30,
  CONTACT_W: 120,
  COMPARE_W: 136,
  BLOCK_W: 168,
  EMPTY_W: 150,
  COIL_W: 124,
  COIL_GAP: 44,
  MIN_COIL_X: 330,
} as const;

export type Box = { x: number; y: number; w: number; h: number };
/** The element whose outgoing power a wire carries, or the left rail. */
export type Feed = number | "rail";
export type Leaf = Extract<LadNode, { type: "contact" | "compare" | "block" }>;
export type Cell =
  | { kind: "node"; node: Leaf; box: Box; feed: Feed }
  | { kind: "empty"; id: number; box: Box }
  | { kind: "coil"; coil: Coil; index: number; box: Box };
export type Group = { id: number; box: Box; count: number };
export type Wire = {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  power: Feed;
};
/** A gap on a series wire: where the keyboard or a click places the insertion cursor. */
export type Point = { box: Box; target: Target; after: number | null };
export type Area = "network" | "coil";
export type Zone = { box: Box; target: Target; area: Area };
export type RungLayout = {
  rung: number;
  root: number;
  width: number;
  height: number;
  netEnd: number;
  netHeight: number;
  coilX: number;
  railRight: number;
  cells: Cell[];
  groups: Group[];
  wires: Wire[];
  points: Point[];
  /** In priority order: the first zone that contains a point decides. */
  zones: Zone[];
};

type Size = { w: number; h: number };

export function sizeOf(node: LadNode): Size {
  switch (node.type) {
    case "contact":
      return { w: G.CONTACT_W, h: G.ROW };
    case "compare":
      return { w: G.COMPARE_W, h: G.ROW };
    case "block":
      return { w: G.BLOCK_W, h: 2 * G.ROW };
    case "series": {
      if (!node.items.length) return { w: G.EMPTY_W, h: G.ROW };
      const sizes = node.items.map(sizeOf);
      return {
        w: G.GAP * (sizes.length + 1) + sizes.reduce((a, s) => a + s.w, 0),
        h: Math.max(...sizes.map((s) => s.h)),
      };
    }
    case "parallel": {
      const sizes = node.branches.map(sizeOf);
      return {
        w: 2 * G.JX + Math.max(...sizes.map((s) => s.w)),
        h: sizes.reduce((a, s) => a + s.h, 0),
      };
    }
  }
}

/** The x of the coil column, the same for every rung of the document. */
export function coilColumn(document: LadDocument): number {
  const widest = Math.max(
    0,
    ...document.rungs.map((r) => G.RAIL + sizeOf(r.network).w),
  );
  return Math.max(G.MIN_COIL_X, widest + G.COIL_GAP);
}

const lastLeaf = (node: LadNode): number =>
  node.type === "series" && node.items.length
    ? lastLeaf(node.items[node.items.length - 1] as LadNode)
    : node.id;

export function layoutRung(rung: Rung, coilX: number): RungLayout {
  const cells: Cell[] = [];
  const groups: Group[] = [];
  const wires: Wire[] = [];
  const points: Point[] = [];
  const leafZones: Zone[] = [];
  const groupZones: Zone[] = [];
  const rungZones: Zone[] = [];
  const wire = (x1: number, y1: number, x2: number, y2: number, power: Feed) =>
    wires.push({ x1, y1, x2, y2, power });
  const body = (y: number, h: number) => y + h - (G.ROW - G.TOP - G.BODY);

  const place = (node: LadNode, x: number, y: number, feed: Feed): Size => {
    const size = sizeOf(node);
    const line = y + G.LINE;
    switch (node.type) {
      case "contact":
      case "compare":
      case "block": {
        cells.push({ kind: "node", node, box: { x, y, ...size }, feed });
        const bottom = body(y, size.h);
        const half = size.w / 2;
        leafZones.push(
          {
            box: { x: x - G.GAP / 2, y, w: half + G.GAP / 2, h: bottom - y },
            target: { zone: "before", id: node.id },
            area: "network",
          },
          {
            box: { x: x + half, y, w: half + G.GAP / 2, h: bottom - y },
            target: { zone: "after", id: node.id },
            area: "network",
          },
          {
            box: { x, y: bottom, w: size.w, h: y + size.h - bottom + 8 },
            target: { zone: "below", id: node.id },
            area: "network",
          },
        );
        return size;
      }
      case "series": {
        if (!node.items.length) {
          cells.push({ kind: "empty", id: node.id, box: { x, y, ...size } });
          wire(x, line, x + size.w, line, feed);
          const target: Target = { zone: "start", rung: rung.id };
          points.push({
            box: { x, y: line - 11, w: size.w, h: 22 },
            target,
            after: null,
          });
          rungZones.push({
            box: { x, y, w: size.w, h: G.ROW },
            target,
            area: "network",
          });
          return size;
        }
        let cursor = x;
        let f = feed;
        node.items.forEach((item, i) => {
          wire(cursor, line, cursor + G.GAP, line, f);
          const previous = node.items[i - 1];
          points.push({
            box: { x: cursor, y: line - 11, w: G.GAP, h: 22 },
            target: previous
              ? { zone: "after", id: previous.id }
              : { zone: "before", id: item.id },
            after: previous?.id ?? null,
          });
          cursor += G.GAP;
          cursor += place(item, cursor, y, f).w;
          f = item.id;
        });
        const last = node.items[node.items.length - 1] as LadNode;
        wire(cursor, line, cursor + G.GAP, line, f);
        points.push({
          box: { x: cursor, y: line - 11, w: G.GAP, h: 22 },
          target: { zone: "after", id: last.id },
          after: last.id,
        });
        return size;
      }
      case "parallel": {
        const inner = size.w - 2 * G.JX;
        let by = y;
        let lastLine = line;
        for (const branch of node.branches) {
          const s = sizeOf(branch);
          const bl = by + G.LINE;
          wire(x, bl, x + G.JX, bl, feed);
          place(branch, x + G.JX, by, feed);
          if (s.w < inner) {
            wire(x + G.JX + s.w, bl, x + G.JX + inner, bl, branch.id);
            groupZones.push({
              box: {
                x: x + G.JX + s.w,
                y: by,
                w: inner - s.w,
                h: G.TOP + G.BODY,
              },
              target: { zone: "after", id: lastLeaf(branch) },
              area: "network",
            });
          }
          wire(x + G.JX + inner, bl, x + size.w, bl, branch.id);
          lastLine = bl;
          by += s.h;
        }
        wire(x, line, x, lastLine, feed);
        wire(x + size.w, line, x + size.w, lastLine, node.id);
        groups.push({
          id: node.id,
          box: { x, y, ...size },
          count: node.branches.length,
        });
        groupZones.push(
          {
            box: {
              x: x - G.GAP / 2,
              y,
              w: G.GAP / 2 + G.JX,
              h: G.TOP + G.BODY,
            },
            target: { zone: "before", id: node.id },
            area: "network",
          },
          {
            box: {
              x: x + size.w - G.JX,
              y,
              w: G.JX + G.GAP / 2,
              h: G.TOP + G.BODY,
            },
            target: { zone: "after", id: node.id },
            area: "network",
          },
          {
            box: { x, y: y + size.h - 24, w: size.w, h: 34 },
            target: { zone: "branch", id: node.id },
            area: "network",
          },
        );
        return size;
      }
    }
  };

  const root = rung.network;
  const net = place(root, G.RAIL, 0, "rail");
  const netEnd = G.RAIL + net.w;
  const items = root.type === "series" ? root.items : [root];
  const first = items[0];
  const last = items[items.length - 1];
  const strip = G.TOP + G.BODY;
  rungZones.push(
    {
      box: { x: 0, y: 0, w: G.RAIL + G.GAP / 2, h: strip },
      target: first
        ? { zone: "before", id: first.id }
        : { zone: "start", rung: rung.id },
      area: "network",
    },
    {
      box: {
        x: netEnd - G.GAP / 2,
        y: 0,
        w: coilX - G.JX - netEnd + G.GAP / 2,
        h: strip,
      },
      target: last
        ? { zone: "after", id: last.id }
        : { zone: "start", rung: rung.id },
      area: "network",
    },
    {
      box: { x: G.RAIL, y: net.h - 4, w: net.w, h: G.BOTTOM + 4 },
      target: first
        ? { zone: "below", id: root.id }
        : { zone: "start", rung: rung.id },
      area: "network",
    },
  );

  // Coils: one column, each on its own row, all fed by the rung's power.
  const coilZones: Zone[] = [];
  const line0 = G.LINE;
  const railRight = coilX + G.COIL_W + 24;
  const n = rung.coils.length;
  wire(netEnd, line0, n ? coilX - G.JX : railRight, line0, root.id);
  rung.coils.forEach((coil, index) => {
    const y = index * G.ROW;
    const line = y + G.LINE;
    wire(coilX - G.JX, line, coilX, line, root.id);
    wire(coilX + G.COIL_W, line, railRight, line, root.id);
    cells.push({
      kind: "coil",
      coil,
      index,
      box: { x: coilX, y, w: G.COIL_W, h: G.ROW },
    });
    const zx = coilX - G.JX - 10;
    const zw = railRight - zx;
    coilZones.push(
      {
        box: { x: zx, y, w: zw, h: G.ROW / 2 },
        target: { zone: "coil", rung: rung.id, index },
        area: "coil",
      },
      {
        box: { x: zx, y: y + G.ROW / 2, w: zw, h: G.ROW / 2 },
        target: { zone: "coil", rung: rung.id, index: index + 1 },
        area: "coil",
      },
    );
  });
  if (n > 1)
    wire(coilX - G.JX, line0, coilX - G.JX, (n - 1) * G.ROW + line0, root.id);
  coilZones.push({
    box: {
      x: coilX - G.JX - 10,
      y: n * G.ROW,
      w: railRight - coilX + G.JX + 10,
      h: n ? G.BOTTOM : G.ROW + G.BOTTOM,
    },
    target: { zone: "coil", rung: rung.id, index: n },
    area: "coil",
  });

  const height = Math.max(net.h, Math.max(1, n) * G.ROW) + G.BOTTOM;
  return {
    rung: rung.id,
    root: root.id,
    width: railRight + 6,
    height,
    netEnd,
    netHeight: net.h,
    coilX,
    railRight,
    cells,
    groups,
    wires,
    points,
    zones: [...leafZones, ...groupZones, ...rungZones, ...coilZones],
  };
}

const inside = (b: Box, x: number, y: number) =>
  x >= b.x && x < b.x + b.w && y >= b.y && y < b.y + b.h;

/**
 * The placement target under logical position (x, y) for something that belongs in
 * `area`: a target, `wrong` when the position is a zone of the other area (a coil over
 * the network, a contact over the coils), or null over empty space.
 */
export function hitTest(
  layout: RungLayout,
  x: number,
  y: number,
  area: Area,
): { target: Target } | { wrong: Area } | null {
  for (const z of layout.zones)
    if (inside(z.box, x, y))
      return z.area === area ? { target: z.target } : { wrong: z.area };
  return null;
}

/** The box a target refers to, for drawing the insertion preview. */
export function targetBox(layout: RungLayout, target: Target): Box | null {
  if (target.zone === "start")
    return (
      layout.cells.find((c) => c.kind === "empty")?.box ?? {
        x: G.RAIL,
        y: 0,
        w: G.EMPTY_W,
        h: G.ROW,
      }
    );
  if (target.zone === "coil") {
    const y = target.index * G.ROW;
    return { x: layout.coilX, y, w: G.COIL_W, h: 0 };
  }
  if (target.id === layout.root)
    return { x: G.RAIL, y: 0, w: layout.netEnd - G.RAIL, h: layout.netHeight };
  const cell = layout.cells.find(
    (c) => c.kind === "node" && c.node.id === target.id,
  );
  if (cell) return cell.box;
  return layout.groups.find((g) => g.id === target.id)?.box ?? null;
}

/** Where a field of an element is drawn, in layout coordinates. */
export function fieldBox(
  layout: RungLayout,
  element: number,
  field: Field,
): Box | null {
  for (const c of layout.cells) {
    const { x, y, w } = c.box;
    if (c.kind === "coil" && c.coil.id === element)
      return { x: x - 6, y: y + 2, w: w + 12, h: 22 };
    if (c.kind !== "node" || c.node.id !== element) continue;
    const n = c.node;
    if (n.type === "contact") return { x: x - 6, y: y + 2, w: w + 12, h: 22 };
    if (n.type === "compare")
      return field === "left"
        ? { x: x + 4, y: y + G.TOP + 2, w: 54, h: 20 }
        : { x: x + w - 58, y: y + G.TOP + 2, w: 54, h: 20 };
    if (n.type === "block") {
      if (field === "instance")
        return { x: x + 30, y: y + G.TOP + 2, w: w - 60, h: 20 };
      if (field === "preset") return { x: x + 24, y: y + 50, w: 62, h: 20 };
      if (field === "reset") return { x: x + 24, y: y + 76, w: 62, h: 20 };
      if (field === "output") return { x: x + w - 86, y: y + 50, w: 62, h: 20 };
    }
  }
  return null;
}

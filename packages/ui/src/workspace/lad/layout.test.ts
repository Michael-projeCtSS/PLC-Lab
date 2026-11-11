import { describe, expect, it } from "vitest";
import type { LadDocument, LadNode, Rung } from "../ladModel";
import { G, coilColumn, hitTest, layoutRung, targetBox } from "./layout";

const NO = (id: number, variable: string): LadNode => ({
  type: "contact",
  id,
  contact: "normallyOpen",
  variable,
});
const doc = (rungs: Rung[]): LadDocument => ({
  format: "plclab-lad",
  version: 2,
  name: "Main",
  variables: [],
  rungs,
});
// (A OR C) AND B -> Q
const nested: Rung = {
  id: 1,
  network: {
    type: "series",
    id: 2,
    items: [
      { type: "parallel", id: 5, branches: [NO(3, "A"), NO(6, "C")] },
      NO(4, "B"),
    ],
  },
  coils: [{ id: 7, variable: "Q" }],
};
const box = (l: ReturnType<typeof layoutRung>, id: number) => {
  const c = l.cells.find(
    (c) =>
      (c.kind === "node" && c.node.id === id) ||
      (c.kind === "coil" && c.coil.id === id),
  );
  if (!c) throw new Error(`no cell ${id}`);
  return c.box;
};

describe("rung layout derived from the ladder tree", () => {
  const l = layoutRung(nested, coilColumn(doc([nested])));

  it("places branches below each other and series to the right", () => {
    const a = box(l, 3);
    const c = box(l, 6);
    const b = box(l, 4);
    expect(c.x).toBe(a.x);
    expect(c.y).toBe(a.y + G.ROW);
    expect(b.y).toBe(a.y);
    expect(b.x).toBeGreaterThan(a.x + a.w);
    // The coil sits in the coil column, right of the network.
    expect(box(l, 7).x).toBe(l.coilX);
    expect(l.coilX).toBeGreaterThanOrEqual(l.netEnd + G.COIL_GAP);
  });

  it("draws the junctions of the parallel block and powers each wire from its source", () => {
    const vertical = l.wires.filter((w) => w.x1 === w.x2);
    const line = G.LINE;
    // Left junction fed by the rail, right junction by the block's own power.
    expect(vertical).toContainEqual(
      expect.objectContaining({ y1: line, y2: line + G.ROW, power: "rail" }),
    );
    expect(vertical).toContainEqual(
      expect.objectContaining({ y1: line, y2: line + G.ROW, power: 5 }),
    );
    // The wire into B carries the parallel block's power; the wire to the coil the rung's.
    const intoB = l.wires.find(
      (w) => w.y1 === line && w.y2 === line && w.x2 === box(l, 4).x,
    );
    expect(intoB?.power).toBe(5);
    expect(l.wires.some((w) => w.power === 2 && w.x2 === l.coilX)).toBe(true);
  });

  it("hit-tests positions into semantic targets", () => {
    const a = box(l, 3);
    const mid = a.y + G.LINE;
    expect(hitTest(l, a.x + 10, mid, "network")).toEqual({
      target: { zone: "before", id: 3 },
    });
    expect(hitTest(l, a.x + a.w - 10, mid, "network")).toEqual({
      target: { zone: "after", id: 3 },
    });
    // Just under A's body, between A and C: a branch beside A only.
    expect(
      hitTest(l, a.x + a.w / 2, a.y + G.TOP + G.BODY + 4, "network"),
    ).toEqual({
      target: { zone: "below", id: 3 },
    });
    // Below the last branch, left of it on the junction: a new branch of the block.
    const group = l.groups[0]!.box;
    expect(hitTest(l, group.x + 4, group.y + group.h - 6, "network")).toEqual({
      target: { zone: "branch", id: 5 },
    });
    // Under the whole network: in parallel with all of it.
    expect(hitTest(l, G.RAIL + 4, l.netHeight + 10, "network")).toEqual({
      target: { zone: "below", id: 2 },
    });
    // Coil positions accept coils; contacts there are told it is the wrong area.
    const q = box(l, 7);
    expect(hitTest(l, q.x + 10, q.y + 10, "coil")).toEqual({
      target: { zone: "coil", rung: 1, index: 0 },
    });
    expect(hitTest(l, q.x + 10, q.y + G.ROW - 10, "coil")).toEqual({
      target: { zone: "coil", rung: 1, index: 1 },
    });
    expect(hitTest(l, q.x + 10, q.y + 10, "network")).toEqual({
      wrong: "coil",
    });
    expect(hitTest(l, a.x + 10, mid, "coil")).toEqual({ wrong: "network" });
    // Far right of everything: nothing.
    expect(hitTest(l, l.width + 50, mid, "network")).toBeNull();
  });

  it("offers an insertion point on every series wire", () => {
    const afters = l.points.map((p) => p.after);
    // Before the block, between the block and B, after B.
    expect(afters).toEqual([null, 5, 4]);
    expect(l.points.map((p) => p.target)).toEqual(
      expect.arrayContaining([
        { zone: "before", id: 5 },
        { zone: "after", id: 5 },
        { zone: "after", id: 4 },
      ]),
    );
  });

  it("is deterministic and aligns the coil column of all rungs", () => {
    const short: Rung = {
      id: 10,
      network: { type: "series", id: 11, items: [NO(12, "X")] },
      coils: [],
    };
    const d = doc([nested, short]);
    const x = coilColumn(d);
    expect(layoutRung(short, x).coilX).toBe(layoutRung(nested, x).coilX);
    expect(layoutRung(nested, x)).toEqual(layoutRung(nested, x));
  });

  it("gives an empty network one start zone and a preview box", () => {
    const empty: Rung = {
      id: 20,
      network: { type: "series", id: 21, items: [] },
      coils: [{ id: 22, variable: "" }],
    };
    const e = layoutRung(empty, coilColumn(doc([empty])));
    const cell = e.cells.find((c) => c.kind === "empty")!;
    expect(hitTest(e, cell.box.x + 20, cell.box.y + 20, "network")).toEqual({
      target: { zone: "start", rung: 20 },
    });
    expect(targetBox(e, { zone: "start", rung: 20 })).toEqual(cell.box);
  });
});

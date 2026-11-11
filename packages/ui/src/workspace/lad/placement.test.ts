import { describe, expect, it } from "vitest";
import {
  elementIds,
  findNode,
  serializeLad,
  type LadDocument,
  type LadNode,
  type Rung,
} from "../ladModel";
import {
  applyDrop,
  expression,
  type DropResult,
  type Source,
  type Target,
} from "./placement";

const NO = (id: number, variable: string): LadNode => ({
  type: "contact",
  id,
  contact: "normallyOpen",
  variable,
});
const series = (id: number, items: LadNode[]): LadNode => ({
  type: "series",
  id,
  items,
});
const rung = (id: number, network: LadNode, coil = id + 100): Rung => ({
  id,
  network,
  coils: [{ id: coil, variable: "Q" }],
});
const doc = (...rungs: Rung[]): LadDocument => ({
  format: "plclab-lad",
  version: 2,
  name: "Main",
  variables: ["A", "B", "C", "D", "Q", "Q2"].map((name) => ({
    name,
    type: "BOOL" as const,
    direction: name.startsWith("Q") ? ("output" as const) : ("input" as const),
  })),
  rungs,
});
const newNo: Source = {
  kind: "new",
  item: { type: "contact", contact: "normallyOpen" },
};
/** Drop, then bind the new contact to `variable`. */
function drop(d: LadDocument, target: Target, variable = "C", source = newNo) {
  const r = applyDrop(d, source, target);
  if ("error" in r) throw new Error(r.error);
  const n = findNode(r.document, r.selected);
  if (source.kind === "new" && n?.type === "contact") n.variable = variable;
  return r.document;
}
const logic = (d: LadDocument, i = 0) => expression(d.rungs[i]!.network);
const unique = (d: LadDocument) => {
  const ids = elementIds(d);
  expect(new Set(ids).size).toBe(ids.length);
};
const error = (r: DropResult) => ("error" in r ? r.error : null);

describe("placement: series and parallel by drop target", () => {
  const ab = doc(rung(1, series(2, [NO(3, "A"), NO(4, "B")])));

  it("A: a contact dropped to the right of A is in series: A AND B", () => {
    const d = drop(
      doc(rung(1, series(2, [NO(3, "A")]))),
      { zone: "after", id: 3 },
      "B",
    );
    expect(logic(d)).toBe("A AND B");
    unique(d);
  });

  it("B: a contact dropped below A is in parallel: A OR C", () => {
    const d = drop(doc(rung(1, series(2, [NO(3, "A")]))), {
      zone: "below",
      id: 3,
    });
    expect(logic(d)).toBe("(A OR C)");
    expect(d.rungs[0]!.network.type).toBe("series");
  });

  it("C: below A in A AND B branches only A: (A OR C) AND B, not A OR (C AND B)", () => {
    const d = drop(ab, { zone: "below", id: 3 });
    expect(logic(d)).toBe("(A OR C) AND B");
    expect(logic(d)).not.toBe("A OR (C AND B)");
    unique(d);
  });

  it("below B gives A AND (B OR C); before A gives C AND A AND B", () => {
    expect(logic(drop(ab, { zone: "below", id: 4 }))).toBe("A AND (B OR C)");
    expect(logic(drop(ab, { zone: "before", id: 3 }))).toBe("C AND A AND B");
    expect(logic(drop(ab, { zone: "after", id: 3 }))).toBe("A AND C AND B");
  });

  it("below the whole network wraps all of it: (A AND B) OR C", () => {
    const d = drop(ab, { zone: "below", id: 2 });
    expect(logic(d)).toBe("((A AND B) OR C)");
    expect(d.rungs[0]!.network.id).toBe(2);
  });

  it("a new branch of a parallel block, and below a branch, add to that block", () => {
    const c = drop(ab, { zone: "below", id: 3 });
    const block = (c.rungs[0]!.network as { items: LadNode[] }).items[0]!;
    expect(logic(drop(c, { zone: "branch", id: block.id }, "D"))).toBe(
      "(A OR C OR D) AND B",
    );
    // Below A, which is now a branch: a sibling branch right after A.
    expect(logic(drop(c, { zone: "below", id: 3 }, "D"))).toBe(
      "(A OR D OR C) AND B",
    );
    // In series inside a branch.
    expect(logic(drop(c, { zone: "after", id: 3 }, "D"))).toBe(
      "((A AND D) OR C) AND B",
    );
  });

  it("an empty rung takes the element at its start", () => {
    const d = drop(
      doc(rung(1, series(2, []))),
      { zone: "start", rung: 1 },
      "A",
    );
    expect(logic(d)).toBe("A");
  });

  it("boxes and comparators are placed like contacts", () => {
    const r = applyDrop(
      ab,
      { kind: "new", item: { type: "block", block: "TON" } },
      {
        zone: "after",
        id: 4,
      },
    );
    if ("error" in r) throw new Error(r.error);
    expect(logic(r.document)).toBe("A AND B AND TON(T1)");
    const c = applyDrop(
      ab,
      { kind: "new", item: { type: "compare", op: "gt" } },
      {
        zone: "below",
        id: 4,
      },
    );
    if ("error" in c) throw new Error(c.error);
    expect(logic(c.document)).toBe("A AND (B OR CMP(gt))");
  });
});

describe("placement: moving and copying existing elements", () => {
  const ab = doc(rung(1, series(2, [NO(3, "A"), NO(4, "B")])));
  const move = (id: number, copy = false): Source => ({
    kind: "move",
    id,
    copy,
  });

  it("moves B below A: A OR B, keeping B's id and binding", () => {
    const r = applyDrop(ab, move(4), { zone: "below", id: 3 });
    if ("error" in r) throw new Error(r.error);
    expect(logic(r.document)).toBe("(A OR B)");
    expect(r.selected).toBe(4);
    expect(findNode(r.document, 4)).toEqual(NO(4, "B"));
    unique(r.document);
  });

  it("moves an element out of a branch; the one-branch block collapses", () => {
    const c = drop(ab, { zone: "below", id: 3 });
    const C = elementIds(c).find(
      (id) =>
        findNode(c, id)?.type === "contact" &&
        (findNode(c, id) as { variable: string }).variable === "C",
    )!;
    const r = applyDrop(c, move(C), { zone: "after", id: 4 });
    if ("error" in r) throw new Error(r.error);
    expect(logic(r.document)).toBe("A AND B AND C");
    expect(JSON.stringify(r.document)).not.toContain('"parallel"');
    unique(r.document);
  });

  it("reorders a series and moves to another rung", () => {
    const r = applyDrop(ab, move(3), { zone: "after", id: 4 });
    if ("error" in r) throw new Error(r.error);
    expect(logic(r.document)).toBe("B AND A");
    const two = doc(
      rung(1, series(2, [NO(3, "A"), NO(4, "B")])),
      rung(5, series(6, [NO(7, "C")])),
    );
    const m = applyDrop(two, move(4), { zone: "after", id: 7 });
    if ("error" in m) throw new Error(m.error);
    expect([logic(m.document, 0), logic(m.document, 1)]).toEqual([
      "A",
      "C AND B",
    ]);
    // The last element can leave a rung: the rung keeps an empty network.
    const e = applyDrop(m.document, move(7), { zone: "before", id: 3 });
    if ("error" in e) throw new Error(e.error);
    expect([logic(e.document, 0), logic(e.document, 1)]).toEqual([
      "C AND A",
      "B",
    ]);
  });

  it("a copy gets new ids and a new instance; the original stays", () => {
    const t = drop(ab, { zone: "after", id: 4 }, "", {
      kind: "new",
      item: { type: "block", block: "TON" },
    });
    const ton = elementIds(t).find((id) => findNode(t, id)?.type === "block")!;
    const r = applyDrop(t, move(ton, true), { zone: "below", id: 3 });
    if ("error" in r) throw new Error(r.error);
    expect(logic(r.document)).toBe("(A OR TON(T2)) AND B AND TON(T1)");
    expect(r.selected).not.toBe(ton);
    unique(r.document);
  });

  it("a copy may go right beside or below its original", () => {
    const r = applyDrop(ab, move(3, true), { zone: "below", id: 3 });
    if ("error" in r) throw new Error(r.error);
    expect(logic(r.document)).toBe("(A OR A) AND B");
    const s = applyDrop(ab, move(3, true), { zone: "after", id: 3 });
    if ("error" in s) throw new Error(s.error);
    expect(logic(s.document)).toBe("A AND A AND B");
  });

  it("refuses to drop a block into itself and does nothing for a drop in place", () => {
    const c = drop(ab, { zone: "below", id: 3 });
    const block = (c.rungs[0]!.network as { items: LadNode[] }).items[0]!;
    expect(error(applyDrop(c, move(block.id), { zone: "after", id: 3 }))).toBe(
      "intoItself",
    );
    expect(error(applyDrop(ab, move(3), { zone: "after", id: 3 }))).toBe(
      "noChange",
    );
    expect(error(applyDrop(ab, move(4), { zone: "after", id: 3 }))).toBe(
      "noChange",
    );
    expect(error(applyDrop(ab, move(3), { zone: "before", id: 4 }))).toBe(
      "noChange",
    );
  });

  it("only the rungs a drop changes are rewritten", () => {
    // Rung 5 holds a redundant series in a series: it must stay exactly as it is.
    const odd = rung(5, series(6, [series(7, [NO(8, "C"), NO(9, "D")])]));
    const d = doc(rung(1, series(2, [NO(3, "A"), NO(4, "B")])), odd);
    const r = applyDrop(d, move(4), { zone: "below", id: 3 });
    if ("error" in r) throw new Error(r.error);
    expect(r.document.rungs[1]).toBe(odd);
  });
});

describe("placement: coils and invalid drops", () => {
  const d = doc(
    rung(1, series(2, [NO(3, "A")])),
    rung(5, series(6, [NO(7, "B")])),
  );
  it("coils go to coil positions only; network elements never do", () => {
    const coil: Source = { kind: "new", item: { type: "coil", kind: "set" } };
    expect(error(applyDrop(d, coil, { zone: "after", id: 3 }))).toBe(
      "coilInNetwork",
    );
    expect(
      error(applyDrop(d, newNo, { zone: "coil", rung: 1, index: 0 })),
    ).toBe("elementInCoils");
    const r = applyDrop(d, coil, { zone: "coil", rung: 1, index: 1 });
    if ("error" in r) throw new Error(r.error);
    expect(r.document.rungs[0]!.coils).toEqual([
      { id: 101, variable: "Q" },
      { id: r.selected, variable: "", kind: "set" },
    ]);
  });

  it("moves a coil within its rung and to another rung", () => {
    const two = applyDrop(
      d,
      { kind: "new", item: { type: "coil", kind: "output" } },
      { zone: "coil", rung: 1, index: 1 },
    );
    if ("error" in two) throw new Error(two.error);
    const added = two.selected;
    const up = applyDrop(
      two.document,
      { kind: "move", id: added, copy: false },
      { zone: "coil", rung: 1, index: 0 },
    );
    if ("error" in up) throw new Error(up.error);
    expect(up.document.rungs[0]!.coils.map((c) => c.id)).toEqual([added, 101]);
    // Dropping right below itself is no change.
    expect(
      error(
        applyDrop(
          two.document,
          { kind: "move", id: added, copy: false },
          { zone: "coil", rung: 1, index: 2 },
        ),
      ),
    ).toBe("noChange");
    const away = applyDrop(
      two.document,
      { kind: "move", id: 101, copy: false },
      { zone: "coil", rung: 5, index: 1 },
    );
    if ("error" in away) throw new Error(away.error);
    expect(away.document.rungs.map((r) => r.coils.map((c) => c.id))).toEqual([
      [added],
      [105, 101],
    ]);
  });

  it("an unknown target changes nothing", () => {
    const before = serializeLad(d);
    expect(error(applyDrop(d, newNo, { zone: "after", id: 999 }))).toBe(
      "unknown",
    );
    expect(error(applyDrop(d, newNo, { zone: "start", rung: 999 }))).toBe(
      "unknown",
    );
    expect(serializeLad(d)).toBe(before);
  });
});

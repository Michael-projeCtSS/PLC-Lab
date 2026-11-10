import { describe, expect, it } from "vitest";
import { anyKey, byKey, byPointer, HeldButtons } from "./heldButtons";

describe("held push buttons", () => {
  it("every press gets exactly one release", () => {
    const held = new HeldButtons();
    expect(held.press("start", { pointer: 1 })).toBe(true);
    expect(held.release("start", byPointer(1))).toBe(true);
    // A repeated pointerup and the capture loss after it end nothing.
    expect(held.release("start", byPointer(1))).toBe(false);
    expect(held.release("start", byPointer(1))).toBe(false);
  });

  it("a held button is not pressed a second time", () => {
    const held = new HeldButtons();
    expect(held.press("start", { pointer: 1 })).toBe(true);
    expect(held.press("start", { pointer: 2 })).toBe(false);
    expect(held.press("start", { key: " " })).toBe(false);
  });

  it("only the holder that pressed a button ends its hold", () => {
    const held = new HeldButtons();
    held.press("start", { pointer: 1 });
    expect(held.release("start", byPointer(2))).toBe(false);
    expect(held.release("start", byKey(" "))).toBe(false);
    expect(held.release("start", anyKey)).toBe(false);
    expect(held.release("start", byPointer(1))).toBe(true);

    held.press("hold", { key: "Enter" });
    expect(held.release("hold", byKey(" "))).toBe(false);
    expect(held.release("hold", byPointer(1))).toBe(false);
    expect(held.release("hold", anyKey)).toBe(true);
  });

  it("a new press after the release is not ended by an old holder", () => {
    const held = new HeldButtons();
    held.press("start", { pointer: 1 });
    held.release("start", byPointer(1));
    held.press("start", { key: " " });
    expect(held.release("start", byPointer(1))).toBe(false);
    expect(held.release("start", byKey(" "))).toBe(true);
  });

  it("releasing everything names each held button once, in press order", () => {
    const held = new HeldButtons();
    held.press("hold", { pointer: 1 });
    held.press("start", { key: " " });
    expect(held.releaseAll()).toEqual(["hold", "start"]);
    expect(held.releaseAll()).toEqual([]);
    expect(held.release("hold", byPointer(1))).toBe(false);
  });
});

/**
 * The push buttons the operator holds pressed in the process view, and what holds each:
 * a pointer (by its pointer id) or a key (ADR-0048). A press starts a hold only when the
 * button is not held yet, and only a matching holder ends it. A second pointer, a
 * repeated pointerup or a capture loss after the release therefore never sends a second
 * press or release: every press gets exactly one release.
 */
export type Holder = { pointer: number } | { key: string };
export type HolderMatch = (holder: Holder) => boolean;

export const byPointer =
  (id: number): HolderMatch =>
  (holder) =>
    "pointer" in holder && holder.pointer === id;
export const byKey =
  (key: string): HolderMatch =>
  (holder) =>
    "key" in holder && holder.key === key;
export const anyKey: HolderMatch = (holder) => "key" in holder;

export class HeldButtons {
  private readonly held = new Map<string, Holder>();

  /** Start a hold; `false` when the button is already held. */
  press(component: string, holder: Holder): boolean {
    if (this.held.has(component)) return false;
    this.held.set(component, holder);
    return true;
  }

  /** End the hold when `match` accepts its holder; `false` when nothing ended. */
  release(component: string, match: HolderMatch): boolean {
    const holder = this.held.get(component);
    if (!holder || !match(holder)) return false;
    this.held.delete(component);
    return true;
  }

  /** End every hold: the buttons that must be released, in the order they were pressed. */
  releaseAll(): string[] {
    const components = [...this.held.keys()];
    this.held.clear();
    return components;
  }
}

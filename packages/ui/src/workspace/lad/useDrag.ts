/**
 * The drag session of the ladder editor (ADR-0052): pointer-driven, so it behaves the
 * same with a mouse, a pen and automated pointer input. A press becomes a drag after a
 * few pixels; the pointer is hit-tested against the rung layouts (logical coordinates,
 * any zoom); the target is validated with the placement engine on every move; release
 * applies it once; Esc or a release over nothing changes nothing. Ctrl copies an existing
 * element instead of moving it. Moving near the edges of the scroll area scrolls it.
 */
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { LadDocument } from "../ladModel";
import { hitTest, type Area, type RungLayout } from "./layout";
import {
  applyDrop,
  placesCoil,
  type DropError,
  type Source,
  type Target,
} from "./placement";

export type Over =
  | {
      rung: number;
      target: Target;
      error: DropError | null;
      x: number;
      y: number;
    }
  | { rung: number; wrong: Area; x: number; y: number }
  | { rung: number; nothing: true; x: number; y: number };

export type DragState = {
  source: Source;
  label: string;
  x: number;
  y: number;
  over: Over | null;
};

const THRESHOLD = 5;
const EDGE = 32;

export function useDrag({
  document,
  zoom,
  canvases,
  layouts,
  scroller,
  drop,
}: {
  document: LadDocument | null;
  zoom: number;
  canvases: React.RefObject<Map<number, HTMLDivElement>>;
  layouts: Map<number, RungLayout>;
  scroller: React.RefObject<HTMLElement | null>;
  drop: (source: Source, target: Target) => void;
}) {
  const [drag, setDrag] = useState<DragState | null>(null);
  const pending = useRef<{
    source: Source;
    label: string;
    x0: number;
    y0: number;
  } | null>(null);
  const live = useRef<DragState | null>(null);
  const justDropped = useRef(false);
  const latest = useRef({ document, zoom, layouts, drop });
  useLayoutEffect(() => {
    latest.current = { document, zoom, layouts, drop };
  });

  /**
   * Where rungs can be seen: the scroll area below its sticky toolbar. A rung scrolled
   * out of it (behind the toolbar or another panel) is no drop target. Null when the
   * area is not laid out.
   */
  const view = useCallback(() => {
    const el = scroller.current;
    const r = el?.getBoundingClientRect();
    if (!el || !r || r.width <= 3 * EDGE || r.height <= 3 * EDGE) return null;
    const bar = el.querySelector(".lad-toolbar")?.getBoundingClientRect();
    return {
      el,
      left: r.left,
      right: r.right,
      top: Math.max(r.top, bar?.bottom ?? r.top),
      bottom: r.bottom,
    };
  }, [scroller]);

  const locate = useCallback(
    (x: number, y: number, source: Source): Over | null => {
      const { document: d, zoom: z, layouts: ls } = latest.current;
      if (!d) return null;
      const v = view();
      if (v && (x < v.left || x >= v.right || y < v.top || y >= v.bottom))
        return null;
      for (const [rung, el] of canvases.current ?? []) {
        const r = el.getBoundingClientRect();
        if (x < r.left || x >= r.right || y < r.top || y >= r.bottom) continue;
        const layout = ls.get(rung);
        if (!layout) return null;
        const lx = (x - r.left) / z;
        const ly = (y - r.top) / z;
        const hit = hitTest(
          layout,
          lx,
          ly,
          placesCoil(d, source) ? "coil" : "network",
        );
        if (!hit) return { rung, nothing: true, x: lx, y: ly };
        if ("wrong" in hit) return { rung, wrong: hit.wrong, x: lx, y: ly };
        const result = applyDrop(d, source, hit.target);
        return {
          rung,
          target: hit.target,
          error: "error" in result ? result.error : null,
          x: lx,
          y: ly,
        };
      }
      return null;
    },
    [canvases, view],
  );

  const update = useCallback(
    (x: number, y: number, copy: boolean) => {
      const p = pending.current;
      if (!p) return;
      const source: Source =
        p.source.kind === "move" ? { ...p.source, copy } : p.source;
      const state: DragState = {
        source,
        label: p.label,
        x,
        y,
        over: locate(x, y, source),
      };
      live.current = state;
      setDrag(state);
    },
    [locate],
  );

  const cancel = useCallback(() => {
    pending.current = null;
    live.current = null;
    setDrag(null);
  }, []);

  useEffect(() => {
    const move = (e: PointerEvent) => {
      const p = pending.current;
      if (!p) return;
      if (
        !live.current &&
        Math.hypot(e.clientX - p.x0, e.clientY - p.y0) < THRESHOLD
      )
        return;
      e.preventDefault();
      // Near an edge of the visible rung area the view follows the pointer (driven by
      // pointer movement only: the interface runs no timers). Over the toolbar, where a
      // drag from the palette starts, nothing scrolls.
      const v = view();
      if (v) {
        const { clientX: x, clientY: y } = e;
        const inside = y >= v.top && y < v.bottom && x >= v.left && x < v.right;
        const dy = !inside
          ? 0
          : y < v.top + EDGE
            ? -24
            : y > v.bottom - EDGE
              ? 24
              : 0;
        const dx = !inside
          ? 0
          : x < v.left + EDGE
            ? -24
            : x > v.right - EDGE
              ? 24
              : 0;
        if (dx || dy) v.el.scrollBy(dx, dy);
      }
      update(e.clientX, e.clientY, e.ctrlKey);
    };
    const up = () => {
      const state = live.current;
      pending.current = null;
      live.current = null;
      if (!state) return;
      setDrag(null);
      justDropped.current = true;
      setTimeout(() => (justDropped.current = false), 0);
      const over = state.over;
      if (over && "target" in over && !over.error)
        latest.current.drop(state.source, over.target);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || !pending.current) return;
      const dragging = !!live.current;
      cancel();
      if (dragging) {
        e.preventDefault();
        e.stopPropagation();
      }
    };
    const ctrl = (e: KeyboardEvent) => {
      const state = live.current;
      if (state && e.key === "Control")
        update(state.x, state.y, e.type === "keydown");
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("blur", cancel);
    window.addEventListener("keydown", key, true);
    window.addEventListener("keydown", ctrl);
    window.addEventListener("keyup", ctrl);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("keydown", ctrl);
      window.removeEventListener("keyup", ctrl);
    };
  }, [update, cancel, view]);

  return {
    drag,
    /** Start a possible drag on pointer down; it becomes one after a few pixels. */
    grab: (e: React.PointerEvent, source: Source, label: string) => {
      pending.current = { source, label, x0: e.clientX, y0: e.clientY };
    },
    cancel,
    /** True right after a drag ended: the click that follows is not a command. */
    justDropped: () => justDropped.current,
  };
}

/**
 * The context menu of a ladder element (ADR-0052): opened by right-click, the context-menu
 * key or Shift+F10; every action is an ordinary editor command on the selected element.
 */
import { useEffect, useRef, useState } from "react";
import type { T } from "./common";

export type MenuItem = { label: string; run: () => void; disabled?: boolean };

export function ContextMenu({
  t,
  x,
  y,
  items,
  close,
}: {
  t: T;
  x: number;
  y: number;
  items: MenuItem[];
  close: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(0);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    const away = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) close();
    };
    window.addEventListener("pointerdown", away, true);
    return () => window.removeEventListener("pointerdown", away, true);
  }, [close]);
  const enabled = items
    .map((i, n) => (i.disabled ? -1 : n))
    .filter((n) => n >= 0);
  const focus = (n: number) => {
    setActive(n);
    ref.current?.querySelectorAll<HTMLElement>('[role="menuitem"]')[n]?.focus();
  };
  return (
    <div
      ref={ref}
      className="lad-menu"
      role="menu"
      aria-label={t("lad.menu")}
      style={{
        left: Math.min(x, window.innerWidth - 240),
        top: Math.min(y, window.innerHeight - 30 * items.length - 12),
      }}
      onKeyDown={(e) => {
        e.stopPropagation();
        const at = enabled.indexOf(active);
        if (e.key === "ArrowDown")
          focus(enabled[(at + 1) % enabled.length] ?? 0);
        else if (e.key === "ArrowUp")
          focus(enabled[(at - 1 + enabled.length) % enabled.length] ?? 0);
        else if (e.key === "Escape" || e.key === "Tab") close();
        else return;
        e.preventDefault();
      }}
    >
      {items.map((item, n) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          className="lad-menu-item"
          disabled={item.disabled}
          tabIndex={n === active ? 0 : -1}
          onClick={() => {
            close();
            item.run();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}

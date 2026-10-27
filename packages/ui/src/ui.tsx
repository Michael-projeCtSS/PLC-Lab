/** Design-system primitives shared by pages and the workspace. */
import { useEffect, useRef, useState, type ReactNode } from "react";

export function Badge({
  kind = "normal",
  children,
  title,
}: {
  kind?: string;
  children: ReactNode;
  title?: string;
}) {
  return (
    <span className={`badge ${kind}`} title={title}>
      {children}
    </span>
  );
}

export function PlcModeBadge({ mode }: { mode: "RUN" | "STOP" | "FAULT" }) {
  const kind =
    mode === "RUN" ? "running" : mode === "FAULT" ? "fault" : "stopped";
  return <Badge kind={kind}>PLC {mode}</Badge>;
}

export function SimulationBadge({
  state,
}: {
  state: "RUNNING" | "PAUSED" | "STOPPED";
}) {
  const kind =
    state === "RUNNING" ? "active" : state === "PAUSED" ? "warning" : "normal";
  return <Badge kind={kind}>Sim {state.toLowerCase()}</Badge>;
}

export function Tabs<T extends string>({
  tabs,
  active,
  onSelect,
  label,
  extra,
}: {
  tabs: readonly { id: T; label: string; badge?: ReactNode }[];
  active: T;
  onSelect: (id: T) => void;
  label: string;
  extra?: ReactNode;
}) {
  return (
    <div className="tabs" role="tablist" aria-label={label}>
      {tabs.map((t) => (
        <button
          key={t.id}
          role="tab"
          aria-selected={t.id === active}
          onClick={() => onSelect(t.id)}
        >
          {t.label}
          {t.badge}
        </button>
      ))}
      {extra && (
        <div
          className="row spacer"
          style={{ justifyContent: "flex-end", paddingRight: 6 }}
        >
          {extra}
        </div>
      )}
    </div>
  );
}

export function ErrorNotice({
  error,
  onDismiss,
}: {
  error: unknown;
  onDismiss?: () => void;
}) {
  if (!error) return null;
  const message = error instanceof Error ? error.message : String(error);
  return (
    <div className="notice error row" role="alert">
      <span className="grow">{message}</span>
      {onDismiss && (
        <button className="btn btn-ghost btn-sm" onClick={onDismiss}>
          Dismiss
        </button>
      )}
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return <div className="empty">{children}</div>;
}

/** Drag handle that resizes an adjacent pane; size is clamped and reported in pixels. */
export function Splitter({
  direction,
  size,
  onResize,
  min,
  max,
  invert = false,
}: {
  direction: "vertical" | "horizontal";
  size: number;
  onResize: (size: number) => void;
  min: number;
  max: number;
  invert?: boolean;
}) {
  const start = useRef<{ pos: number; size: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const onPointerDown = (e: React.PointerEvent) => {
    start.current = {
      pos: direction === "vertical" ? e.clientX : e.clientY,
      size,
    };
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    setDragging(true);
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!start.current) return;
    const delta =
      (direction === "vertical" ? e.clientX : e.clientY) - start.current.pos;
    onResize(
      Math.min(
        max,
        Math.max(min, start.current.size + (invert ? -delta : delta)),
      ),
    );
  };
  const end = () => {
    start.current = null;
    setDragging(false);
  };
  return (
    <div
      role="separator"
      aria-orientation={direction}
      className={`${direction === "vertical" ? "splitter-v" : "splitter-h"}${dragging ? " dragging" : ""}`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={end}
      onPointerCancel={end}
    />
  );
}

export function Modal({
  title,
  children,
  onClose,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
}) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="modal" role="dialog" aria-modal="true" aria-label={title}>
        <h2>{title}</h2>
        {children}
      </div>
    </div>
  );
}

/** Fixed-row-height virtualized list for large logs. */
export function VirtualList<T>({
  items,
  rowHeight,
  render,
  stickToEnd = false,
  label,
}: {
  items: readonly T[];
  rowHeight: number;
  render: (item: T, index: number) => ReactNode;
  stickToEnd?: boolean;
  label: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ top: 0, height: 300 });
  const follow = useRef(true);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () =>
      setView({ top: el.scrollTop, height: el.clientHeight || 300 });
    measure();
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(measure);
    observer?.observe(el);
    return () => observer?.disconnect();
  }, []);
  useEffect(() => {
    const el = ref.current;
    if (el && stickToEnd && follow.current) el.scrollTop = el.scrollHeight;
  }, [items.length, stickToEnd]);
  const first = Math.max(0, Math.floor(view.top / rowHeight) - 5);
  const last = Math.min(
    items.length,
    Math.ceil((view.top + view.height) / rowHeight) + 5,
  );
  return (
    <div
      ref={ref}
      className="vlist"
      role="log"
      aria-label={label}
      onScroll={(e) => {
        const el = e.currentTarget;
        follow.current =
          el.scrollTop + el.clientHeight >= el.scrollHeight - rowHeight;
        setView({ top: el.scrollTop, height: el.clientHeight });
      }}
    >
      <div style={{ height: items.length * rowHeight, position: "relative" }}>
        {items.slice(first, last).map((item, i) => (
          <div
            key={first + i}
            style={{
              position: "absolute",
              top: (first + i) * rowHeight,
              left: 0,
              right: 0,
              height: rowHeight,
            }}
          >
            {render(item, first + i)}
          </div>
        ))}
      </div>
    </div>
  );
}

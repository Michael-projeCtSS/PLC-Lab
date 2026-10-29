/** Design-system primitives shared by pages and the workspace. */
import { useInterface } from "./locale";
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
  const { t } = useInterface();
  const kind =
    state === "RUNNING" ? "active" : state === "PAUSED" ? "warning" : "normal";
  return (
    <Badge kind={kind}>
      {t(
        state === "RUNNING"
          ? "status.running"
          : state === "PAUSED"
            ? "status.paused"
            : "status.stopped",
      )}
    </Badge>
  );
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
  const { t } = useInterface();
  if (!error) return null;
  const message = error instanceof Error ? error.message : String(error);
  return (
    <div className="notice error row" role="alert">
      <div className="grow">
        <strong>{t("error.heading")}</strong>
        <details>
          <summary>{t("error.details")}</summary>
          <div style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>
            {message}
          </div>
        </details>
      </div>
      {onDismiss && (
        <button className="btn btn-ghost btn-sm" onClick={onDismiss}>
          {t("common.dismiss")}
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
  const dialog = useRef<HTMLDivElement>(null);
  const previousFocus = useRef(
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  );
  const close = useRef(onClose);
  useEffect(() => {
    close.current = onClose;
  }, [onClose]);
  useEffect(() => {
    const previous = previousFocus.current;
    const focusable = () =>
      Array.from(
        dialog.current?.querySelectorAll<HTMLElement>(
          'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), summary, [tabindex="0"]',
        ) ?? [],
      );
    focusable()[0]?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close.current();
      }
      if (e.key === "Tab") {
        const items = focusable();
        const first = items[0],
          last = items[items.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    };
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("keydown", key);
      previous?.focus();
    };
  }, []);
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        ref={dialog}
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
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

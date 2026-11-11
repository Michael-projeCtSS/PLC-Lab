/**
 * Editing a ladder operand in place (ADR-0052): a combobox on the diagram with completion
 * from the program's declarations. Enter accepts the highlighted suggestion, Tab accepts
 * and moves to the next field, Esc cancels, arrows move through the list, Ctrl+Space
 * opens it. An undeclared name is never bound silently: it is offered for creation with
 * its type and kind shown for confirmation. Keys typed here never reach the editor's
 * shortcuts.
 */
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  LAD_TYPES,
  type Direction,
  type LadDocument,
  type LadType,
} from "../ladModel";
import type { T } from "./common";
import {
  expectationFor,
  literalType,
  suggest,
  takenNames,
  type Field,
  type Suggestion,
} from "./complete";

/** What the student chose for a field. */
export type Accepted =
  | { kind: "variable"; name: string }
  | { kind: "literal"; text: string }
  | { kind: "instance"; name: string }
  | { kind: "create"; name: string; type: LadType; direction: Direction }
  | { kind: "clear" };

const DIRECTIONS: Direction[] = ["input", "output", "local"];

export function InlineEditor({
  t,
  document,
  element,
  field,
  label,
  initial,
  typed,
  channels,
  accept,
  cancel,
}: {
  t: T;
  document: LadDocument;
  element: number;
  field: Field;
  /** Accessible name: "Preset time PT of TON RunDelay". */
  label: string;
  initial: string;
  /** Opened by typing a character (keep the caret after it) or on the current value. */
  typed: boolean;
  channels: Map<string, string>;
  /** Apply the choice; `advance` is 1 or -1 after Tab / Shift+Tab, 0 after Enter. */
  accept: (choice: Accepted, advance: -1 | 0 | 1) => void;
  cancel: () => void;
}) {
  const [text, setText] = useState(initial);
  // On the current value the list starts complete; typing filters it.
  const [query, setQuery] = useState(typed ? initial : "");
  const [active, setActive] = useState(0);
  const [open, setOpen] = useState(true);
  const [creating, setCreating] = useState<{
    name: string;
    type: LadType;
    direction: Direction;
  } | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const list = useId();
  const expect = expectationFor(document, element, field);
  const suggestions = open
    ? suggest(document, element, field, query, channels)
    : [];
  const current = suggestions[Math.min(active, suggestions.length - 1)];

  useEffect(() => {
    const el = input.current;
    if (!el) return;
    el.focus();
    // A typed first character continues; the current value is selected for replacing.
    if (typed) el.setSelectionRange(el.value.length, el.value.length);
    else el.select();
  }, [typed]);

  const choose = (s: Suggestion | undefined, advance: -1 | 0 | 1) => {
    const typed = text.trim();
    if (!s) {
      if (!typed) return accept({ kind: "clear" }, advance);
      // A literal field keeps the student's draft; Rust reports what is wrong with it.
      if (expect.literals.length)
        return accept({ kind: "literal", text: typed }, advance);
      if (expect.instance) {
        setMessage(t("lad.suggest.instanceTaken", { name: typed }));
        return;
      }
      setMessage(t("lad.suggest.unknown", { name: typed }));
      return;
    }
    if (s.kind === "variable")
      return accept({ kind: "variable", name: s.name }, advance);
    if (s.kind === "literal")
      return accept({ kind: "literal", text: s.text }, advance);
    if (s.kind === "instance")
      return accept({ kind: "instance", name: s.name }, advance);
    setCreating({ name: s.name, type: s.type, direction: s.direction });
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    e.stopPropagation();
    const n = suggestions.length;
    if (e.key === "ArrowDown") {
      setOpen(true);
      setActive((a) => (n ? (a + 1) % n : 0));
    } else if (e.key === "ArrowUp") {
      setOpen(true);
      setActive((a) => (n ? (a - 1 + n) % n : 0));
    } else if (e.key === " " && e.ctrlKey) {
      setOpen(true);
    } else if (e.key === "Enter") {
      // Enter with the exact text of a suggestion takes that one, else the highlighted one.
      const exact = suggestions.find(
        (s) =>
          s.kind !== "create" &&
          ("name" in s ? s.name : s.text).toLowerCase() ===
            text.trim().toLowerCase(),
      );
      choose(exact ?? current, 0);
    } else if (e.key === "Tab") {
      const exact = suggestions.find(
        (s) =>
          s.kind !== "create" &&
          ("name" in s ? s.name : s.text).toLowerCase() ===
            text.trim().toLowerCase(),
      );
      if (text.trim() === initial.trim() && !exact) cancel();
      else choose(exact ?? current, e.shiftKey ? -1 : 1);
    } else if (e.key === "Escape") {
      cancel();
    } else return;
    e.preventDefault();
  };

  const confirmCreate = () => {
    if (!creating) return;
    if (takenNames(document, 0).has(creating.name.toLowerCase())) {
      setMessage(t("lad.suggest.instanceTaken", { name: creating.name }));
      return;
    }
    accept({ kind: "create", ...creating }, 0);
  };

  const describeAccess = expect.instance
    ? t("lad.suggest.instanceHint")
    : `${expect.types.join(" | ")} · ${t(expect.writable ? "lad.suggest.write" : "lad.suggest.read")}${expect.literals.length ? ` · ${t("lad.suggest.literals")}` : ""}`;

  return (
    <div className="lad-inline">
      <input
        ref={input}
        className="input mono lad-inline-input"
        role="combobox"
        aria-label={label}
        aria-expanded={open && suggestions.length > 0}
        aria-controls={list}
        aria-autocomplete="list"
        aria-activedescendant={
          current
            ? `${list}-${Math.min(active, suggestions.length - 1)}`
            : undefined
        }
        aria-invalid={message ? true : undefined}
        value={text}
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => {
          setText(e.target.value);
          setQuery(e.target.value);
          setActive(0);
          setOpen(true);
          setMessage(null);
          setCreating(null);
        }}
        onKeyDown={onKeyDown}
        onBlur={(e) => {
          // Leaving for the suggestion list keeps editing; leaving elsewhere commits only
          // what is unambiguous (an existing name or a literal) and otherwise cancels.
          const to = e.relatedTarget as Node | null;
          if (to && popup.current?.contains(to)) return;
          const typed = text.trim();
          if (typed === initial.trim()) return cancel();
          const exact = suggest(document, element, field, typed, channels).find(
            (s) =>
              (s.kind === "variable" || s.kind === "instance") &&
              s.name.toLowerCase() === typed.toLowerCase(),
          );
          if (exact && exact.kind === "variable")
            accept({ kind: "variable", name: exact.name }, 0);
          else if (exact && exact.kind === "instance")
            accept({ kind: "instance", name: exact.name }, 0);
          else if (expect.literals.length && literalType(typed))
            accept({ kind: "literal", text: typed }, 0);
          else cancel();
        }}
      />
      <Popup anchor={input}>
        <div
          ref={popup}
          className="lad-suggest"
          onMouseDown={(e) => e.preventDefault()}
        >
          <p className="lad-suggest-head">{describeAccess}</p>
          {creating ? (
            <form
              className="lad-create"
              aria-label={t("lad.suggest.createTitle", { name: creating.name })}
              onSubmit={(e) => {
                e.preventDefault();
                confirmCreate();
              }}
            >
              <strong className="mono">
                {t("lad.suggest.createTitle", { name: creating.name })}
              </strong>
              <label className="field">
                {t("lad.variableType")}
                <select
                  className="input"
                  value={creating.type}
                  onChange={(e) =>
                    setCreating({
                      ...creating,
                      type: e.target.value as LadType,
                    })
                  }
                  onKeyDown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Escape") setCreating(null);
                  }}
                >
                  {(expect.types.length ? expect.types : LAD_TYPES).map((x) => (
                    <option key={x} value={x}>
                      {x}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                {t("lad.direction")}
                <select
                  className="input"
                  value={creating.direction}
                  onChange={(e) =>
                    setCreating({
                      ...creating,
                      direction: e.target.value as Direction,
                    })
                  }
                  onKeyDown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Escape") setCreating(null);
                  }}
                >
                  {DIRECTIONS.filter(
                    (d) => !expect.writable || d !== "input",
                  ).map((d) => (
                    <option key={d} value={d}>
                      {t(`lad.dir.${d}`)}
                    </option>
                  ))}
                </select>
              </label>
              <div className="row" style={{ gap: 6 }}>
                <button
                  className="btn btn-sm btn-primary"
                  type="submit"
                  autoFocus
                >
                  {t("lad.suggest.createConfirm")}
                </button>
                <button
                  className="btn btn-sm"
                  type="button"
                  onClick={() => setCreating(null)}
                >
                  {t("lad.suggest.cancel")}
                </button>
              </div>
            </form>
          ) : (
            <ul id={list} role="listbox" aria-label={t("lad.suggest.list")}>
              {suggestions.map((s, i) => (
                <li
                  key={`${s.kind}:${"name" in s ? s.name : s.text}`}
                  id={`${list}-${i}`}
                  role="option"
                  aria-selected={i === Math.min(active, suggestions.length - 1)}
                  className="lad-option"
                  onClick={() => choose(s, 0)}
                  onMouseEnter={() => setActive(i)}
                >
                  <Option t={t} s={s} />
                </li>
              ))}
              {!suggestions.length && (
                <li className="lad-option empty" role="presentation">
                  {text.trim()
                    ? expect.literals.length
                      ? t("lad.suggest.keepLiteral", { text: text.trim() })
                      : t("lad.suggest.none", {
                          types: expect.types.join(" | "),
                        })
                    : t("lad.suggest.typeToSearch")}
                </li>
              )}
            </ul>
          )}
          {message && (
            <p className="lad-suggest-message" role="alert">
              {message}
            </p>
          )}
        </div>
      </Popup>
    </div>
  );
}

function Option({ t, s }: { t: T; s: Suggestion }) {
  if (s.kind === "variable")
    return (
      <>
        <span className="mono lad-option-name">{s.name}</span>
        <span className="badge">{s.type}</span>
        <span className="muted">{t(`lad.dir.${s.direction}`)}</span>
        {s.channel && (
          <span className="muted mono">
            {t("lad.suggest.channel", { channel: s.channel })}
          </span>
        )}
        {s.comment && (
          <span className="muted lad-option-comment">{s.comment}</span>
        )}
      </>
    );
  if (s.kind === "literal")
    return (
      <>
        <span className="mono lad-option-name">{s.text}</span>
        <span className="badge">{s.type}</span>
        <span className="muted">{t("lad.suggest.literal")}</span>
      </>
    );
  if (s.kind === "instance")
    return (
      <>
        <span className="mono lad-option-name">{s.name}</span>
        <span className="muted">{t("lad.suggest.instanceNew")}</span>
      </>
    );
  return (
    <span className="lad-option-create">
      {t("lad.suggest.create", { name: s.name })}{" "}
      <span className="muted">
        {s.type} · {t(`lad.dir.${s.direction}`)}
      </span>
    </span>
  );
}

/** The suggestion list floats above everything, under its input, outside scroll clipping. */
function Popup({
  anchor,
  children,
}: {
  anchor: React.RefObject<HTMLElement | null>;
  children: React.ReactNode;
}) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  useLayoutEffect(() => {
    const update = () =>
      setRect(anchor.current?.getBoundingClientRect() ?? null);
    update();
    window.addEventListener("scroll", update, true);
    window.addEventListener("resize", update);
    return () => {
      window.removeEventListener("scroll", update, true);
      window.removeEventListener("resize", update);
    };
  }, [anchor]);
  if (!rect) return null;
  const below = window.innerHeight - rect.bottom > 220 || rect.top < 240;
  return createPortal(
    <div
      className="lad-popup"
      style={{
        left: Math.max(4, Math.min(rect.left, window.innerWidth - 344)),
        ...(below
          ? { top: rect.bottom + 2 }
          : { bottom: window.innerHeight - rect.top + 2 }),
      }}
    >
      {children}
    </div>,
    globalThis.document.body,
  );
}

/**
 * Instruction completion (ADR-0052): typed at an insertion point or opened with F8, it
 * lists the instructions LAD supports with a short description and their pins — box pins
 * as the runtime declares them — and inserts the chosen one as a real element.
 */
import { useEffect, useId, useRef, useState } from "react";
import { describeInstruction, type T } from "./common";
import { matchInstructions, type Instruction } from "./complete";

export function InstructionPicker({
  t,
  list,
  initial,
  where,
  choose,
  cancel,
}: {
  t: T;
  list: readonly Instruction[];
  initial: string;
  /** "after StartPB in rung 2": where the instruction goes. */
  where: string;
  choose: (i: Instruction) => void;
  cancel: () => void;
}) {
  const [text, setText] = useState(initial);
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const id = useId();
  const matches = matchInstructions(list, text, (i) =>
    describeInstruction(t, i),
  );
  const current = matches[Math.min(active, matches.length - 1)];
  useEffect(() => input.current?.focus(), []);

  return (
    <div
      className="lad-picker"
      role="dialog"
      aria-label={t("lad.picker.title")}
    >
      <p className="lad-suggest-head">
        {t("lad.picker.title")} · {where}
      </p>
      <input
        ref={input}
        className="input mono"
        role="combobox"
        aria-label={t("lad.picker.input")}
        aria-expanded={matches.length > 0}
        aria-controls={id}
        aria-autocomplete="list"
        aria-activedescendant={
          current ? `${id}-${Math.min(active, matches.length - 1)}` : undefined
        }
        value={text}
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => {
          setText(e.target.value);
          setActive(0);
        }}
        onBlur={(e) => {
          if (
            !e.currentTarget.parentElement?.contains(
              e.relatedTarget as Node | null,
            )
          )
            cancel();
        }}
        onKeyDown={(e) => {
          e.stopPropagation();
          const n = matches.length;
          if (e.key === "ArrowDown") setActive((a) => (n ? (a + 1) % n : 0));
          else if (e.key === "ArrowUp")
            setActive((a) => (n ? (a - 1 + n) % n : 0));
          else if (e.key === "Enter" || e.key === "Tab") {
            if (current) choose(current);
          } else if (e.key === "Escape") cancel();
          else return;
          e.preventDefault();
        }}
      />
      <ul
        id={id}
        role="listbox"
        aria-label={t("lad.picker.list")}
        onMouseDown={(e) => e.preventDefault()}
      >
        {matches.map((i, n) => (
          <li
            key={i.name}
            id={`${id}-${n}`}
            role="option"
            aria-selected={n === Math.min(active, matches.length - 1)}
            className="lad-option instr"
            onClick={() => choose(i)}
            onMouseEnter={() => setActive(n)}
          >
            <span className="mono lad-option-name">{i.name}</span>
            <span className="lad-option-comment">
              {describeInstruction(t, i)}
            </span>
            {i.pins.length > 0 && (
              <span className="mono muted lad-pins">
                {i.pins
                  .filter((p) => p.direction === "in")
                  .map((p) => `${p.name}: ${p.type}`)
                  .join(", ")}
                {i.pins.some((p) => p.direction === "out") && " → "}
                {i.pins
                  .filter((p) => p.direction === "out")
                  .map((p) => `${p.name}: ${p.type}`)
                  .join(", ")}
              </span>
            )}
          </li>
        ))}
        {!matches.length && (
          <li className="lad-option empty" role="presentation">
            {t("lad.picker.none")}
          </li>
        )}
      </ul>
    </div>
  );
}

/**
 * The properties panel of the selected element (ADR-0051): every setting as an ordinary
 * form control. Since ADR-0052 the diagram is edited in place; this panel stays as the
 * fallback for keyboard and screen-reader users and for settings such as contact kinds.
 */
import { useState } from "react";
import {
  findCoil,
  findNode,
  isTimer,
  setBlockPin,
  setCoil,
  updateElement,
  type CoilKind,
  type CompareOp,
  type ContactKind,
  type LadDocument,
  type LadNode,
  type LadType,
  type Operand,
} from "../ladModel";
import { COILS, CONTACTS, OPS, type Selected, type T } from "./common";

/** The settings of the selected element: variables, kinds, operands, block pins. */
export function Properties({
  t,
  document,
  selected,
  commit,
}: {
  t: T;
  document: LadDocument;
  selected: Selected;
  commit: (d: LadDocument) => void;
}) {
  if (!selected) return null;
  const vars = (type: LadType, writable = false) =>
    document.variables
      .filter((v) => v.type === type && (!writable || v.direction !== "input"))
      .map((v) => v.name);
  if (selected.kind === "coil") {
    const coil = findCoil(document, selected.id);
    if (!coil) return null;
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <VariableSelect
          t={t}
          label={t("lad.coilVariable")}
          value={coil.variable}
          options={vars("BOOL", true)}
          onChange={(variable) =>
            commit(setCoil(document, coil.id, { variable }))
          }
        />
        <label className="field">
          {t("lad.coilKind")}
          <select
            className="input"
            value={coil.kind ?? "output"}
            onChange={(e) =>
              commit(
                setCoil(document, coil.id, {
                  kind: e.target.value as CoilKind,
                }),
              )
            }
          >
            {COILS.map((c) => (
              <option key={c.kind} value={c.kind}>
                {c.symbol} {t(`lad.coil.${c.kind}`)}
              </option>
            ))}
          </select>
        </label>
      </section>
    );
  }
  if (selected.kind === "rung" || selected.kind === "point") return null;
  const node = findNode(document, selected.id);
  if (!node) return null;
  const edit = (change: (n: LadNode) => LadNode) =>
    commit(updateElement(document, node.id, change));
  if (node.type === "contact")
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <VariableSelect
          t={t}
          label={t("lad.contactVariable")}
          value={node.variable}
          options={vars("BOOL")}
          onChange={(variable) =>
            edit((n) => (n.type === "contact" ? { ...n, variable } : n))
          }
        />
        <label className="field">
          {t("lad.contactKind")}
          <select
            className="input"
            value={node.contact}
            onChange={(e) =>
              edit((n) =>
                n.type === "contact"
                  ? { ...n, contact: e.target.value as ContactKind }
                  : n,
              )
            }
          >
            {CONTACTS.map((c) => (
              <option key={c.kind} value={c.kind}>
                {c.symbol} {t(c.label, { variable: "" }).trim()}
              </option>
            ))}
          </select>
        </label>
      </section>
    );
  if (node.type === "compare") {
    const all = document.variables.map((v) => v.name);
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <OperandEditor
          key={`l${node.id}${JSON.stringify(node.left)}`}
          t={t}
          label={t("lad.left")}
          value={node.left}
          options={all}
          onChange={(left) =>
            edit((n) => (n.type === "compare" ? { ...n, left } : n))
          }
        />
        <label className="field">
          {t("lad.comparison")}
          <select
            className="input"
            value={node.op}
            onChange={(e) =>
              edit((n) =>
                n.type === "compare"
                  ? { ...n, op: e.target.value as CompareOp }
                  : n,
              )
            }
          >
            {OPS.map((o) => (
              <option key={o.op} value={o.op}>
                {o.op.toUpperCase()} {o.symbol}
              </option>
            ))}
          </select>
        </label>
        <OperandEditor
          key={`r${node.id}${JSON.stringify(node.right)}`}
          t={t}
          label={t("lad.right")}
          value={node.right}
          options={all}
          onChange={(right) =>
            edit((n) => (n.type === "compare" ? { ...n, right } : n))
          }
        />
      </section>
    );
  }
  if (node.type === "block") {
    const timer = isTimer(node.block);
    return (
      <section
        className="lad-properties row wrap"
        aria-label={t("lad.properties")}
      >
        <InstanceName
          key={`i${node.id}${node.instance}`}
          t={t}
          value={node.instance}
          onChange={(instance) =>
            edit((n) => (n.type === "block" ? { ...n, instance } : n))
          }
        />
        <OperandEditor
          key={`p${node.id}${JSON.stringify(node.preset)}`}
          t={t}
          label={t(timer ? "lad.presetPT" : "lad.presetPV")}
          value={node.preset ?? { constant: "" }}
          options={vars(timer ? "TIME" : "INT")}
          onChange={(preset) =>
            edit((n) => (n.type === "block" ? { ...n, preset } : n))
          }
        />
        {!timer && (
          <OperandEditor
            key={`z${node.id}${JSON.stringify(node.reset)}`}
            t={t}
            label={t(node.block === "CTD" ? "lad.loadLD" : "lad.resetR")}
            value={node.reset ?? { constant: "FALSE" }}
            options={vars("BOOL")}
            onChange={(reset) =>
              commit(
                setBlockPin(
                  document,
                  node.id,
                  "reset",
                  "constant" in reset &&
                    reset.constant.toUpperCase() === "FALSE"
                    ? null
                    : reset,
                ),
              )
            }
          />
        )}
        <VariableSelect
          t={t}
          label={t(timer ? "lad.outputET" : "lad.outputCV")}
          value={node.output ?? ""}
          options={vars(timer ? "TIME" : "INT", true)}
          none={t("lad.noOutput")}
          onChange={(output) =>
            commit(setBlockPin(document, node.id, "output", output))
          }
        />
      </section>
    );
  }
  return null;
}

function VariableSelect({
  t,
  label,
  value,
  options,
  none,
  onChange,
}: {
  t: T;
  label: string;
  value: string;
  options: string[];
  none?: string;
  onChange: (v: string) => void;
}) {
  const list =
    value && !options.includes(value) ? [value, ...options] : options;
  return (
    <label className="field lad-assign">
      {label}
      <select
        className="input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">{none ?? t("lad.noVariable")}</option>
        {list.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
    </label>
  );
}

/** A variable or a literal; literals are checked by the compiler in Rust. */
function OperandEditor({
  t,
  label,
  value,
  options,
  onChange,
}: {
  t: T;
  label: string;
  value: Operand;
  options: string[];
  onChange: (o: Operand) => void;
}) {
  const isVariable = "variable" in value;
  const [text, setText] = useState(isVariable ? "" : value.constant);
  return (
    <fieldset className="lad-operand">
      <legend>{label}</legend>
      <select
        className="input"
        aria-label={t("lad.operandKind", { operand: label })}
        value={isVariable ? "variable" : "constant"}
        onChange={(e) =>
          onChange(
            e.target.value === "variable"
              ? { variable: options[0] ?? "" }
              : { constant: text || "0" },
          )
        }
      >
        <option value="variable">{t("lad.operand.variable")}</option>
        <option value="constant">{t("lad.operand.constant")}</option>
      </select>
      {isVariable ? (
        <select
          className="input"
          aria-label={t("lad.operandVariable", { operand: label })}
          value={value.variable}
          onChange={(e) => onChange({ variable: e.target.value })}
        >
          <option value="">{t("lad.noVariable")}</option>
          {options.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      ) : (
        <input
          className="input"
          aria-label={t("lad.operandConstant", { operand: label })}
          value={text}
          size={9}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => text !== value.constant && onChange({ constant: text })}
          onKeyDown={(e) => {
            if (e.key === "Enter") onChange({ constant: text });
          }}
        />
      )}
    </fieldset>
  );
}

function InstanceName({
  t,
  value,
  onChange,
}: {
  t: T;
  value: string;
  onChange: (v: string) => void;
}) {
  const [text, setText] = useState(value);
  return (
    <label className="field">
      {t("lad.instance")}
      <input
        className="input mono"
        value={text}
        size={10}
        maxLength={40}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => text.trim() && text !== value && onChange(text.trim())}
        onKeyDown={(e) => {
          if (e.key === "Enter" && text.trim()) onChange(text.trim());
        }}
      />
    </label>
  );
}

import { useState } from "react";
import type {
  RuntimeSnapshotDto,
  RuntimeValue,
  RuntimeVariable,
} from "@plc-lab/ipc";
import {
  formatRuntimeValue,
  inputDraft,
  requestedInput,
} from "./runtime-values";

function InputEditor({
  variable,
  disabled,
  commit,
}: {
  variable: RuntimeVariable;
  disabled: boolean;
  commit: (value: RuntimeValue) => Promise<void>;
}) {
  const [draft, setDraft] = useState(() => inputDraft(variable.value)),
    [error, setError] = useState<string | null>(null);
  if (variable.value.type === "BOOL")
    return (
      <input
        type="checkbox"
        aria-label={`Set ${variable.name}`}
        checked={variable.value.value}
        disabled={disabled}
        onChange={(event) => {
          void commit({ type: "BOOL", value: event.target.checked });
        }}
      />
    );
  const apply = () => {
    try {
      const value = requestedInput(variable.typeName, draft);
      setError(null);
      void commit(value);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Invalid input value.");
    }
  };
  return (
    <div className="numeric-input">
      <input
        aria-label={`Set ${variable.name}${variable.typeName === "TIME" ? " (microseconds)" : ""}`}
        value={draft}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            apply();
          }
        }}
      />
      <button
        type="button"
        aria-label={`Apply ${variable.name}`}
        disabled={disabled}
        onClick={apply}
      >
        Apply
      </button>
      {error && <span role="alert">{error}</span>}
    </div>
  );
}
export function RuntimeVariables({
  snapshot,
  editable,
  observationKey,
  setInput,
}: {
  snapshot: RuntimeSnapshotDto;
  editable: boolean;
  observationKey: string;
  setInput: (id: string, value: RuntimeValue) => Promise<void>;
}) {
  return (
    <div
      className="declarations live-variables"
      data-testid="runtime-variables"
    >
      {snapshot.variables.map((variable) => (
        <div
          className="declaration-row runtime-variable-row"
          key={variable.variableId}
          data-testid={`runtime-variable-${variable.name}`}
        >
          <span title={variable.name}>{variable.name}</span>
          <span>{variable.typeName}</span>
          <span>
            {variable.role}
            {variable.inputAuthority && (
              <small
                className="input-authority"
                aria-label={`Authority ${variable.name}`}
              >
                {variable.inputAuthority}
              </small>
            )}
          </span>
          <div className="runtime-value-cell">
            <output aria-label={`Value ${variable.name}`}>
              {formatRuntimeValue(variable.value)}
            </output>
            {variable.role === "Input" &&
              variable.inputAuthority === "USER" && (
                <InputEditor
                  key={`${snapshot.sessionId}/${observationKey}/${variable.variableId}`}
                  variable={variable}
                  disabled={!editable}
                  commit={(value) => setInput(variable.variableId, value)}
                />
              )}
          </div>
        </div>
      ))}
      {snapshot.functionBlocks.map((block) => (
        <div
          className="runtime-fb"
          key={block.instanceId}
          data-testid={`runtime-fb-${block.name}`}
        >
          <strong>
            {block.name} · {block.kind}
          </strong>
          <output aria-label={`Q ${block.name}`}>
            Q = {block.q ? "TRUE" : "FALSE"}
          </output>
          <output aria-label={`ET ${block.name}`}>
            ET ={" "}
            {formatRuntimeValue({ type: "TIME", microseconds: block.etMicros })}
          </output>
        </div>
      ))}
      <p className="declaration-note">
        Inputs are staged for the next scan. Apply numeric drafts explicitly.
      </p>
    </div>
  );
}

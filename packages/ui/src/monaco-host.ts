import * as monaco from "monaco-editor/editor/editor.api.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import type { CompilerDiagnostic } from "@plc-lab/ipc";
import { toEditorRange } from "./source-spans";

const environment = self as typeof self & {
  MonacoEnvironment: monaco.Environment;
};
environment.MonacoEnvironment = { getWorker: () => new EditorWorker() };
export const ST_LANGUAGE_ID = "plc-lab-st";
const MARKER_OWNER = "plc-lab-st-compiler";

// This tokenizer controls appearance only; all validation and semantics are Rust-owned.
if (
  !monaco.languages
    .getLanguages()
    .some((language) => language.id === ST_LANGUAGE_ID)
) {
  monaco.languages.register({
    id: ST_LANGUAGE_ID,
    extensions: [".st"],
    aliases: ["PLC Lab Structured Text"],
  });
  monaco.languages.setMonarchTokensProvider(ST_LANGUAGE_ID, {
    ignoreCase: true,
    keywords: [
      "PROGRAM",
      "END_PROGRAM",
      "VAR_INPUT",
      "VAR_OUTPUT",
      "VAR",
      "END_VAR",
      "IF",
      "THEN",
      "ELSE",
      "END_IF",
      "NOT",
      "AND",
      "OR",
    ],
    types: ["BOOL", "INT", "REAL", "TIME", "TON"],
    tokenizer: {
      root: [
        [/\(\*/, "comment", "@comment"],
        [/\/\/.*$/, "comment"],
        [/\b(?:T|TIME)#[0-9A-Za-z.]+/, "number"],
        [/\b(?:TRUE|FALSE)\b/, "constant"],
        [
          /[A-Za-z_][A-Za-z0-9_]*/,
          {
            cases: {
              "@keywords": "keyword",
              "@types": "type",
              "@default": "identifier",
            },
          },
        ],
        [/\d+\.\d+(?:[eE][+-]?\d+)?/, "number.float"],
        [/\d+/, "number"],
        [/:=|=>|<>|<=|>=|[+\-*/=<>]/, "operator"],
        [/[(),;:]/, "delimiter"],
      ],
      comment: [
        [/\*\)/, "comment", "@pop"],
        [/[^*]+/, "comment"],
        [/\*/, "comment"],
      ],
    },
  });
  monaco.languages.setLanguageConfiguration(ST_LANGUAGE_ID, {
    comments: { lineComment: "//", blockComment: ["(*", "*)"] },
    brackets: [["(", ")"]],
    autoClosingPairs: [{ open: "(", close: ")" }],
  });
}

export function createStEditor(
  element: HTMLElement,
  source: string,
  onChange: (source: string) => void,
) {
  const model = monaco.editor.createModel(source, ST_LANGUAGE_ID);
  const editor = monaco.editor.create(element, {
    model,
    theme: "vs-dark",
    automaticLayout: true,
    minimap: { enabled: false },
    fontFamily: "Consolas, monospace",
    fontSize: 13,
    scrollBeyondLastLine: false,
    wordWrap: "on",
    ariaLabel: "Structured Text source",
    padding: { top: 8 },
  });
  const subscription = model.onDidChangeContent(() =>
    onChange(model.getValue()),
  );
  return {
    update(value: string, diagnostics: CompilerDiagnostic[]) {
      if (model.getValue() !== value) model.setValue(value);
      monaco.editor.setModelMarkers(
        model,
        MARKER_OWNER,
        diagnostics.flatMap((d) =>
          d.span
            ? [
                {
                  ...toEditorRange(value, d.span),
                  severity:
                    d.severity === "error"
                      ? monaco.MarkerSeverity.Error
                      : d.severity === "warning"
                        ? monaco.MarkerSeverity.Warning
                        : monaco.MarkerSeverity.Info,
                  message: d.message,
                  code: d.code,
                  source: "Rust ST compiler",
                },
              ]
            : [],
        ),
      );
    },
    focus(span: CompilerDiagnostic["span"]) {
      if (span) {
        const range = toEditorRange(model.getValue(), span);
        editor.setSelection(range);
        editor.revealRangeInCenter(range);
        editor.focus();
      }
    },
    dispose() {
      subscription.dispose();
      editor.dispose();
      model.dispose();
    },
  };
}

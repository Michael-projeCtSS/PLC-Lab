import * as monaco from "monaco-editor/editor/editor.api.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import type { DiagnosticDto } from "@plc-lab/ipc";
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
      "ELSIF",
      "ELSE",
      "END_IF",
      "CASE",
      "OF",
      "END_CASE",
      "NOT",
      "AND",
      "OR",
      "XOR",
      "MOD",
    ],
    types: [
      "BOOL",
      "INT",
      "REAL",
      "TIME",
      "TON",
      "TOF",
      "TP",
      "R_TRIG",
      "F_TRIG",
      "CTU",
      "CTD",
      "INT_TO_REAL",
      "REAL_TO_INT",
    ],
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

monaco.editor.defineTheme("plc-lab-dark", {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "keyword", foreground: "7fb4e0" },
    { token: "type", foreground: "8fc9b0" },
    { token: "constant", foreground: "d8b56a" },
    { token: "number", foreground: "c5a3e0" },
    { token: "comment", foreground: "6b7b88", fontStyle: "italic" },
  ],
  colors: {
    "editor.background": "#14191f",
    "editor.lineHighlightBackground": "#1b222a",
    "editorLineNumber.foreground": "#4b5864",
  },
});

export function createStEditor(
  element: HTMLElement,
  source: string,
  onChange: (source: string) => void,
) {
  const model = monaco.editor.createModel(source, ST_LANGUAGE_ID);
  const editor = monaco.editor.create(element, {
    model,
    theme: "plc-lab-dark",
    automaticLayout: true,
    minimap: { enabled: false },
    fontFamily: "Cascadia Mono, Consolas, monospace",
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
    update(
      value: string,
      diagnostics: DiagnosticDto[],
      runtimeDiagnostics: DiagnosticDto[] = [],
      readOnly = false,
    ) {
      editor.updateOptions({ readOnly });
      if (model.getValue() !== value) model.setValue(value);
      const markers = (owner: string, items: DiagnosticDto[], source: string) =>
        monaco.editor.setModelMarkers(
          model,
          owner,
          items.flatMap((d) =>
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
                    source,
                  },
                ]
              : [],
          ),
        );
      markers(MARKER_OWNER, diagnostics, "Rust ST compiler");
      markers("plc-lab-runtime", runtimeDiagnostics, "Rust PLC runtime");
    },
    focus(span: DiagnosticDto["span"]) {
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

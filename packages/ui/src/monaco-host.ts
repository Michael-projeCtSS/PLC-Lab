import * as monaco from "monaco-editor/editor/editor.api.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import type { DiagnosticDto } from "@plc-lab/ipc";
import { EditorSync, type SourceDiagnostics } from "./editor-sync";
import { tryEditorRange } from "./source-spans";

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
      "FUNCTION",
      "END_FUNCTION",
      "FUNCTION_BLOCK",
      "END_FUNCTION_BLOCK",
      "VAR_IN_OUT",
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

monaco.editor.defineTheme("plc-lab-light", {
  base: "vs",
  inherit: true,
  rules: [
    { token: "keyword", foreground: "1b5fa8" },
    { token: "type", foreground: "17734f" },
    { token: "constant", foreground: "855a00" },
    { token: "number", foreground: "7440a6" },
    { token: "comment", foreground: "5d6c79", fontStyle: "italic" },
  ],
  colors: {
    "editor.background": "#ffffff",
    "editor.lineHighlightBackground": "#eef2f6",
    "editorLineNumber.foreground": "#6b7987",
  },
});

export type EditorTheme = "dark" | "light";
const MONACO_THEME: Record<EditorTheme, string> = {
  dark: "plc-lab-dark",
  light: "plc-lab-light",
};

/** Switch every editor's theme in place: models, undo history and selections stay. */
export function setEditorTheme(theme: EditorTheme) {
  monaco.editor.setTheme(MONACO_THEME[theme]);
}

const SEVERITY = {
  error: monaco.MarkerSeverity.Error,
  warning: monaco.MarkerSeverity.Warning,
  info: monaco.MarkerSeverity.Info,
} as const;

export function createStEditor(
  element: HTMLElement,
  source: string,
  onChange: (source: string) => void,
  theme: EditorTheme = "dark",
  report: (problem: string) => void = () => undefined,
) {
  const model = monaco.editor.createModel(source, ST_LANGUAGE_ID);
  const editor = monaco.editor.create(element, {
    model,
    theme: MONACO_THEME[theme],
    automaticLayout: true,
    minimap: { enabled: false },
    fontFamily: "Cascadia Mono, Consolas, monospace",
    fontSize: 13,
    scrollBeyondLastLine: false,
    wordWrap: "on",
    ariaLabel: "Structured Text source",
    padding: { top: 8 },
  });
  const sync = new EditorSync();
  let replacing = false;
  const subscription = model.onDidChangeContent(() => {
    if (replacing) return;
    const text = model.getValue();
    sync.emitted(text);
    onChange(text);
  });
  /**
   * Markers of one owner, only for diagnostics computed for exactly the current text.
   * Diagnostics of an earlier text are skipped: the next compile result follows within a
   * fraction of a second, and positions of an earlier text would point to the wrong
   * place or outside the text.
   */
  const markers = (owner: string, set: SourceDiagnostics, label: string) => {
    const text = model.getValue();
    if (set.source !== text) return;
    const list: monaco.editor.IMarkerData[] = [];
    for (const d of set.items) {
      if (!d.span) continue;
      const range = tryEditorRange(text, d.span);
      if (!range) {
        report(`${label} ${d.code}: position outside the source`);
        continue;
      }
      list.push({
        ...range,
        severity: SEVERITY[d.severity],
        message: d.message,
        code: d.code,
        source: label,
      });
    }
    monaco.editor.setModelMarkers(model, owner, list);
  };
  return {
    update(
      value: string,
      compiler: SourceDiagnostics,
      runtime: SourceDiagnostics,
      readOnly = false,
    ) {
      editor.updateOptions({ readOnly });
      if (sync.reconcile(value, model.getValue()) === "external") {
        // A program that was opened or replaced, not a keystroke of this editor.
        replacing = true;
        try {
          model.setValue(value);
        } finally {
          replacing = false;
        }
      }
      markers(MARKER_OWNER, compiler, "Rust ST compiler");
      markers("plc-lab-runtime", runtime, "Rust PLC runtime");
    },
    focus(span: DiagnosticDto["span"]) {
      if (!span) return;
      const range = tryEditorRange(model.getValue(), span);
      if (!range) return;
      editor.setSelection(range);
      editor.revealRangeInCenter(range);
      editor.focus();
    },
    dispose() {
      subscription.dispose();
      editor.dispose();
      model.dispose();
    },
  };
}

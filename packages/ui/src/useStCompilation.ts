import { useEffect, useRef, useState } from "react";
import { compileSt, CoreInfoError } from "@plc-lab/ipc";
import type { CompileResponse } from "@plc-lab/ipc";

export const DEFAULT_ST_SOURCE = `PROGRAM Main

VAR_INPUT
    Start : BOOL;
    Stop : BOOL;
END_VAR

VAR_OUTPUT
    Motor : BOOL;
END_VAR

Motor := Start AND NOT Stop;

END_PROGRAM
`;
export type CompilationState =
  | { kind: "not-compiled" }
  | { kind: "compiling" }
  | { kind: "result"; result: CompileResponse }
  | { kind: "transport-error"; code: string; message: string };

export function useStCompilation() {
  const [source, setSource] = useState(DEFAULT_ST_SOURCE);
  const [state, setState] = useState<CompilationState>({
    kind: "not-compiled",
  });
  const revision = useRef(0n),
    request = useRef(0n),
    text = useRef(source),
    mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      request.current += 1n;
    };
  }, []);
  const edit = (value: string) => {
    if (text.current === value) return;
    text.current = value;
    revision.current += 1n;
    request.current += 1n;
    setSource(value);
    setState({ kind: "not-compiled" });
  };
  const compile = async () => {
    const id = ++request.current,
      sourceRevision = revision.current;
    setState({ kind: "compiling" });
    const current = () =>
      mounted.current &&
      request.current === id &&
      revision.current === sourceRevision;
    try {
      const result = await compileSt(text.current);
      if (current()) setState({ kind: "result", result });
    } catch (error: unknown) {
      if (current())
        setState({
          kind: "transport-error",
          code:
            error instanceof CoreInfoError ? error.code : "APP-INTERNAL-001",
          message:
            error instanceof Error
              ? error.message
              : "Compilation request failed.",
        });
    }
  };
  return { source, edit, compile, state };
}

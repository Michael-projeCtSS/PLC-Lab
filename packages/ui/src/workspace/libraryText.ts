/**
 * Library browser text (ADR-0044): signatures and usage examples composed from the
 * backend's signatures. Examples are Structured Text and the same in every language.
 */
import type { PouDto, StandardBlockDto } from "@plc-lab/ipc";
import examples from "./standard-examples.json";

/**
 * Usage examples of the standard blocks; languages/st/tests/pou.rs compiles every one
 * from the same file.
 */
export const STANDARD_EXAMPLES: Record<string, string> = examples;

type Parameter = { name: string; dataType: string; direction: string };

/** `Name(A : BOOL, B : INT) : INT` or `Name(In : BOOL) => Out : BOOL`. */
export function signature(
  name: string,
  parameters: Parameter[],
  returnType: string | null,
): string {
  const ins = parameters
    .filter((p) => p.direction !== "Output")
    .map(
      (p) =>
        `${p.direction === "InOut" ? "VAR_IN_OUT " : ""}${p.name} : ${p.dataType}`,
    )
    .join(", ");
  const outs = parameters
    .filter((p) => p.direction === "Output")
    .map((p) => `${p.name} : ${p.dataType}`)
    .join(", ");
  return `${name}(${ins})${returnType ? ` : ${returnType}` : ""}${outs ? ` => ${outs}` : ""}`;
}

/** An example that uses a user POU as the compiler expects it. */
export function pouExample(pou: PouDto): string {
  const inputs = pou.parameters.filter((p) => p.direction === "Input");
  if (pou.kind === "FUNCTION")
    return `Result := ${pou.name}(${inputs.map((p) => p.name).join(", ")});`;
  const instance = `${pou.name}1`;
  const bindings = pou.parameters.map((p) =>
    p.direction === "Output"
      ? `${p.name} => ${p.name}Out`
      : `${p.name} := ${p.name}Var`,
  );
  return `VAR ${instance} : ${pou.name}; END_VAR\n${instance}(${bindings.join(", ")});`;
}

export function standardSignature(block: StandardBlockDto): string {
  return signature(block.name, block.parameters, null);
}

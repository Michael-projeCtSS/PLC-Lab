import type { RuntimeSnapshotDto } from "@plc-lab/ipc";
import TANK_EXAMPLE_SOURCE from "../../../scenes/tank-filling/example.st?raw";
export { TANK_EXAMPLE_SOURCE };
export function tankInterfaceCompatible(
  snapshot: RuntimeSnapshotDto | null,
): boolean {
  return snapshot !== null && tankInterfaceMismatch(snapshot) === null;
}
export function tankInterfaceMismatch(
  snapshot: RuntimeSnapshotDto | null,
): string | null {
  if (!snapshot) return null;
  const roleName = (role: string) =>
    role === "Input" ? "VAR_INPUT" : role === "Output" ? "VAR_OUTPUT" : "VAR";
  for (const [name, type, role] of [
    ["LevelLow", "BOOL", "Input"],
    ["LevelHigh", "BOOL", "Input"],
    ["LevelPercent", "REAL", "Input"],
    ["InletValve", "BOOL", "Output"],
    ["DrainPump", "BOOL", "Output"],
  ] as const) {
    const found = snapshot.variables.find(
      (v) => v.name.toUpperCase() === name.toUpperCase(),
    );
    const expected = `${name} : ${type} ${roleName(role)}`;
    if (!found)
      return `Tank scene requires ${expected}. The variable is missing.`;
    if (found.typeName !== type || found.role !== role)
      return `Tank scene requires ${expected}. Found ${found.name} : ${found.typeName} ${roleName(found.role)}.`;
  }
  return null;
}

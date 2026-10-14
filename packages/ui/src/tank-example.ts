import type { RuntimeSnapshotDto } from "@plc-lab/ipc";
import TANK_EXAMPLE_SOURCE from "../../../scenes/tank-filling/example.st?raw";
export { TANK_EXAMPLE_SOURCE };
export function tankInterfaceCompatible(
  snapshot: RuntimeSnapshotDto | null,
): boolean {
  if (!snapshot) return false;
  return (
    [
      ["LevelLow", "BOOL", "Input"],
      ["LevelHigh", "BOOL", "Input"],
      ["LevelPercent", "REAL", "Input"],
      ["InletValve", "BOOL", "Output"],
      ["DrainPump", "BOOL", "Output"],
    ] as const
  ).every(([name, type, role]) =>
    snapshot.variables.some(
      (v) =>
        v.name.toUpperCase() === name.toUpperCase() &&
        v.typeName === type &&
        v.role === role,
    ),
  );
}

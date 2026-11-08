/** The exercise's sequence definition for the workspace views (ADR-0043). */
import type { SequenceDefinitionDto } from "@plc-lab/ipc";
import { useInterface } from "../locale";
import { localizeContent } from "../locale/content";
import type { Lab } from "./useLab";

/** The exercise's sequence definition in the interface language, if it has one. */
export function useSequenceDefinition(lab: Lab): SequenceDefinitionDto | null {
  const { preferences } = useInterface();
  const exercise = lab.workspace?.exercise ?? null;
  return exercise?.sequence
    ? (localizeContent(exercise, preferences.language).sequence ?? null)
    : null;
}

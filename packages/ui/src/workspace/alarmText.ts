/** Alarm presentation in the interface language: conditions, priorities and states. */
import type {
  AlarmDto,
  AlarmEventDto,
  SequenceDefinitionDto,
} from "@plc-lab/ipc";
import type { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";

type Translate = ReturnType<typeof useInterface>["t"];
const CONDITION: Record<string, TranslationKey> = {
  OVERFLOW: "alarm.code.OVERFLOW",
  HIGH_LEVEL: "alarm.code.HIGH_LEVEL",
  LOW_LEVEL: "alarm.code.LOW_LEVEL",
  DRY_RUN: "alarm.code.DRY_RUN",
  MOTOR_FAULT: "alarm.code.MOTOR_FAULT",
  DRIVE_FAULT: "alarm.code.DRIVE_FAULT",
  DRIVE_NOT_READY: "alarm.code.DRIVE_NOT_READY",
  PROCESS_TIMEOUT: "alarm.code.PROCESS_TIMEOUT",
  SENSOR_INVALID: "alarm.code.SENSOR_INVALID",
  FIELD_POWER_LOST: "alarm.code.FIELD_POWER_LOST",
  MODULE_FAULT: "alarm.code.MODULE_FAULT",
  COMMUNICATION_LOST: "alarm.code.COMMUNICATION_LOST",
  SEQUENCE_TIMEOUT: "alarm.code.SEQUENCE_TIMEOUT",
  SEQUENCE_DEVIATION: "alarm.code.SEQUENCE_DEVIATION",
  SEQUENCE_INTERLOCK: "alarm.code.SEQUENCE_INTERLOCK",
};
const PRIORITY: Record<AlarmDto["priority"], TranslationKey> = {
  CRITICAL: "alarm.priority.CRITICAL",
  ALARM: "alarm.priority.ALARM",
  WARNING: "alarm.priority.WARNING",
  INFORMATION: "alarm.priority.INFORMATION",
};
export const PRIORITIES: AlarmDto["priority"][] = [
  "CRITICAL",
  "ALARM",
  "WARNING",
  "INFORMATION",
];
export const STATES: AlarmDto["state"][] = [
  "ACTIVE_UNACKNOWLEDGED",
  "ACTIVE_ACKNOWLEDGED",
  "RETURNED_UNACKNOWLEDGED",
  "NORMAL",
];
export const CATEGORY: Record<AlarmDto["category"], TranslationKey> = {
  PROCESS: "alarm.category.PROCESS",
  HARDWARE: "alarm.category.HARDWARE",
  NETWORK: "alarm.category.NETWORK",
};
export const STATE: Record<AlarmDto["state"], TranslationKey> = {
  NORMAL: "alarm.state.NORMAL",
  ACTIVE_UNACKNOWLEDGED: "alarm.state.ACTIVE_UNACKNOWLEDGED",
  ACTIVE_ACKNOWLEDGED: "alarm.state.ACTIVE_ACKNOWLEDGED",
  RETURNED_UNACKNOWLEDGED: "alarm.state.RETURNED_UNACKNOWLEDGED",
};
export const TRANSITION: Record<AlarmEventDto["transition"], TranslationKey> = {
  ACTIVATED: "alarm.transition.ACTIVATED",
  ACKNOWLEDGED: "alarm.transition.ACKNOWLEDGED",
  RETURNED_TO_NORMAL: "alarm.transition.RETURNED_TO_NORMAL",
  WITHDRAWN: "alarm.transition.WITHDRAWN",
};

/** An alarm's source as shown: content names are localized, product labels are not. */
export type AlarmLabel = (a: { id: string; sourceLabel: string }) => string;
export const plainLabel: AlarmLabel = (a) => a.sourceLabel;

/**
 * Sequence alarms (`sequence.<id>.…`) carry the English sequence or interlock name; the
 * workspace shows the localized name from the exercise's sequence definition.
 */
export function sequenceAlarmLabel(
  sequence: SequenceDefinitionDto | null,
): AlarmLabel {
  if (!sequence) return plainLabel;
  const prefix = `sequence.${sequence.id}.`;
  return (a) => {
    if (!a.id.startsWith(prefix)) return a.sourceLabel;
    const rest = a.id.slice(prefix.length);
    const interlock = rest.startsWith("interlock.")
      ? sequence.interlocks.find(
          (i) => i.id === rest.slice("interlock.".length),
        )
      : undefined;
    return interlock?.name ?? sequence.name;
  };
}

/** The condition text in the interface language; the canonical text for unknown codes. */
export function alarmCondition(t: Translate, code: string, text: string) {
  const key = CONDITION[code];
  return key ? t(key) : text;
}
export function alarmPriority(t: Translate, priority: AlarmDto["priority"]) {
  return t(PRIORITY[priority]);
}
export function priorityBadge(priority: AlarmDto["priority"]) {
  return priority === "CRITICAL" || priority === "ALARM" ? "fault" : "warning";
}
export function needsAcknowledgment(a: AlarmDto) {
  return (
    a.state === "ACTIVE_UNACKNOWLEDGED" || a.state === "RETURNED_UNACKNOWLEDGED"
  );
}

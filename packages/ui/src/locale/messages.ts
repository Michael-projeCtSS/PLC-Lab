/**
 * Localized text for structured backend messages. The backend sends a stable code plus
 * typed values; the interface chooses the wording and keeps the canonical English text
 * as the technical original. Unknown codes fall back to that English text, never to a
 * guess.
 */
import type { DiagnosticDto, DiagnosticEntryDto, EventDto } from "@plc-lab/ipc";
import { en, type TranslationKey } from "./en";

export type Translate = (
  key: TranslationKey,
  values?: Record<string, string | number>,
) => string;

export function isTranslationKey(key: string): key is TranslationKey {
  return Object.prototype.hasOwnProperty.call(en, key);
}

/** The translation of a dynamic key, or `fallback` when the key is not in the dictionary. */
export function translateCode(
  t: Translate,
  key: string,
  fallback: string,
  values?: Record<string, string | number>,
): string {
  return isTranslationKey(key) ? t(key, values) : fallback;
}

/** Enumerated detail values such as "none", "warm" and "cold"; anything else is data. */
function value(t: Translate, raw: string): string {
  return translateCode(t, `diag.value.${raw}`, raw);
}

function detail(entry: DiagnosticEntryDto, name: string): string | undefined {
  return entry.details.find((d) => d.name === name)?.value;
}

export function modbusException(t: Translate, code: string): string {
  return translateCode(
    t,
    `modbus.exception.${code}`,
    t("modbus.exception.unknown"),
  );
}

/** The diagnostic message for the entry's kind and details, or its English original. */
export function diagnosticText(
  t: Translate,
  entry: DiagnosticEntryDto,
): string {
  const start = detail(entry, "start");
  switch (entry.kind) {
    case "ModuleMismatch":
      return t("diag.kind.ModuleMismatch", {
        configured: value(t, detail(entry, "configured") ?? "none"),
        installed: detail(entry, "installed") ?? "",
      });
    case "ModbusException": {
      const code = detail(entry, "exception") ?? "";
      return t("diag.kind.ModbusException", {
        exception: code,
        name: modbusException(t, code),
      });
    }
    case "ModeChange":
      return t("diag.kind.ModeChange", {
        from: detail(entry, "from") ?? "",
        to: detail(entry, "to") ?? "",
      });
    case "Startup":
    case "FirstScan":
      return translateCode(t, `diag.kind.${entry.kind}.${start}`, entry.text, {
        retained: detail(entry, "retained") ?? "",
      });
    default:
      return translateCode(t, `diag.kind.${entry.kind}`, entry.text);
  }
}

/** The source of the entry from its typed slot and channel; other sources are names. */
export function diagnosticSource(
  t: Translate,
  entry: DiagnosticEntryDto,
): string {
  if (entry.channel)
    return `${t("diag.source.channel", entry.channel)} (${detail(entry, "address") ?? ""})`;
  if (entry.slot) return t("diag.source.module", entry.slot);
  return entry.source;
}

export function diagnosticDetails(
  t: Translate,
  entry: DiagnosticEntryDto,
): string {
  return entry.details
    .map(
      (d) =>
        `${translateCode(t, `diag.detail.${d.name}`, d.name)}: ${value(t, d.value)}`,
    )
    .join(", ");
}

/** A module status code ("NO_FIELD_POWER", ...) as a badge label. */
export function moduleStatus(t: Translate, status: string): string {
  return translateCode(t, `hw.status.${status}`, status.replaceAll("_", " "));
}
/** A channel quality code ("WIRE_BREAK", ...) as a lower-case word. */
export function qualityLabel(t: Translate, quality: string): string {
  return translateCode(
    t,
    `q.${quality}`,
    quality.replaceAll("_", " ").toLowerCase(),
  );
}

/** The event category ("PLC_MODE", "PROCESS", ...) as a lower-case label. */
export function eventKind(t: Translate, kind: string): string {
  return translateCode(
    t,
    `event.kind.${kind}`,
    kind.replaceAll("_", " ").toLowerCase(),
  );
}

/** The event text: its structured message when the backend sent one, else the original. */
export function eventText(_t: Translate, event: EventDto): string {
  return event.text;
}

/** The localized title of a compiler or runtime diagnostic code, after a separator. */
export function diagnosticTitle(t: Translate, code: string): string {
  const key = `code.${code}`;
  return isTranslationKey(key) ? ` · ${t(key)}` : "";
}

/** One diagnostic for the message log: line, code, title and the original message. */
export function diagnosticLine(t: Translate, d: DiagnosticDto): string {
  const line = d.span ? t("toolbar.line", { line: d.span.start.line }) : "";
  return `${line}${d.code}${diagnosticTitle(t, d.code)}: ${d.message}`;
}

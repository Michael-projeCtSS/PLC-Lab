/**
 * Localized text for structured backend messages. The backend sends a stable code plus
 * typed values; the interface chooses the wording and keeps the canonical English text
 * as the technical original. Unknown codes fall back to that English text, never to a
 * guess.
 */
import type {
  DiagnosticDto,
  DiagnosticEntryDto,
  EventDto,
  MessageArgDto,
  MessageDto,
} from "@plc-lab/ipc";
import { translate } from ".";
import { backendPl } from "./backend-pl";
import { en, type TranslationKey } from "./en";
import type { Preferences } from "./preferences";

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
        configured: moduleText(
          t,
          value(t, detail(entry, "configured") ?? "none"),
        ),
        installed: moduleText(t, detail(entry, "installed") ?? ""),
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
        `${translateCode(t, `diag.detail.${d.name}`, d.name)}: ${moduleText(t, value(t, d.value))}`,
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

/** The event text: its coded message when the backend sent one, else the original. */
export function eventText(
  language: Preferences["language"],
  event: EventDto,
): string {
  return backendText(language, event.message, event.text);
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

/**
 * A coded backend message in the viewer's language. English is the backend's own text;
 * Polish words the code from `backendPl` and each value by its kind: data as it is,
 * terms by their vocabulary code, nested messages recursively. A code without Polish
 * wording shows the English original.
 */
export function backendText(
  language: Preferences["language"],
  m: MessageDto | null | undefined,
  fallback = "",
): string {
  if (!m) return fallback;
  if (language !== "pl") return m.text;
  const value = (a: MessageArgDto): string => {
    if (a.message) return backendText(language, a.message);
    if (a.term === null) return a.text;
    const word = backendPl[a.term];
    if (word !== undefined) return word;
    return isTranslationKey(a.term) ? translate("pl", a.term) : a.text;
  };
  if (m.code === "list") return m.args.map(value).join(", ");
  const template = backendPl[m.code];
  if (template === undefined) return m.text;
  const values = new Map(m.args.map((a) => [a.name, value(a)]));
  return template.replace(
    /\{(\w+)\}/g,
    (match, name: string) => values.get(name) ?? match,
  );
}

const MODULE_TYPES = Object.keys(en).filter((key) =>
  /^module\.(cpu|ps|di|do|ai|ao)/.test(key),
) as TranslationKey[];

/**
 * A backend module name in the viewer's language: every generic module name inside it
 * ("Analog input 4 x U/I") is worded by its type; product names around it stay as sent.
 */
export function moduleText(t: Translate, text: string): string {
  let out = text;
  for (const key of MODULE_TYPES) out = out.replaceAll(en[key], t(key));
  return out;
}

import raw from "../../../../knowledge/locales/pl.json";
import type { SearchHitDto } from "@plc-lab/ipc";
import type { TranslationKey } from "./en";
type Item = {
  id: string;
  fields: { path: string; source: string; text: string }[];
};
const items = new Map<string, Item>(raw.items.map((item) => [item.id, item]));
export function hasCompleteTranslation(id: string): boolean {
  const item = raw.items.find((item) => item.id === id);
  return (
    !!item &&
    (item.file.startsWith("lessons/") ||
      item.file.startsWith("instructions/") ||
      (item.file.startsWith("exercises/") &&
        item.fields.some((field) => field.path === "instructions.0")))
  );
}
export function fidelityLabel(
  value: string,
  t: (key: TranslationKey) => string,
): string {
  const keys: Record<string, TranslationKey | undefined> = {
    EXACT_BEHAVIORAL: "fidelity.EXACT_BEHAVIORAL",
    APPROXIMATED_BEHAVIORAL: "fidelity.APPROXIMATED_BEHAVIORAL",
    CONCEPTUAL: "fidelity.CONCEPTUAL",
    NOT_IMPLEMENTED: "fidelity.NOT_IMPLEMENTED",
  };
  const key = keys[value];
  return key ? t(key) : value;
}
export function contentKindLabel(
  value: string,
  t: (key: TranslationKey) => string,
): string {
  const keys: Record<string, TranslationKey | undefined> = {
    exercise: "content.exercise",
    lesson: "content.lesson",
    article: "content.article",
    instruction: "content.instruction",
    protocol: "content.protocol",
    topic: "content.topic",
    family: "content.family",
    vendor: "content.vendor",
    hardware: "content.hardware",
    troubleshooting: "content.troubleshooting",
    diagnostic: "content.diagnostic",
  };
  const key = keys[value];
  return key ? t(key) : value;
}

/** Replace existing approved display strings only. Never add missing/hidden data. */
export function localizeContent<T>(
  data: T,
  language: "pl" | "en",
  itemId?: string,
): T {
  if (language === "en" || !data) return data;
  function visit(value: unknown, item: Item | undefined, at: string): unknown {
    if (typeof value === "string") {
      const paths = [
        at,
        at.replace(/\.feedback$/, ".onFail"),
        at.replace(/^debrief\./, "troubleshooting.debrief."),
      ];
      const field = item?.fields.find(
        (field) => paths.includes(field.path) && field.source === value,
      );
      return field?.text ?? value;
    }
    if (!value || typeof value !== "object") return value;
    if (Array.isArray(value))
      return value.map((entry, index) =>
        visit(entry, item, at ? `${at}.${index}` : String(index)),
      );
    const object = value as Record<string, unknown>;
    const own =
      typeof object.id === "string" ? items.get(object.id) : undefined;
    return Object.fromEntries(
      Object.entries(object).map(([key, child]) => [
        key,
        visit(child, own ?? item, own ? key : at ? `${at}.${key}` : key),
      ]),
    );
  }
  return visit(data, itemId ? items.get(itemId) : undefined, "") as T;
}

const fold = (value: string) =>
  value
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replaceAll("ł", "l");
export function englishSearchAlias(query: string): string {
  const aliases: Record<string, string> = {
    falownik: "VFD",
    sterownik: "PLC",
    wymuszenie: "force",
  };
  return query
    .split(/(\s+)/)
    .map((word) => aliases[fold(word)] ?? word)
    .join("");
}
/** Search only public prose from installed items, never hints, repair options or debriefs. */
export function searchPolishContent(
  query: string,
  available: readonly { id: string; title: string; summary: string }[],
  category: string | null,
): SearchHitDto[] {
  const words = fold(query)
    .split(/[^a-z0-9_]+/)
    .filter(Boolean);
  if (!words.length) return [];
  const installed = new Map(available.map((item) => [item.id, item]));
  return raw.items
    .flatMap((item) => {
      const meta = installed.get(item.id);
      const title = item.fields.find((field) => field.path === "title");
      const summary = item.fields.find((field) => field.path === "summary");
      if (
        !meta ||
        !title ||
        !summary ||
        (meta.title !== title.source && meta.title !== title.text)
      )
        return [];
      const group = item.file.split("/")[0];
      const itemCategory =
        group === "troubleshooting"
          ? "TROUBLESHOOTING"
          : group === "vendors"
            ? "VENDOR_SPECIFIC"
            : group === "exercises" || group === "lessons"
              ? "PLC_LAB_SIMULATION"
              : "FUNDAMENTAL";
      if (category && category !== itemCategory) return [];
      const text = item.fields
        .filter((field) =>
          /^(title|summary|objectives\.\d+|sections\.\d+\.(heading|body))$/.test(
            field.path,
          ),
        )
        .map((field) => field.text)
        .join(" ");
      if (!words.every((word) => fold(text).includes(word))) return [];
      return [
        {
          id: item.id,
          docType:
            group === "lessons"
              ? "lesson"
              : group === "exercises"
                ? "exercise"
                : "article",
          category: itemCategory,
          title: title.text,
          summary: summary.text,
          vendor: null,
          snippet: summary.text,
        },
      ];
    })
    .sort((a, b) => {
      const score = (hit: SearchHitDto) =>
        words.filter((word) => fold(hit.title).includes(word)).length;
      return score(b) - score(a) || a.id.localeCompare(b.id);
    })
    .slice(0, 40);
}

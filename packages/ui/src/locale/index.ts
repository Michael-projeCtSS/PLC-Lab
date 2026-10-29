import { useContext, useCallback } from "react";
import { Context } from "./context";
import { en, type TranslationKey } from "./en";
import { pl } from "./pl";
import { defaults, type Preferences } from "./preferences";
export { PreferencesProvider } from "./PreferencesProvider";
export function translate(
  language: Preferences["language"],
  key: TranslationKey,
  values: Record<string, string | number> = {},
  polish: Partial<Record<TranslationKey, string>> = pl,
): string {
  const template = (language === "pl" ? polish[key] : en[key]) ?? en[key];
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    String(values[name] ?? match),
  );
}
export function usePreferences() {
  return useContext(Context);
}
export function useInterface() {
  const context = useContext(Context);
  const preferences = context?.preferences ?? defaults("en");
  const t = useCallback(
    (key: TranslationKey, values?: Record<string, string | number>) =>
      translate(preferences.language, key, values),
    [preferences.language],
  );
  return { t, preferences };
}

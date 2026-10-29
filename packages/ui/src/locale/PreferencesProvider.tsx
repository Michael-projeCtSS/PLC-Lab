import { useState, useCallback, useEffect, type ReactNode } from "react";
import { Context, type Interface } from "./context";
import {
  PREFERENCES_KEY,
  readPreferences,
  type Preferences,
} from "./preferences";
export function PreferencesProvider({ children }: { children: ReactNode }) {
  const [initial] = useState(() => readPreferences(navigator.language));
  const [preferences, setPreferences] = useState(initial.preferences);
  const [problem, setProblem] = useState<Interface["problem"]>(
    initial.problem ? "read" : null,
  );
  const update = useCallback(
    (patch: Partial<Omit<Preferences, "version">>) => {
      const next = { ...preferences, ...patch };
      setPreferences(next);
      try {
        localStorage.setItem(PREFERENCES_KEY, JSON.stringify(next));
        setProblem(null);
      } catch {
        setProblem("write");
      }
    },
    [preferences],
  );
  useEffect(() => {
    document.documentElement.lang = preferences.language;
  }, [preferences.language]);
  return (
    <Context.Provider value={{ preferences, update, problem }}>
      {children}
    </Context.Provider>
  );
}

import {
  useState,
  useCallback,
  useEffect,
  useLayoutEffect,
  type ReactNode,
} from "react";
import { Context, type Interface } from "./context";
import {
  PREFERENCES_KEY,
  readPreferences,
  type Preferences,
} from "./preferences";
const LIGHT_QUERY = "(prefers-color-scheme: light)";

/** The applied theme: the chosen one, or for "system" the operating system's, live. */
function useResolvedTheme(theme: Preferences["theme"]): "dark" | "light" {
  const query = () =>
    typeof window.matchMedia === "function"
      ? window.matchMedia(LIGHT_QUERY)
      : null;
  const [systemLight, setSystemLight] = useState(
    () => query()?.matches ?? false,
  );
  useEffect(() => {
    const media = query();
    if (!media) return;
    const change = () => setSystemLight(media.matches);
    change();
    media.addEventListener("change", change);
    return () => media.removeEventListener("change", change);
  }, []);
  if (theme === "system") return systemLight ? "light" : "dark";
  return theme;
}

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
  const theme = useResolvedTheme(preferences.theme);
  // Before paint, so a light theme never flashes dark after a restart.
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  return (
    <Context.Provider value={{ preferences, update, problem, theme }}>
      {children}
    </Context.Provider>
  );
}

import { createContext } from "react";
import type { Preferences } from "./preferences";
export type Interface = {
  preferences: Preferences;
  update: (patch: Partial<Omit<Preferences, "version">>) => void;
  problem: "read" | "write" | null;
  /** The applied theme ("system" resolved to the operating system's appearance). */
  theme: "dark" | "light";
};
export const Context = createContext<Interface | null>(null);

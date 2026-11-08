/**
 * Ownership of the Structured Text while it is edited: the Monaco model holds the text the
 * student types; React state follows it through change events. React re-renders and its
 * effects run after the browser has painted, so a source value from React can be an
 * earlier keystroke of the editor's own ("an echo") while the model already holds newer
 * text. Writing such a value back would undo the newer keystrokes and reset the cursor
 * and the undo history. Only a value the editor never emitted — a program that was opened
 * or replaced — is an external change that belongs in the model.
 */
import type { DiagnosticDto } from "@plc-lab/ipc";

export type Reconcile = "in-sync" | "echo" | "external";

/** Diagnostics together with the exact text they were computed for. */
export type SourceDiagnostics = {
  items: DiagnosticDto[];
  /** The text the compiler or the PLC saw; null when unknown. */
  source: string | null;
};

/** Emitted texts kept until React catches up; bounded so memory stays small. */
const MAX_PENDING = 64;

export class EditorSync {
  private pending: string[] = [];

  /** The editor reported a user edit with this text. */
  emitted(text: string) {
    this.pending.push(text);
    if (this.pending.length > MAX_PENDING) this.pending.shift();
  }

  /**
   * How a source value from React relates to the model's current text. React commits
   * values in emission order and may skip some, so a value is matched with the earliest
   * emitted text equal to it: later emissions (an Enter followed by a Backspace returns to
   * an earlier text) may still be on their way and stay recognised as echoes.
   */
  reconcile(value: string, current: string): Reconcile {
    const index = this.pending.indexOf(value);
    if (index >= 0) {
      this.pending = this.pending.slice(index + 1);
      return value === current ? "in-sync" : "echo";
    }
    this.pending = [];
    return value === current ? "in-sync" : "external";
  }
}

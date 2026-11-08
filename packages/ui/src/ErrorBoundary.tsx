import { Component, type ErrorInfo, type ReactNode } from "react";
import { translate } from "./locale";
import type { TranslationKey } from "./locale/en";
import { readPreferences } from "./locale/preferences";
import { currentDraft, recentProblems, recordProblem } from "./error-log";

type Props = {
  children: ReactNode;
  /** "page": the shell stays usable; "app": nothing else is left to show. */
  scope: "page" | "app";
  /** A new value (another page) clears a shown error; healthy children never remount. */
  resetKey?: string;
};
type State = { error: Error | null; stack: string | null };

/**
 * A component that throws must not take the whole window with it. This boundary records
 * the error, says what happened, shows the latest program text so nothing typed is lost,
 * and offers a fresh start of the interface. It never touches projects or the PLC.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null, stack: null };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  componentDidUpdate(previous: Props) {
    if (this.state.error && previous.resetKey !== this.props.resetKey)
      this.setState({ error: null, stack: null });
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    const stack = info.componentStack ?? null;
    this.setState({ stack });
    recordProblem(
      "render",
      error.message,
      `${error.stack ?? ""}\n${stack ?? ""}`,
    );
  }

  render() {
    const { error, stack } = this.state;
    if (!error) return this.props.children;
    const language = readPreferences(navigator.language).preferences.language;
    const t = (key: TranslationKey) => translate(language, key);
    const draft = currentDraft();
    const problems = recentProblems();
    return (
      <div className="error-screen" role="alert">
        <h2>{t("crash.title")}</h2>
        <p>{t("crash.text")}</p>
        {draft && (
          <>
            <h3>{t("crash.draft")}</h3>
            <p className="muted">{t("crash.draftHelp")}</p>
            <textarea
              className="input mono error-draft"
              readOnly
              value={draft.source}
              aria-label={t("crash.draft")}
            />
          </>
        )}
        <div className="row" style={{ gap: 8 }}>
          {draft && (
            <button
              className="btn"
              onClick={() =>
                void navigator.clipboard
                  ?.writeText(draft.source)
                  .catch(() => undefined)
              }
            >
              {t("crash.copy")}
            </button>
          )}
          {this.props.scope === "page" && (
            <button
              className="btn"
              onClick={() => this.setState({ error: null, stack: null })}
            >
              {t("crash.retry")}
            </button>
          )}
          <button
            className="btn btn-primary"
            onClick={() => window.location.reload()}
          >
            {t("crash.reload")}
          </button>
        </div>
        <details>
          <summary>{t("crash.details")}</summary>
          <pre className="error-details">
            {[
              error.stack ?? error.message,
              stack,
              ...problems.map((p) => `${p.time} ${p.kind}: ${p.message}`),
            ]
              .filter(Boolean)
              .join("\n\n")}
          </pre>
        </details>
      </div>
    );
  }
}

import { useInterface, usePreferences } from "../locale";
import { Modal } from "../ui";
import { AboutPage } from "./AboutPage";

export function Settings({ onClose }: { onClose: () => void }) {
  const { t, preferences } = useInterface();
  const context = usePreferences();
  if (!context) throw new Error("Settings requires PreferencesProvider");
  return (
    <Modal title={t("settings.title")} onClose={onClose}>
      <div className="settings-fields">
        <h3>{t("settings.general")}</h3>
        <label className="field">
          {t("settings.language")}
          <select
            className="input"
            value={preferences.language}
            onChange={(e) =>
              context.update({
                language: e.target.value === "pl" ? "pl" : "en",
              })
            }
            autoFocus
          >
            <option value="pl" lang="pl">
              Polski
            </option>
            <option value="en" lang="en">
              English
            </option>
          </select>
        </label>
        <label className="field">
          {t("settings.appearance")}
          <select className="input" value="dark" disabled>
            <option value="dark">{t("settings.dark")}</option>
          </select>
        </label>
        <p className="muted">{t("settings.darkHelp")}</p>
        <label className="field">
          {t("settings.density")}
          <select
            className="input"
            value={preferences.density}
            onChange={(e) =>
              context.update({
                density:
                  e.target.value === "compact" ? "compact" : "comfortable",
              })
            }
          >
            <option value="comfortable">{t("settings.comfortable")}</option>
            <option value="compact">{t("settings.compact")}</option>
          </select>
        </label>
        <label className="row">
          <input
            type="checkbox"
            checked={preferences.explanations}
            onChange={(e) => context.update({ explanations: e.target.checked })}
          />
          {t("settings.explanations")}
        </label>
        <p className="muted" role={context.problem ? "alert" : "status"}>
          {context.problem
            ? t(
                context.problem === "read"
                  ? "settings.storageReadError"
                  : "settings.storageError",
              )
            : t("settings.saved")}
        </p>
        <button className="btn btn-primary" onClick={onClose}>
          {t("common.close")}
        </button>
      </div>
      <details className="settings-about">
        <summary>{t("about.title")}</summary>
        <AboutPage />
      </details>
    </Modal>
  );
}

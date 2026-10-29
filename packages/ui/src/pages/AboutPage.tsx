import { useInterface } from "../locale";
import { api } from "@plc-lab/ipc";
import { Badge, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";

/** Product information and developer diagnostics (versions, contract, knowledge packs). */
export function AboutPage() {
  const { t } = useInterface();
  const info = useLoad(() => api.coreInfo(), []);
  const status = useLoad(() => api.knowledgeStatus(), []);
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>{t("about.title")}</h1>
          <p>{t("about.intro")}</p>
        </div>
      </div>
      <ErrorNotice error={info.error ?? status.error} />
      <div className="tiles">
        <div className="panel">
          <div className="panel-head">
            <h4>{t("about.application")}</h4>
          </div>
          <dl className="kv panel-body">
            <dt>{t("about.version")}</dt>
            <dd>{info.data?.appVersion ?? "—"}</dd>
            <dt>{t("about.build")}</dt>
            <dd>{info.data?.buildContext ?? "—"}</dd>
            <dt>{t("about.core")}</dt>
            <dd>{info.data?.coreVersion ?? "—"}</dd>
            <dt>{t("about.contract")}</dt>
            <dd>{info.data ? `v${info.data.contractVersion}` : "—"}</dd>
            <dt>{t("about.execution")}</dt>
            <dd>{info.data?.execution ?? "—"}</dd>
          </dl>
        </div>
        <div className="panel">
          <div className="panel-head">
            <h4>{t("about.packs")}</h4>
            <span className="spacer" />
            {status.data && (
              <span className="muted">
                {t("about.indexed", { count: status.data.indexedItems })}
              </span>
            )}
          </div>
          <div className="panel-body col">
            {status.data?.packs.length === 0 && (
              <p className="muted">{t("about.noPacks")}</p>
            )}
            {status.data?.packs.map((p) => (
              <div key={p.path} className="source-card">
                <div className="row">
                  <strong className="grow">{p.title ?? p.path}</strong>
                  <Badge
                    kind={
                      p.status === "LOADED"
                        ? "active"
                        : p.status === "PARTIAL"
                          ? "warning"
                          : "fault"
                    }
                  >
                    {t(
                      p.status === "LOADED"
                        ? "about.loaded"
                        : p.status === "PARTIAL"
                          ? "about.partial"
                          : "about.failed",
                    )}
                  </Badge>
                </div>
                <div className="muted">
                  {p.id} {p.version} ·{" "}
                  {t("about.counts", {
                    articles: p.articles,
                    lessons: p.lessons,
                    exercises: p.exercises,
                    scenes: p.scenes,
                    sources: p.sources,
                    profiles: p.profiles,
                  })}
                </div>
                <div className="dim mono" style={{ wordBreak: "break-all" }}>
                  {p.path}
                </div>
                {p.errors.map((e) => (
                  <div
                    key={e}
                    className="notice error"
                    style={{ marginTop: 4 }}
                  >
                    {e}
                  </div>
                ))}
                {p.warnings.map((w) => (
                  <div
                    key={w}
                    className="notice warning"
                    style={{ marginTop: 4 }}
                  >
                    {w}
                  </div>
                ))}
              </div>
            ))}
            {status.data && (
              <div className="muted" style={{ fontSize: 12 }}>
                {t("about.locations")}
                <ul>
                  {status.data.searchPaths.map((p) => (
                    <li
                      key={p}
                      className="mono"
                      style={{ wordBreak: "break-all" }}
                    >
                      {p}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

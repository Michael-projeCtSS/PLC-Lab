import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { api } from "@plc-lab/ipc";
import { Badge, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";

const DISTRIBUTION: Record<string, TranslationKey> = {
  DEVELOPMENT_RUN: "about.dist.DEVELOPMENT_RUN",
  RELEASE_BUILD: "about.dist.RELEASE_BUILD",
  UNSIGNED_DEVELOPMENT_BUILD: "about.dist.UNSIGNED_DEVELOPMENT_BUILD",
  SIGNED_PRODUCTION_RELEASE: "about.dist.SIGNED_PRODUCTION_RELEASE",
  MANIFEST_MISMATCH: "about.dist.MANIFEST_MISMATCH",
};

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
            <dt>{t("about.commit")}</dt>
            <dd
              className="mono"
              title={info.data?.sourceCommit ?? undefined}
              data-testid="about-commit"
            >
              {info.data?.sourceCommit
                ? info.data.sourceCommit.slice(0, 12)
                : t("about.unknown")}
            </dd>
            <dt>{t("about.builtAt")}</dt>
            <dd>
              {info.data?.builtAt
                ? new Date(info.data.builtAt).toLocaleString()
                : t("about.unknown")}
            </dd>
            <dt>{t("about.distribution")}</dt>
            <dd data-testid="about-distribution">
              {info.data
                ? t(DISTRIBUTION[info.data.distribution] ?? "about.unknown")
                : "—"}
            </dd>
            <dt>{t("about.signing")}</dt>
            <dd>
              {info.data
                ? t(info.data.signed ? "about.signed" : "about.unsigned")
                : "—"}
            </dd>
            <dt>{t("about.build")}</dt>
            <dd>{info.data?.buildContext ?? "—"}</dd>
            <dt>{t("about.core")}</dt>
            <dd>{info.data?.coreVersion ?? "—"}</dd>
            <dt>{t("about.contract")}</dt>
            <dd>{info.data ? `v${info.data.contractVersion}` : "—"}</dd>
            <dt>{t("about.execution")}</dt>
            <dd>{info.data?.execution ?? "—"}</dd>
          </dl>
          {info.data?.sourceDirty && (
            <p className="notice warning" role="note">
              {t("about.dirty")}
            </p>
          )}
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

import { useInterface } from "../locale";
import { api, type LibraryDto, type ProjectListDto } from "@plc-lab/ipc";
import { ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import type { Navigate } from "../route";

const AREAS = [
  {
    page: "learn",
    title: "home.learn.title",
    text: "home.learn.text",
  },
  {
    page: "practice",
    title: "home.practice.title",
    text: "home.practice.text",
  },
  {
    page: "troubleshoot",
    title: "home.troubleshoot.title",
    text: "home.troubleshoot.text",
  },
  {
    page: "freelab",
    title: "home.freelab.title",
    text: "home.freelab.text",
  },
  {
    page: "knowledge",
    title: "home.knowledge.title",
    text: "home.knowledge.text",
  },
  {
    page: "progress",
    title: "home.progress.title",
    text: "home.progress.text",
  },
] as const;

export function HomePage({ navigate }: { navigate: Navigate }) {
  const { t } = useInterface();
  const data = useLoad(
    async () => ({
      library: await api.library(),
      projects: await api.projects(),
    }),
    [],
  );
  const library: LibraryDto | undefined = data.data?.library;
  const projects: ProjectListDto | undefined = data.data?.projects;
  const next = library?.lessons.find((l) => !l.completed) ?? null;
  const done = library?.lessons.filter((l) => l.completed).length ?? 0;
  const total = library?.lessons.length ?? 0;
  const recent = projects?.projects[0] ?? null;
  return (
    <div className="page-inner">
      <ErrorNotice error={data.error} />
      {library?.notices.map((n) => (
        <div key={n} className="notice warning" style={{ marginBottom: 12 }}>
          {n}
        </div>
      ))}
      <section className="hero">
        <div className="hero-main">
          <p className="eyebrow">{t("home.next")}</p>
          <h1>{t("home.title")}</h1>
          <p>{t("home.intro")}</p>
          <div className="row" style={{ marginTop: 16 }}>
            {next ? (
              <button
                className="btn btn-primary btn-lg"
                onClick={() => navigate({ page: "learn", lesson: next.id })}
              >
                {done === 0
                  ? t("home.first")
                  : t("home.continue", { number: next.number })}
              </button>
            ) : (
              <button
                className="btn btn-primary btn-lg"
                onClick={() => navigate({ page: "practice" })}
              >
                {t("home.practice.title")}
              </button>
            )}
            <button
              className="btn btn-lg"
              onClick={() => navigate({ page: "freelab" })}
            >
              {t("home.openLab")}
            </button>
          </div>
        </div>
        <div className="panel">
          <div className="panel-head">
            <h3>{t("common.continue")}</h3>
          </div>
          <div className="panel-body col">
            {next && (
              <button
                className="list-row"
                style={{ border: "1px solid var(--line)", borderRadius: 4 }}
                onClick={() => navigate({ page: "learn", lesson: next.id })}
              >
                <span className="lesson-number">{next.number}</span>
                <span className="grow">
                  <strong>{next.title}</strong>
                  <br />
                  <span className="muted">
                    {t("home.minutes", { minutes: next.minutes })}
                  </span>
                </span>
              </button>
            )}
            {recent ? (
              <button
                className="list-row"
                style={{ border: "1px solid var(--line)", borderRadius: 4 }}
                onClick={() =>
                  navigate({
                    page: "workspace",
                    projectId: recent.id,
                    section: recent.kind === "freeLab" ? "freelab" : "practice",
                  })
                }
              >
                <span className="grow">
                  <strong>{recent.name}</strong>
                  <br />
                  <span className="muted">
                    {t("home.recent")}{" "}
                    {recent.updatedAt.replace("T", " ").replace("Z", " UTC")}
                  </span>
                </span>
              </button>
            ) : (
              <p className="muted">{t("home.empty")}</p>
            )}
            {total > 0 && (
              <div>
                <div
                  className="row muted"
                  style={{ fontSize: 12, marginBottom: 4 }}
                >
                  {t("home.curriculum")} <span className="spacer" /> {done} /{" "}
                  {total} {t("home.lessons")}
                </div>
                <div className="progress-bar">
                  <div style={{ width: `${(100 * done) / total}%` }} />
                </div>
              </div>
            )}
          </div>
        </div>
      </section>
      <div className="tiles">
        {AREAS.map((a) => (
          <button
            key={a.page}
            className="tile"
            onClick={() =>
              navigate(
                a.page === "learn"
                  ? { page: "learn", lesson: null }
                  : a.page === "knowledge"
                    ? { page: "knowledge", article: null, query: "" }
                    : { page: a.page },
              )
            }
          >
            <h3>{t(a.title)}</h3>
            <p>{t(a.text)}</p>
          </button>
        ))}
      </div>
    </div>
  );
}

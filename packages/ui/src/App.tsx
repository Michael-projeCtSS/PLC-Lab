import { useCallback, useEffect, useState } from "react";
import { api, onOpenRequest } from "@plc-lab/ipc";
import { HomePage } from "./pages/HomePage";
import { LearnPage } from "./pages/LearnPage";
import { ExercisesPage } from "./pages/ExercisesPage";
import { FreeLabPage } from "./pages/FreeLabPage";
import { KnowledgePage } from "./pages/KnowledgePage";
import { ProgressPage } from "./pages/ProgressPage";
import { AboutPage } from "./pages/AboutPage";
import { WorkspacePage } from "./workspace/WorkspacePage";
import type { Navigate, Route } from "./route";
import { PreferencesProvider, useInterface } from "./locale";
import { Settings } from "./pages/Settings";

const NAV = [
  "home",
  "learn",
  "practice",
  "commission",
  "troubleshoot",
  "freelab",
  "knowledge",
  "progress",
] as const;

function initial(page: Route["page"]): Route {
  switch (page) {
    case "learn":
      return { page, lesson: null };
    case "knowledge":
      return { page, article: null, query: "" };
    case "workspace":
      return { page: "home" };
    default:
      return { page } as Route;
  }
}

export function App() {
  return (
    <PreferencesProvider>
      <Application />
    </PreferencesProvider>
  );
}

function Application() {
  const { t, preferences } = useInterface();
  const [settings, setSettings] = useState(false);
  const [route, setRoute] = useState<Route>({ page: "home" });
  const navigate: Navigate = useCallback((next) => setRoute(next), []);
  const [openProblem, setOpenProblem] = useState<string | null>(null);
  // A project requested by a launch (root launcher, second instance) opens here; the host
  // resolved and validated the file, the view only takes the result.
  useEffect(() => {
    let active = true;
    const take = () =>
      api.takeOpenRequest().then(
        (r) => {
          if (!active || !r) return;
          if (r.projectId) {
            setOpenProblem(null);
            setRoute({
              page: "workspace",
              projectId: r.projectId,
              section: "freelab",
            });
          } else if (r.problem) setOpenProblem(r.problem);
        },
        (e: unknown) =>
          active && setOpenProblem(e instanceof Error ? e.message : String(e)),
      );
    void take();
    let unlisten: (() => void) | null = null;
    onOpenRequest(() => void take()).then(
      (u) => (active ? (unlisten = u) : u()),
      () => undefined,
    );
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);
  const active = route.page === "workspace" ? route.section : route.page;
  return (
    <div
      className="app"
      data-density={preferences.density}
      data-explanations={preferences.explanations}
    >
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            PL
          </span>
          PLC Lab
        </div>
        <nav className="nav" aria-label={t("nav.main")}>
          {NAV.map((n) => (
            <button
              key={n}
              aria-current={active === n ? "page" : undefined}
              onClick={() => navigate(initial(n))}
            >
              {t(`nav.${n}`)}
            </button>
          ))}
        </nav>
        <div className="topbar-right">
          {!api.available() && (
            <span className="badge warning">{t("common.preview")}</span>
          )}
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => navigate({ page: "about" })}
          >
            {t("common.about")}
          </button>
          <button
            className="btn btn-ghost settings-button"
            aria-label={t("settings.title")}
            onClick={() => setSettings(true)}
          >
            <span aria-hidden="true">⚙</span>
            <span>{t("settings.title")}</span>
          </button>
        </div>
      </header>
      <main className="page" id="main">
        {openProblem && (
          <div className="notice error" role="alert" style={{ margin: 16 }}>
            {t("common.openProblem")} {openProblem}{" "}
            <button
              className="btn btn-ghost btn-sm"
              onClick={() => setOpenProblem(null)}
            >
              {t("common.dismiss")}
            </button>
          </div>
        )}
        {route.page === "home" && <HomePage navigate={navigate} />}
        {route.page === "learn" && (
          <LearnPage lessonId={route.lesson} navigate={navigate} />
        )}
        {route.page === "practice" && (
          <ExercisesPage kind="practice" navigate={navigate} />
        )}
        {route.page === "commission" && (
          <ExercisesPage kind="commissioning" navigate={navigate} />
        )}
        {route.page === "troubleshoot" && (
          <ExercisesPage kind="troubleshooting" navigate={navigate} />
        )}
        {route.page === "freelab" && <FreeLabPage navigate={navigate} />}
        {route.page === "knowledge" && (
          <KnowledgePage
            articleId={route.article}
            initialQuery={route.query}
            navigate={navigate}
          />
        )}
        {route.page === "progress" && <ProgressPage navigate={navigate} />}
        {route.page === "about" && <AboutPage />}
        {route.page === "workspace" && (
          <WorkspacePage
            key={route.projectId}
            projectId={route.projectId}
            navigate={navigate}
            back={route.section}
          />
        )}
      </main>
      {settings && <Settings onClose={() => setSettings(false)} />}
    </div>
  );
}

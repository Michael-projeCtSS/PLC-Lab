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

const NAV: { page: Route["page"]; label: string }[] = [
  { page: "home", label: "HOME" },
  { page: "learn", label: "LEARN" },
  { page: "practice", label: "PRACTICE" },
  { page: "troubleshoot", label: "TROUBLESHOOT" },
  { page: "freelab", label: "FREE LAB" },
  { page: "knowledge", label: "KNOWLEDGE" },
  { page: "progress", label: "PROGRESS" },
];

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
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            PL
          </span>
          PLC Lab
        </div>
        <nav className="nav" aria-label="Main">
          {NAV.map((n) => (
            <button
              key={n.page}
              aria-current={active === n.page ? "page" : undefined}
              onClick={() => navigate(initial(n.page))}
            >
              {n.label}
            </button>
          ))}
        </nav>
        <div className="topbar-right">
          {!api.available() && (
            <span className="badge warning">Preview without desktop core</span>
          )}
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => navigate({ page: "about" })}
          >
            About
          </button>
        </div>
      </header>
      <main className="page" id="main">
        {openProblem && (
          <div className="notice error" role="alert" style={{ margin: 16 }}>
            The project could not be opened: {openProblem}{" "}
            <button
              className="btn btn-ghost btn-sm"
              onClick={() => setOpenProblem(null)}
            >
              Dismiss
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
    </div>
  );
}

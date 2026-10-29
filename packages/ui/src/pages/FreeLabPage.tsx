import { useInterface } from "../locale";
import { useState } from "react";
import { api, type ProjectSummaryDto } from "@plc-lab/ipc";
import { Badge, Empty, ErrorNotice, Modal } from "../ui";
import { useLoad } from "../useLoad";
import type { Navigate } from "../route";

export function FreeLabPage({ navigate }: { navigate: Navigate }) {
  const { t } = useInterface();
  const data = useLoad(
    async () => ({
      library: await api.library(),
      projects: await api.projects(),
    }),
    [],
  );
  const [error, setError] = useState<unknown>(null);
  const [creating, setCreating] = useState(false);
  const [renaming, setRenaming] = useState<ProjectSummaryDto | null>(null);
  const library = data.data?.library;
  const projects = data.data?.projects;
  const sceneTitle = (id: string) =>
    library?.scenes.find((s) => s.id === id)?.title ?? id;
  const platformName = (id: string) =>
    library?.platforms.find((p) => p.id === id)?.displayName ?? id;
  const run = (work: Promise<unknown>) =>
    work.then(() => data.reload(), setError);
  const importFile = async (file: File) => {
    if (file.size > 4 * 1024 * 1024) throw new Error(t("lab.fileLarge"));
    const project = await api.importProject(await file.text());
    navigate({ page: "workspace", projectId: project.id, section: "freelab" });
  };
  const exportProject = async (p: ProjectSummaryDto) => {
    const file = await api.exportProject(p.id);
    const url = URL.createObjectURL(
      new Blob([file.text], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = file.fileName;
    a.click();
    URL.revokeObjectURL(url);
  };
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>{t("lab.title")}</h1>
          <p>{t("lab.intro")}</p>
        </div>
        <span className="spacer" />
        <label className="btn">
          {t("lab.import")}
          <input
            type="file"
            accept=".plclab,application/json"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (file) importFile(file).catch(setError);
            }}
          />
        </label>
        <button
          className="btn btn-primary"
          onClick={() => setCreating(true)}
          disabled={!library}
        >
          {t("lab.new")}
        </button>
      </div>
      <ErrorNotice
        error={data.error ?? error}
        onDismiss={() => setError(null)}
      />
      {projects && projects.problems.length > 0 && (
        <div className="notice warning" style={{ marginBottom: 12 }}>
          {t("lab.readProblem")} {projects.problems.join("; ")}
        </div>
      )}
      {projects && projects.projects.length === 0 && (
        <Empty>
          <h3>{t("lab.emptyTitle")}</h3>
          <p>{t("lab.empty")}</p>
          <button
            className="btn btn-primary"
            disabled={!library}
            onClick={() => setCreating(true)}
          >
            {t("lab.new")}
          </button>
        </Empty>
      )}
      {projects && projects.projects.length > 0 && (
        <div className="list">
          {projects.projects.map((p) => (
            <div key={p.id} className="list-row" style={{ cursor: "default" }}>
              <div className="grow">
                <strong>{p.name}</strong>{" "}
                {p.kind === "exercise" ? (
                  <Badge>{t("lab.exercise")}</Badge>
                ) : (
                  <Badge kind="auto">{t("lab.title")}</Badge>
                )}
                <div className="muted" style={{ fontSize: 12 }}>
                  {sceneTitle(p.scene)} · {platformName(p.platform)} ·{" "}
                  {t("lab.saved")}{" "}
                  {p.updatedAt.replace("T", " ").replace("Z", " UTC")}
                </div>
              </div>
              <button
                className="btn btn-primary btn-sm"
                onClick={() =>
                  navigate({
                    page: "workspace",
                    projectId: p.id,
                    section: "freelab",
                  })
                }
              >
                {t("common.open")}
              </button>
              <button className="btn btn-sm" onClick={() => setRenaming(p)}>
                {t("lab.saveAs")}
              </button>
              <button
                className="btn btn-sm"
                onClick={() => exportProject(p).catch(setError)}
              >
                {t("common.export")}
              </button>
              <button
                className="btn btn-sm btn-danger"
                onClick={() => {
                  if (window.confirm(t("lab.deleteConfirm", { name: p.name })))
                    run(api.deleteProject(p.id));
                }}
              >
                {t("common.delete")}
              </button>
            </div>
          ))}
        </div>
      )}
      {projects && (
        <p className="dim" style={{ marginTop: 10, fontSize: 12 }}>
          {t("lab.folder")} {projects.directory}
        </p>
      )}
      {creating && library && (
        <NewProject
          scenes={library.scenes}
          platforms={library.platforms}
          onClose={() => setCreating(false)}
          onCreate={(name, scene, platform) =>
            api.createProject({ name, scene, platform }).then(
              (p) =>
                navigate({
                  page: "workspace",
                  projectId: p.id,
                  section: "freelab",
                }),
              setError,
            )
          }
        />
      )}
      {renaming && (
        <Modal title={t("lab.saveTitle")} onClose={() => setRenaming(null)}>
          <SaveAs
            initial={`${renaming.name} (copy)`}
            onSave={(name) => {
              const source = renaming;
              setRenaming(null);
              run(api.saveProjectAs(source.id, name));
            }}
          />
        </Modal>
      )}
    </div>
  );
}

function SaveAs({
  initial,
  onSave,
}: {
  initial: string;
  onSave: (name: string) => void;
}) {
  const { t } = useInterface();
  const [name, setName] = useState(initial);
  return (
    <>
      <label className="field">
        {t("lab.newName")}
        <input
          className="input"
          value={name}
          maxLength={120}
          onChange={(e) => setName(e.target.value)}
          autoFocus
        />
      </label>
      <div className="row">
        <span className="spacer" />
        <button
          className="btn btn-primary"
          disabled={!name.trim()}
          onClick={() => onSave(name.trim())}
        >
          {t("lab.saveCopy")}
        </button>
      </div>
    </>
  );
}

function NewProject({
  scenes,
  platforms,
  onClose,
  onCreate,
}: {
  scenes: { id: string; title: string; summary: string }[];
  platforms: {
    id: string;
    displayName: string;
    tier: string;
    fidelity: { level: string };
  }[];
  onClose: () => void;
  onCreate: (name: string, scene: string, platform: string) => void;
}) {
  const { t } = useInterface();
  const [name, setName] = useState("");
  const [scene, setScene] = useState(scenes[0]?.id ?? "");
  const [platform, setPlatform] = useState("generic.iec");
  const selected = scenes.find((s) => s.id === scene);
  const profile = platforms.find((p) => p.id === platform);
  return (
    <Modal title={t("lab.newTitle")} onClose={onClose}>
      <label className="field">
        {t("lab.name")}
        <input
          className="input"
          value={name}
          maxLength={120}
          placeholder={selected?.title ?? ""}
          onChange={(e) => setName(e.target.value)}
          autoFocus
        />
      </label>
      <label className="field">
        {t("lab.scene")}
        <select
          className="input"
          value={scene}
          onChange={(e) => setScene(e.target.value)}
        >
          {scenes.map((s) => (
            <option key={s.id} value={s.id}>
              {s.title}
            </option>
          ))}
        </select>
      </label>
      {selected && (
        <p className="muted" style={{ fontSize: 12 }}>
          {selected.summary}
        </p>
      )}
      <label className="field">
        {t("lab.profile")}
        <select
          className="input"
          value={platform}
          onChange={(e) => setPlatform(e.target.value)}
        >
          {platforms.map((p) => (
            <option key={p.id} value={p.id}>
              {p.displayName}
            </option>
          ))}
        </select>
      </label>
      {profile && (
        <p className="muted" style={{ fontSize: 12 }}>
          {t("lab.fidelity")}{" "}
          <strong>
            {profile.fidelity.level.replaceAll("_", " ").toLowerCase()}
          </strong>
          . {t("lab.profileHelp")}
        </p>
      )}
      <div className="row">
        <span className="spacer" />
        <button className="btn" onClick={onClose}>
          {t("common.cancel")}
        </button>
        <button
          className="btn btn-primary"
          disabled={!scene}
          onClick={() => onCreate(name.trim(), scene, platform)}
        >
          {t("lab.create")}
        </button>
      </div>
    </Modal>
  );
}

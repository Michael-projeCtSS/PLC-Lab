import { useState } from "react";
import { useInterface } from "../locale";
import { Tabs } from "../ui";
import { StructuredTextEditor } from "../StructuredTextEditor";

export function PreviewBanner() {
  const { t } = useInterface();
  return (
    <aside className="preview-banner" aria-label={t("preview.title")}>
      <strong>{t("preview.title")}</strong>
      <span>{t("preview.description")}</span>
    </aside>
  );
}

export function NativeRequired({ title }: { title?: string }) {
  const { t } = useInterface();
  return (
    <section className="notice warning" role="note">
      <h3>{title ?? t("preview.nativeTitle")}</h3>
      <p>{t("preview.nativeRequired")}</p>
      <p className="muted">{t("preview.isolation")}</p>
    </section>
  );
}

export function PreviewUnavailable({ title }: { title: string }) {
  return (
    <div className="page-inner">
      <h1>{title}</h1>
      <NativeRequired />
    </div>
  );
}

/** Real editor/navigation widgets with unsaved text only; no runtime state or fake plant. */
export function PreviewWorkspace() {
  const { t } = useInterface();
  const [mode, setMode] = useState<"basic" | "advanced">("basic");
  const [tab, setTab] = useState("program");
  const [source, setSource] = useState(
    "PROGRAM Main\nVAR\nEND_VAR\n\nEND_PROGRAM\n",
  );
  const tabs = [
    { id: "program", label: t("ws.program") },
    { id: "process", label: t("ws.process") },
    ...(mode === "advanced"
      ? [
          { id: "hardware", label: t("ws.hardware") },
          { id: "network", label: t("ws.network") },
          { id: "diagnostics", label: t("ws.diagnostics") },
          { id: "commissioning", label: t("ws.commissioning") },
        ]
      : []),
  ];
  return (
    <div className="page-inner preview-workspace">
      <div className="page-header">
        <div>
          <h1>{t("lab.title")}</h1>
          <p>{t("preview.editor")}</p>
        </div>
        <div className="segmented" role="group" aria-label={t("ws.mode")}>
          {(["basic", "advanced"] as const).map((value) => (
            <button
              key={value}
              aria-pressed={mode === value}
              onClick={() => {
                setMode(value);
                setTab("program");
              }}
            >
              {t(`ws.${value}`)}
            </button>
          ))}
        </div>
      </div>
      <NativeRequired />
      <Tabs tabs={tabs} active={tab} onSelect={setTab} label={t("ws.views")} />
      {tab === "program" ? (
        <div className="preview-editor">
          <StructuredTextEditor
            source={source}
            onChange={setSource}
            diagnostics={[]}
            focusedDiagnostic={null}
          />
        </div>
      ) : (
        <NativeRequired
          title={
            tabs.find((item) => item.id === tab)?.label ??
            t("preview.nativeTitle")
          }
        />
      )}
      <div className="row">
        <button className="btn" disabled title={t("preview.nativeRequired")}>
          {t("control.build")}
        </button>
        <span>{t("preview.unsaved")}</span>
      </div>
    </div>
  );
}

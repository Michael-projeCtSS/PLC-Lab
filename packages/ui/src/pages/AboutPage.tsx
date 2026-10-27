import { api } from "@plc-lab/ipc";
import { Badge, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";

/** Product information and developer diagnostics (versions, contract, knowledge packs). */
export function AboutPage() {
  const info = useLoad(() => api.coreInfo(), []);
  const status = useLoad(() => api.knowledgeStatus(), []);
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>About PLC Lab</h1>
          <p>
            An educational simulator for PLC programming, I/O hardware, process
            behavior, industrial communication and fault finding. PLC Lab is not
            affiliated with any PLC vendor and does not emulate vendor firmware.
          </p>
        </div>
      </div>
      <ErrorNotice error={info.error ?? status.error} />
      <div className="tiles">
        <div className="panel">
          <div className="panel-head">
            <h4>Application</h4>
          </div>
          <dl className="kv panel-body">
            <dt>Version</dt>
            <dd>{info.data?.appVersion ?? "—"}</dd>
            <dt>Build</dt>
            <dd>{info.data?.buildContext ?? "—"}</dd>
            <dt>Simulation core</dt>
            <dd>{info.data?.coreVersion ?? "—"}</dd>
            <dt>Desktop contract</dt>
            <dd>{info.data ? `v${info.data.contractVersion}` : "—"}</dd>
            <dt>Execution</dt>
            <dd>{info.data?.execution ?? "—"}</dd>
          </dl>
        </div>
        <div className="panel">
          <div className="panel-head">
            <h4>Knowledge packs</h4>
            <span className="spacer" />
            {status.data && (
              <span className="muted">
                {status.data.indexedItems} indexed items
              </span>
            )}
          </div>
          <div className="panel-body col">
            {status.data?.packs.length === 0 && (
              <p className="muted">No knowledge pack was found.</p>
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
                    {p.status}
                  </Badge>
                </div>
                <div className="muted">
                  {p.id} {p.version} · {p.articles} articles · {p.lessons}{" "}
                  lessons · {p.exercises} exercises · {p.scenes} scenes ·{" "}
                  {p.sources} sources · {p.profiles} platform profiles
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
                Search locations:
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

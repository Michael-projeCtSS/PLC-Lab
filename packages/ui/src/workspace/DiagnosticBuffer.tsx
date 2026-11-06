/**
 * The CPU diagnostic buffer, newest first: time, severity, category, source, message,
 * structured details and whether a condition is still active or when it cleared. The
 * backend classifies, pairs and filters the entries; this view only chooses filters and
 * pages, and words each entry from its code and details.
 */
import { useState } from "react";
import { api, type DiagnosticBufferDto } from "@plc-lab/ipc";
import { seconds } from "../format";
import { useInterface } from "../locale";
import {
  diagnosticDetails,
  diagnosticSource,
  diagnosticText,
} from "../locale/messages";
import { Badge, ErrorNotice } from "../ui";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

const PAGE = 50;
const SEVERITY_KIND: Record<string, string> = {
  error: "fault",
  warning: "warning",
  info: "normal",
};
const SEVERITIES = ["error", "warning", "info"] as const;
const CATEGORIES = ["CPU", "HARDWARE", "NETWORK"] as const;

export function DiagnosticBuffer({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const ws = lab.ws;
  const [severity, setSeverity] = useState<string | null>(null);
  const [category, setCategory] = useState<string | null>(null);
  const [moduleId, setModuleId] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const buffer = useFrameQuery<DiagnosticBufferDto>(
    lab,
    () =>
      ws
        ? api.diagnosticBuffer(ws, {
            offset,
            limit: PAGE,
            severity,
            category,
            moduleId,
          })
        : Promise.reject(new Error("No workspace")),
    [offset, severity, category, moduleId],
  );
  const modules = lab.project?.station.modules ?? [];
  const select = (set: (v: string | null) => void) => (value: string) => {
    set(value === "" ? null : value);
    setOffset(0);
  };
  const data = buffer.data;
  return (
    <div className="panel">
      <div className="panel-head row wrap" style={{ gap: 8 }}>
        <h4>{t("diag.title")}</h4>
        <span className="muted">
          {t("diag.newest")}
          {data
            ? ` · ${t("diag.counts", {
                matching: data.matching,
                shown: Math.min(Number(data.recorded), data.capacity),
                capacity: data.capacity,
                recorded: data.recorded,
              })}`
            : ""}
        </span>
        <span className="spacer" />
        <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
          {t("diag.severity")}
          <select
            className="input"
            value={severity ?? ""}
            onChange={(e) => select(setSeverity)(e.target.value)}
          >
            <option value="">{t("diag.all")}</option>
            {SEVERITIES.map((s) => (
              <option key={s} value={s}>
                {t(`diag.severity.${s}`)}
              </option>
            ))}
          </select>
        </label>
        <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
          {t("diag.category")}
          <select
            className="input"
            value={category ?? ""}
            onChange={(e) => select(setCategory)(e.target.value)}
          >
            <option value="">{t("diag.all")}</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {t(`diag.category.${c}`)}
              </option>
            ))}
          </select>
        </label>
        <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
          {t("diag.module")}
          <select
            className="input"
            value={moduleId ?? ""}
            onChange={(e) => select(setModuleId)(e.target.value)}
          >
            <option value="">{t("diag.all")}</option>
            {modules.map((m) => (
              <option key={m.id} value={m.id}>
                {m.id}
              </option>
            ))}
          </select>
        </label>
      </div>
      <ErrorNotice error={buffer.error} />
      <table className="table" aria-label={t("diag.table")}>
        <thead>
          <tr>
            <th>{t("diag.time")}</th>
            <th>{t("diag.severity")}</th>
            <th>{t("diag.category")}</th>
            <th>{t("diag.source")}</th>
            <th>{t("diag.message")}</th>
            <th>{t("diag.state")}</th>
            <th>{t("diag.details")}</th>
          </tr>
        </thead>
        <tbody>
          {data?.entries.length === 0 && (
            <tr>
              <td colSpan={7} className="muted">
                {t("diag.none")}
              </td>
            </tr>
          )}
          {data?.entries.map((d, i) => (
            <tr key={`${d.timeUs}-${offset + i}`}>
              <td className="mono">{seconds(d.timeUs)}</td>
              <td>
                <Badge kind={SEVERITY_KIND[d.severity] ?? "normal"}>
                  {t(`diag.severity.${d.severity}`)}
                </Badge>
              </td>
              <td>{t(`diag.category.${d.category}`)}</td>
              <td>{diagnosticSource(t, d)}</td>
              <td
                style={{ whiteSpace: "normal" }}
                title={t("diag.original", { text: d.text })}
              >
                {diagnosticText(t, d)}
              </td>
              <td>
                {d.event === "EVENT"
                  ? t("diag.event")
                  : d.event === "OUTGOING"
                    ? t("diag.cleared")
                    : d.active
                      ? t("diag.active")
                      : d.clearedUs
                        ? t("diag.clearedAfter", {
                            time: seconds(
                              String(BigInt(d.clearedUs) - BigInt(d.timeUs)),
                            ),
                          })
                        : t("diag.gone")}
              </td>
              <td className="dim">{diagnosticDetails(t, d)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row" style={{ padding: 8, gap: 8 }}>
        <button
          className="btn btn-sm"
          disabled={offset === 0}
          onClick={() => setOffset(Math.max(0, offset - PAGE))}
        >
          {t("diag.newer")}
        </button>
        <button
          className="btn btn-sm"
          disabled={!data || offset + PAGE >= data.matching}
          onClick={() => setOffset(offset + PAGE)}
        >
          {t("diag.older")}
        </button>
      </div>
    </div>
  );
}

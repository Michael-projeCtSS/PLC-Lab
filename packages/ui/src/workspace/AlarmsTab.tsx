/**
 * Operator alarm view (ADR-0037). Basic mode lists the alarms that need attention with
 * acknowledgment; advanced mode adds filters, selection, details and the bounded history.
 * Every state, filter result and history entry comes from the backend alarm engine, and
 * acknowledging is a backend operation that never changes the condition behind an alarm.
 */
import { api, type AlarmDto, type AlarmsDto } from "@plc-lab/ipc";
import { useState } from "react";
import { seconds } from "../format";
import { useInterface } from "../locale";
import { Badge } from "../ui";
import {
  CATEGORY,
  PRIORITIES,
  STATE,
  STATES,
  TRANSITION,
  alarmCondition,
  alarmPriority,
  needsAcknowledgment,
  priorityBadge,
  plainLabel,
  sequenceAlarmLabel,
  type AlarmLabel,
} from "./alarmText";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";
import { useSequenceDefinition } from "./sequenceDefinition";

const PAGE = 50;

/** Compact alarm list for other views: source, condition and state. */
export function AlarmSummary({
  alarms,
  label = plainLabel,
}: {
  alarms: AlarmDto[];
  label?: AlarmLabel;
}) {
  const { t } = useInterface();
  return (
    <>
      {alarms.map((a) => (
        <span key={a.id} className={`badge ${priorityBadge(a.priority)}`}>
          {label(a)}: {alarmCondition(t, a.kind, a.text)} ({t(STATE[a.state])})
        </span>
      ))}
    </>
  );
}

/** Basic mode: a visible indicator of the most urgent alarm and what to investigate. */
export function AlarmIndicator({
  alarms,
  open,
  label = plainLabel,
}: {
  alarms: AlarmDto[];
  open: () => void;
  label?: AlarmLabel;
}) {
  const { t } = useInterface();
  const top = alarms[0];
  if (!top) return null;
  const urgent = priorityBadge(top.priority) === "fault";
  return (
    <div
      className={`notice ${urgent ? "error" : "warning"} row`}
      role="status"
      aria-label={t("alarm.indicatorLabel")}
    >
      <div className="grow">
        <strong>{t("alarm.indicator", { count: alarms.length })}</strong>{" "}
        {t("alarm.indicatorExplain", {
          alarm: `${label(top)}: ${alarmCondition(t, top.kind, top.text)} (${alarmPriority(t, top.priority)})`,
        })}
      </div>
      <button className="btn btn-sm" onClick={open}>
        {t("alarm.open")}
      </button>
    </div>
  );
}

function AlarmDetail({
  alarm,
  inspect,
  label,
}: {
  alarm: AlarmDto;
  inspect: (a: AlarmDto) => void;
  label: AlarmLabel;
}) {
  const { t } = useInterface();
  return (
    <div
      className="panel"
      role="region"
      aria-label={t("alarm.detailsTitle")}
      style={{ marginTop: 8 }}
    >
      <div className="panel-head row">
        <h4 className="grow">
          {label(alarm)}: {alarmCondition(t, alarm.kind, alarm.text)}
        </h4>
        <button className="btn btn-sm btn-ghost" onClick={() => inspect(alarm)}>
          {t("alarm.inspect")}
        </button>
      </div>
      <dl className="kv panel-body">
        <dt>{t("alarm.detail.id")}</dt>
        <dd className="mono">{alarm.id}</dd>
        <dt>{t("alarm.detail.code")}</dt>
        <dd className="mono">{alarm.kind}</dd>
        <dt>{t("alarm.detail.category")}</dt>
        <dd>{t(CATEGORY[alarm.category])}</dd>
        <dt>{t("alarm.detail.source")}</dt>
        <dd className="mono">{alarm.component}</dd>
        <dt>{t("alarm.col.state")}</dt>
        <dd>{t(STATE[alarm.state])}</dd>
        <dt>{t("alarm.detail.activated")}</dt>
        <dd className="mono">
          {alarm.activatedUs ? seconds(alarm.activatedUs) : "—"}
        </dd>
        <dt>{t("alarm.detail.changed")}</dt>
        <dd className="mono">
          {alarm.changedUs ? seconds(alarm.changedUs) : "—"}
        </dd>
        <dt>{t("alarm.detail.acknowledgments")}</dt>
        <dd>{alarm.acknowledgments}</dd>
      </dl>
    </div>
  );
}

export function AlarmsTab({
  lab,
  advanced,
  inspect,
}: {
  lab: Lab;
  advanced: boolean;
  inspect: (alarm: AlarmDto) => void;
}) {
  const { t } = useInterface();
  const label = sequenceAlarmLabel(useSequenceDefinition(lab));
  const ws = lab.ws;
  useEvidence(lab, "alarm view");
  const [offset, setOffset] = useState(0);
  const [priority, setPriority] = useState("");
  const [state, setState] = useState("");
  const [source, setSource] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [detail, setDetail] = useState<string | null>(null);
  const filtered = useFrameQuery<AlarmsDto | null>(
    lab,
    () =>
      ws && advanced
        ? api.alarms(ws, {
            offset,
            limit: PAGE,
            priority: priority || null,
            state: state || null,
            source: source.trim() || null,
          })
        : Promise.resolve(null),
    [offset, advanced, priority, state, source],
  );
  const all = lab.frame?.alarms ?? [];
  // Basic mode shows the frame's list; advanced mode the backend's filtered list.
  const alarms = advanced ? (filtered.data?.alarms ?? []) : all;
  const filtering = advanced && (priority || state || source.trim());
  const eligible = new Set(alarms.filter(needsAcknowledgment).map((a) => a.id));
  const chosen = selected.filter((id) => eligible.has(id));
  const waiting = all.filter(needsAcknowledgment).length;
  const shown = all.find((a) => a.id === detail) ?? null;
  const acknowledge = (ids: string[] | null) => {
    if (!ws) return;
    void lab
      .act(
        "alarm",
        () => api.acknowledgeAlarms(ws, ids),
        (r) => r.frame,
      )
      .then(() => setSelected([]));
  };
  const refilter = (apply: () => void) => {
    apply();
    setOffset(0);
  };
  const history = filtered.data;
  return (
    <div
      style={{ position: "absolute", inset: 0, overflow: "auto", padding: 16 }}
      className="col"
    >
      <div className="panel">
        <div className="panel-head row wrap">
          <h4 className="grow">{t("alarm.active")}</h4>
          {advanced && (
            <button
              className="btn btn-sm"
              disabled={lab.busy !== null || chosen.length === 0}
              title={t("alarm.acknowledgeHint")}
              onClick={() => acknowledge(chosen)}
            >
              {t("alarm.acknowledgeSelected", { count: chosen.length })}
            </button>
          )}
          {waiting > 0 && (
            <button
              className="btn btn-sm btn-primary"
              disabled={lab.busy !== null}
              title={t("alarm.acknowledgeHint")}
              onClick={() => acknowledge(null)}
            >
              {t("alarm.acknowledgeAll")}
            </button>
          )}
        </div>
        <div className="panel-body col">
          <p className="muted">{t("alarm.explain")}</p>
          {advanced && (
            <div
              className="row wrap"
              role="group"
              aria-label={t("alarm.filters")}
            >
              <select
                className="input"
                aria-label={t("alarm.filter.priority")}
                value={priority}
                onChange={(e) => refilter(() => setPriority(e.target.value))}
              >
                <option value="">{t("alarm.filter.allPriorities")}</option>
                {PRIORITIES.map((p) => (
                  <option key={p} value={p}>
                    {alarmPriority(t, p)}
                  </option>
                ))}
              </select>
              <select
                className="input"
                aria-label={t("alarm.filter.state")}
                value={state}
                onChange={(e) => refilter(() => setState(e.target.value))}
              >
                <option value="">{t("alarm.filter.allStates")}</option>
                {STATES.map((s) => (
                  <option key={s} value={s}>
                    {t(STATE[s])}
                  </option>
                ))}
              </select>
              <input
                className="input"
                aria-label={t("alarm.filter.source")}
                placeholder={t("alarm.filter.source")}
                value={source}
                maxLength={128}
                onChange={(e) => refilter(() => setSource(e.target.value))}
              />
            </div>
          )}
          {alarms.length === 0 ? (
            <span className="muted">
              {filtering ? t("alarm.noMatch") : t("alarm.none")}
            </span>
          ) : (
            <table className="table" aria-label={t("alarm.active")}>
              <thead>
                <tr>
                  {advanced && <th />}
                  <th>{t("alarm.col.time")}</th>
                  <th>{t("alarm.col.priority")}</th>
                  <th>{t("alarm.col.source")}</th>
                  <th>{t("alarm.col.message")}</th>
                  <th>{t("alarm.col.condition")}</th>
                  <th>{t("alarm.col.acknowledgment")}</th>
                  <th>{t("alarm.col.action")}</th>
                </tr>
              </thead>
              <tbody>
                {alarms.map((a) => {
                  const message = alarmCondition(t, a.kind, a.text);
                  return (
                    <tr key={a.id} title={t(STATE[a.state])}>
                      {advanced && (
                        <td>
                          <input
                            type="checkbox"
                            aria-label={t("alarm.select", {
                              alarm: `${label(a)}: ${message}`,
                            })}
                            disabled={!eligible.has(a.id)}
                            checked={chosen.includes(a.id)}
                            onChange={(e) =>
                              setSelected(
                                e.target.checked
                                  ? [...chosen, a.id]
                                  : chosen.filter((id) => id !== a.id),
                              )
                            }
                          />
                        </td>
                      )}
                      <td className="mono">
                        {a.activatedUs ? seconds(a.activatedUs) : "—"}
                      </td>
                      <td>
                        <Badge kind={priorityBadge(a.priority)}>
                          {alarmPriority(t, a.priority)}
                        </Badge>
                      </td>
                      <td>{label(a)}</td>
                      <td>{message}</td>
                      <td>
                        {a.condition
                          ? t("alarm.condition.present")
                          : t("alarm.condition.cleared")}
                      </td>
                      <td>
                        {needsAcknowledgment(a)
                          ? t("alarm.ack.no")
                          : t("alarm.ack.yes")}
                      </td>
                      <td className="row">
                        {needsAcknowledgment(a) && (
                          <button
                            className="btn btn-sm"
                            disabled={lab.busy !== null}
                            title={t("alarm.acknowledgeHint")}
                            onClick={() => acknowledge([a.id])}
                          >
                            {t("alarm.acknowledge")}
                          </button>
                        )}
                        <button
                          className="btn btn-sm btn-ghost"
                          aria-expanded={detail === a.id}
                          onClick={() =>
                            setDetail(detail === a.id ? null : a.id)
                          }
                        >
                          {t("alarm.details")}
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
          {shown && (
            <AlarmDetail alarm={shown} inspect={inspect} label={label} />
          )}
        </div>
      </div>
      {advanced && history && (
        <div className="panel">
          <div className="panel-head row wrap">
            <h4 className="grow">{t("alarm.history")}</h4>
            <span className="muted">
              {t("alarm.kept", {
                matching: history.matching,
                kept: history.kept,
                recorded: history.recorded,
                capacity: history.capacity,
              })}
            </span>
          </div>
          <div className="panel-body col">
            <span className="muted">
              {t("alarm.configured", { count: history.configured })}
            </span>
            {history.history.length === 0 ? (
              <span className="muted">
                {filtering ? t("alarm.noMatch") : t("alarm.historyEmpty")}
              </span>
            ) : (
              <table className="table" aria-label={t("alarm.history")}>
                <thead>
                  <tr>
                    <th>{t("alarm.col.time")}</th>
                    <th>{t("alarm.col.source")}</th>
                    <th>{t("alarm.col.message")}</th>
                    <th>{t("alarm.col.transition")}</th>
                    <th>{t("alarm.col.priority")}</th>
                    <th>{t("alarm.col.state")}</th>
                  </tr>
                </thead>
                <tbody>
                  {history.history.map((e) => (
                    <tr key={e.seq}>
                      <td className="mono">{seconds(e.timeUs)}</td>
                      <td>
                        {label({ id: e.alarm, sourceLabel: e.sourceLabel })}
                      </td>
                      <td>{alarmCondition(t, e.code, e.message)}</td>
                      <td>{t(TRANSITION[e.transition])}</td>
                      <td>{alarmPriority(t, e.priority)}</td>
                      <td>{t(STATE[e.to])}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <div className="row">
              <button
                className="btn btn-sm"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - PAGE))}
              >
                {t("alarm.newer")}
              </button>
              <button
                className="btn btn-sm"
                disabled={offset + PAGE >= history.matching}
                onClick={() => setOffset(offset + PAGE)}
              >
                {t("alarm.older")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

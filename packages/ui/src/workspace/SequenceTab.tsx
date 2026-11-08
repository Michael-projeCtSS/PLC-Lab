/**
 * Sequence inspector (ADR-0043): the backend sequence monitor's view of the PLC program.
 * Everything shown comes from the frame; the view evaluates no condition itself.
 */
import type {
  AlarmDto,
  SequenceConditionDto,
  SequenceConditionRowDto,
  SequenceDefinitionDto,
  SequenceEventDto,
  SequenceStatusDto,
  SequenceValueDto,
} from "@plc-lab/ipc";
import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { Badge } from "../ui";
import {
  alarmCondition,
  priorityBadge,
  STATE as ALARM_STATE,
} from "./alarmText";

type Translate = ReturnType<typeof useInterface>["t"];
type State = SequenceStatusDto["state"];

const STATE: Record<State, TranslationKey> = {
  UNBOUND: "seq.state.UNBOUND",
  STOPPED: "seq.state.STOPPED",
  IDLE: "seq.state.IDLE",
  RUNNING: "seq.state.RUNNING",
  HELD: "seq.state.HELD",
  COMPLETED: "seq.state.COMPLETED",
  FAULTED: "seq.state.FAULTED",
};
const EXPLAIN: Record<State, TranslationKey> = {
  UNBOUND: "seq.explain.UNBOUND",
  STOPPED: "seq.explain.STOPPED",
  IDLE: "seq.explain.IDLE",
  RUNNING: "seq.explain.RUNNING",
  HELD: "seq.explain.HELD",
  COMPLETED: "seq.explain.COMPLETED",
  FAULTED: "seq.explain.FAULTED",
};
const BADGE: Record<State, string> = {
  UNBOUND: "normal",
  STOPPED: "stopped",
  IDLE: "normal",
  RUNNING: "running",
  HELD: "warning",
  COMPLETED: "active",
  FAULTED: "fault",
};
type StepState = SequenceStatusDto["steps"][number]["state"];
const STEP_STATE: Record<StepState, TranslationKey> = {
  INACTIVE: "seq.step.INACTIVE",
  ACTIVE: "seq.step.ACTIVE",
  HELD: "seq.step.HELD",
  COMPLETED: "seq.step.COMPLETED",
  FAULTED: "seq.step.FAULTED",
};
const EVENT: Record<SequenceEventDto["code"], TranslationKey> = {
  runStarted: "seq.event.runStarted",
  transitionCompleted: "seq.event.transitionCompleted",
  sequenceStarted: "seq.event.sequenceStarted",
  sequenceCompleted: "seq.event.sequenceCompleted",
  sequenceAborted: "seq.event.sequenceAborted",
  startBlocked: "seq.event.startBlocked",
  transitionBlocked: "seq.event.transitionBlocked",
  interlockActivated: "seq.event.interlockActivated",
  interlockCleared: "seq.event.interlockCleared",
  held: "seq.event.held",
  resumed: "seq.event.resumed",
  stepTimeout: "seq.event.stepTimeout",
  faultCleared: "seq.event.faultCleared",
  unknownStep: "seq.event.unknownStep",
  undefinedTransition: "seq.event.undefinedTransition",
  conditionNotMet: "seq.event.conditionNotMet",
  permissiveViolated: "seq.event.permissiveViolated",
  interlockViolated: "seq.event.interlockViolated",
  progressDuringHold: "seq.event.progressDuringHold",
  progressAfterFault: "seq.event.progressAfterFault",
};
const REASON: Record<string, TranslationKey> = {
  stepTimeout: "seq.reason.stepTimeout",
  unknownStep: "seq.reason.unknownStep",
  undefinedTransition: "seq.reason.undefinedTransition",
  conditionNotMet: "seq.reason.conditionNotMet",
  permissiveViolated: "seq.reason.permissiveViolated",
  interlockViolated: "seq.reason.interlockViolated",
  progressDuringHold: "seq.reason.progressDuringHold",
  progressAfterFault: "seq.reason.progressAfterFault",
};
const START: Record<NonNullable<SequenceEventDto["start"]>, TranslationKey> = {
  WARM: "seq.start.WARM",
  COLD: "seq.start.COLD",
  ATTACHED: "seq.start.ATTACHED",
};
const REACTION: Record<
  SequenceDefinitionDto["interlocks"][number]["reaction"],
  TranslationKey
> = {
  blockStart: "seq.reaction.blockStart",
  holdProgress: "seq.reaction.holdProgress",
  abortSequence: "seq.reaction.abortSequence",
};
const LAYER: Record<SequenceConditionRowDto["layer"], TranslationKey> = {
  inputs: "seq.layer.inputs",
  outputs: "seq.layer.outputs",
  process: "seq.layer.process",
  next: "seq.layer.next",
};
const OP: Record<SequenceConditionRowDto["op"], string> = {
  eq: "=",
  ne: "≠",
  gt: ">",
  ge: "≥",
  lt: "<",
  le: "≤",
};

function seqValue(v: SequenceValueDto | null, t: Translate): string {
  if (!v) return t("seq.noValue");
  if (v.type === "bool") return v.value ? "TRUE" : "FALSE";
  return String(Math.round(v.value * 1000) / 1000);
}

function seconds(us: string): string {
  return (Number(us) / 1_000_000).toFixed(1);
}

/** A condition comparison as an engineer reads it: `NOT InletOpen = TRUE`. */
function rowText(r: SequenceConditionRowDto, t: Translate) {
  return `${r.negated ? "NOT " : ""}${r.source} ${OP[r.op]} ${seqValue(r.expected, t)}`;
}

function Mark({ ok }: { ok: boolean | null }) {
  const { t } = useInterface();
  const label =
    ok === null
      ? t("seq.noValue")
      : ok
        ? t("seq.satisfied")
        : t("seq.notSatisfied");
  return (
    <span
      className={`seq-mark ${ok === null ? "unknown" : ok ? "ok" : "no"}`}
      role="img"
      aria-label={label}
      title={label}
    >
      {ok === null ? "?" : ok ? "✓" : "✗"}
    </span>
  );
}

function Condition({
  condition,
  advanced,
}: {
  condition: SequenceConditionDto;
  advanced: boolean;
}) {
  const { t } = useInterface();
  return (
    <ul className="seq-rows">
      {condition.rows.map((r, i) => (
        <li key={`${r.signal}-${i}`}>
          <Mark ok={r.satisfied === null ? null : r.satisfied !== r.negated} />
          <code>{rowText(r, t)}</code>
          <span className="muted">
            {" "}
            {t("seq.now", { value: seqValue(r.actual, t) })}
            {advanced && ` · ${t(LAYER[r.layer])}`}
          </span>
        </li>
      ))}
    </ul>
  );
}

export function SequenceTab({
  definition,
  status,
  alarms,
  advanced,
}: {
  definition: SequenceDefinitionDto;
  status: SequenceStatusDto | null;
  alarms: AlarmDto[];
  advanced: boolean;
}) {
  const { t } = useInterface();
  if (!status)
    return <p className="inspector-section muted">{t("seq.noStatus")}</p>;
  const steps = new Map(definition.steps.map((s) => [s.id, s]));
  const transitions = new Map(definition.transitions.map((x) => [x.id, x]));
  const interlocks = new Map(definition.interlocks.map((x) => [x.id, x]));
  const stepLabel = (id: string | null) => {
    const s = id ? steps.get(id) : undefined;
    return s ? `S${s.number} ${s.name}` : (id ?? "—");
  };
  const name = (map: Map<string, { name: string }>, id: string | null) =>
    (id && map.get(id)?.name) ?? id ?? "";
  const permissiveName = (id: string) => {
    for (const x of definition.transitions)
      for (const p of x.permissives) if (p.id === id) return p.name;
    return id;
  };
  const index = definition.steps.findIndex((s) => s.id === status.activeStep);
  const outgoing = status.transitions.filter((x) => x.fromActive);
  const timeout = status.timeoutUs ? Number(status.timeoutUs) : null;
  const elapsed = Number(status.elapsedUs);
  const prefix = `sequence.${definition.id}.`;
  const related = alarms.filter((a) => a.id.startsWith(prefix));
  const alarmLabel = (a: AlarmDto) => {
    const interlock = a.id.slice(prefix.length).replace(/^interlock\./, "");
    return interlocks.get(interlock)?.name ?? definition.name;
  };
  const eventText = (e: SequenceEventDto) =>
    t(EVENT[e.code], {
      step: stepLabel(e.step),
      from: stepLabel(e.from),
      transition: name(transitions, e.transition),
      interlock: name(interlocks, e.interlock),
      value: e.stepValue ?? "?",
      start: e.start ? t(START[e.start]) : "",
    });
  const eventDetails = (e: SequenceEventDto) =>
    [
      ...e.permissives.map(permissiveName),
      ...e.interlocks.map((id) => name(interlocks, id)),
      ...(advanced
        ? e.values.map((v) => `${v.source} = ${seqValue(v.value, t)}`)
        : []),
    ].join(", ");
  return (
    <div className="sequence-inspector">
      <section className="inspector-section" aria-label={t("seq.status")}>
        <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
          <h4 style={{ margin: 0 }}>{definition.name}</h4>
          <Badge kind={BADGE[status.state]}>{t(STATE[status.state])}</Badge>
        </div>
        <p data-testid="sequence-explain">
          {t(EXPLAIN[status.state], {
            step: stepLabel(status.activeStep),
            reason: status.fault
              ? t(REASON[status.fault] ?? "seq.reason.unknown")
              : "",
          })}
        </p>
        <dl className="kv">
          <dt>{t("seq.active")}</dt>
          <dd data-testid="sequence-active">{stepLabel(status.activeStep)}</dd>
          {index >= 0 && (
            <>
              <dt>{t("seq.progress")}</dt>
              <dd>
                {t("seq.progressValue", {
                  index: index + 1,
                  count: definition.steps.length,
                })}
              </dd>
            </>
          )}
          {advanced && (
            <>
              <dt>{t("seq.previous")}</dt>
              <dd>{stepLabel(status.previousStep)}</dd>
              <dt>{t("seq.next")}</dt>
              <dd>
                {outgoing.length
                  ? outgoing
                      .map((x) => stepLabel(transitions.get(x.id)?.to ?? null))
                      .join(", ")
                  : "—"}
              </dd>
            </>
          )}
          <dt>{t("seq.elapsed")}</dt>
          <dd data-testid="sequence-timer">
            {timeout !== null
              ? t("seq.timerOf", {
                  elapsed: seconds(status.elapsedUs),
                  timeout: seconds(status.timeoutUs ?? "0"),
                  remaining: (Math.max(0, timeout - elapsed) / 1e6).toFixed(1),
                })
              : t("seq.timer", { elapsed: seconds(status.elapsedUs) })}
          </dd>
        </dl>
        {timeout !== null && (
          <progress
            className={`seq-timer ${elapsed >= timeout ? "expired" : ""}`}
            max={timeout}
            value={Math.min(elapsed, timeout)}
            aria-label={t("seq.elapsed")}
          />
        )}
        <p className="muted seq-note">{t("seq.observer")}</p>
      </section>
      {outgoing.length > 0 && (
        <section className="inspector-section" aria-label={t("seq.waiting")}>
          <h4>{t("seq.waiting")}</h4>
          {outgoing.map((x) => {
            const def = transitions.get(x.id);
            const refused = x.permissives.filter(
              (p) => p.condition.satisfied !== true,
            );
            return (
              <div key={x.id} className="seq-transition">
                <div>
                  <Mark ok={x.condition.satisfied} />{" "}
                  <strong>{def?.name ?? x.id}</strong>{" "}
                  <span className="muted">→ {stepLabel(def?.to ?? null)}</span>
                </div>
                <Condition condition={x.condition} advanced={advanced} />
                {(refused.length > 0 || x.blockedBy.length > 0) && (
                  <div className="seq-blocked" role="note">
                    {t("seq.notPermitted", {
                      reasons: [
                        ...refused.map((p) => permissiveName(p.id)),
                        ...x.blockedBy.map((id) => name(interlocks, id)),
                      ].join(", "),
                    })}
                  </div>
                )}
                {x.permissives.length > 0 && (
                  <div className="seq-permissives">
                    <h5>{t("seq.permissives")}</h5>
                    {x.permissives.map((p) => (
                      <div key={p.id}>
                        <Mark ok={p.condition.satisfied} />{" "}
                        {permissiveName(p.id)}
                        {advanced && (
                          <Condition condition={p.condition} advanced />
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </section>
      )}
      <section className="inspector-section" aria-label={t("seq.chart")}>
        <h4>{t("seq.chart")}</h4>
        <ol className="seq-chart">
          {definition.steps.map((s) => {
            const state =
              status.steps.find((x) => x.id === s.id)?.state ?? "INACTIVE";
            return (
              <li
                key={s.id}
                className={`seq-step ${state.toLowerCase()}`}
                aria-current={s.id === status.activeStep ? "step" : undefined}
              >
                <div className="seq-box">
                  <span className="seq-number">S{s.number}</span> {s.name}
                  <span className="seq-state">{t(STEP_STATE[state])}</span>
                </div>
                {definition.transitions
                  .filter((x) => x.from === s.id)
                  .map((x) => {
                    const live = status.transitions.find((y) => y.id === x.id);
                    return (
                      <div key={x.id} className="seq-arrow">
                        ↓ {x.name} → S{steps.get(x.to)?.number}
                        {live?.fromActive && (
                          <>
                            {" "}
                            <Mark ok={live.condition.satisfied} />
                          </>
                        )}
                      </div>
                    );
                  })}
              </li>
            );
          })}
        </ol>
      </section>
      <section className="inspector-section" aria-label={t("seq.interlocks")}>
        <h4>{t("seq.interlocks")}</h4>
        {definition.interlocks.length === 0 && (
          <p className="muted">{t("seq.none")}</p>
        )}
        {definition.interlocks.map((def) => {
          const live = status.interlocks.find((x) => x.id === def.id);
          return (
            <div key={def.id} className="seq-interlock">
              <Badge kind={live?.active ? "warning" : "normal"}>
                {live?.active
                  ? t("seq.interlockActive")
                  : live?.applies
                    ? t("seq.interlockClear")
                    : t("seq.interlockElsewhere")}
              </Badge>{" "}
              {def.name}
              <div className="muted">
                {t(REACTION[def.reaction], { ms: def.reactionMs ?? 0 })}
              </div>
              {advanced && live && (
                <Condition condition={live.condition} advanced />
              )}
            </div>
          );
        })}
      </section>
      {related.length > 0 && (
        <section className="inspector-section" aria-label={t("seq.alarms")}>
          <h4>{t("seq.alarms")}</h4>
          <ul className="seq-alarms">
            {related.map((a) => (
              <li key={a.id}>
                <Badge kind={priorityBadge(a.priority)}>
                  {t(ALARM_STATE[a.state])}
                </Badge>{" "}
                {alarmLabel(a)}: {alarmCondition(t, a.kind, a.text)}
              </li>
            ))}
          </ul>
        </section>
      )}
      {advanced && (
        <section className="inspector-section" aria-label={t("seq.history")}>
          <h4>
            {t("seq.history")}{" "}
            <span className="muted">
              {t("seq.historyCount", { count: status.totalEvents })}
            </span>
          </h4>
          <table className="table seq-history">
            <thead>
              <tr>
                <th>{t("seq.time")}</th>
                <th>{t("seq.event")}</th>
                <th>{t("seq.details")}</th>
              </tr>
            </thead>
            <tbody>
              {[...status.history].reverse().map((e) => (
                <tr key={e.seq} className={e.fault ? "seq-fault" : undefined}>
                  <td className="mono">{seconds(e.timeUs)} s</td>
                  <td>{eventText(e)}</td>
                  <td className="muted">{eventDetails(e)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}

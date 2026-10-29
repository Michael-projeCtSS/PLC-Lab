/**
 * Causal inspector for one PLC cycle, rendered from the backend observation:
 * PLC read (scan input image) → logic result (committed values) → output command as
 * received by the field devices → process response → inputs staged for the next scan.
 * Nothing is recomputed here.
 */
import { useInterface } from "../locale";
import type { CauseEffectDto, EffectDto, SignalDto } from "@plc-lab/ipc";
import { formatValue, seconds, valueClass } from "../format";

function Items({
  items,
  signals,
  empty,
}: {
  items: EffectDto[];
  signals: Map<number, SignalDto>;
  empty: string;
}) {
  if (items.length === 0) return <div className="dim">{empty}</div>;
  return (
    <>
      {items.map((e) => {
        const s = signals.get(e.signal);
        return (
          <div
            key={e.signal}
            className={`ce-item${e.changed ? " changed" : ""}`}
          >
            <span className="ce-name">{s?.label ?? `#${e.signal}`}</span>
            <span
              className={`${valueClass(e.value)}${e.changed ? " changed" : ""}`}
            >
              {formatValue(e.value, s?.unit ?? null)}
            </span>
          </div>
        );
      })}
    </>
  );
}

export function CauseEffect({
  ce,
  signals,
}: {
  ce: CauseEffectDto | null;
  signals: SignalDto[];
}) {
  const { t, preferences } = useInterface();
  if (!ce)
    return (
      <div className="cause-empty">
        <h3>{t("cause.title")}</h3>
        <p>{t("cause.empty")}</p>
        {preferences.explanations && (
          <ol>
            {(
              [
                "cause.read",
                "cause.decided",
                "cause.command",
                "cause.process",
                "cause.next",
              ] as const
            ).map((key) => (
              <li key={key}>{t(key).replace(/^\d · /, "")}</li>
            ))}
          </ol>
        )}
      </div>
    );
  const map = new Map(signals.map((s) => [s.id, s]));
  const scanned = ce.outcome === "SCANNED" || ce.outcome === "FAULTED";
  return (
    <div className="ce" aria-label={t("cause.cycle", { cycle: ce.cycle })}>
      <div className="row muted" style={{ fontSize: 11, marginBottom: 4 }}>
        {t("cause.cycle", { cycle: ce.cycle })} · {seconds(ce.startUs)} →{" "}
        {seconds(ce.endUs)}
        <span className="spacer" />
        {ce.outcome === "SCANNED"
          ? t("cause.scanned")
          : ce.outcome === "FAULTED"
            ? "PLC FAULT"
            : ce.outcome === "NOT_SCANNED_FAULT"
              ? t("cause.fault")
              : t("cause.stop")}
      </div>
      <div className="ce-step">
        <h5>
          {t("cause.read")} <span>{t("cause.inputImage")}</span>
        </h5>
        <Items items={ce.saw} signals={map} empty={t("cause.noInputs")} />
      </div>
      <div className="ce-arrow">{t("cause.logic")}</div>
      <div className="ce-step">
        <h5>
          {t("cause.decided")}{" "}
          <span>{scanned ? t("cause.committed") : t("cause.held")}</span>
        </h5>
        {ce.fault ? (
          <div className="notice error">
            {ce.fault.code}: {ce.fault.message}
          </div>
        ) : (
          <Items
            items={ce.decided}
            signals={map}
            empty={t("cause.noOutputs")}
          />
        )}
      </div>
      <div className="ce-arrow">{t("cause.modules")}</div>
      <div className="ce-step">
        <h5>
          {t("cause.command")} <span>{t("cause.received")}</span>
        </h5>
        <Items items={ce.applied} signals={map} empty={t("cause.noActuator")} />
      </div>
      <div className="ce-arrow">{t("cause.machine")}</div>
      <div className="ce-step">
        <h5>
          {t("cause.process")} <span>{t("cause.physical")}</span>
        </h5>
        {ce.process.length === 0 ? (
          <div className="dim">{t("cause.noChange")}</div>
        ) : (
          ce.process.map((c) => {
            const s = map.get(c.signal);
            return (
              <div key={c.signal} className="ce-item changed">
                <span className="ce-name">{s?.label ?? `#${c.signal}`}</span>
                <span className="value">
                  {formatValue(c.before, s?.unit ?? null)} →{" "}
                  {formatValue(c.after, s?.unit ?? null)}
                </span>
              </div>
            );
          })
        )}
      </div>
      <div className="ce-arrow">{t("cause.sensors")}</div>
      <div className="ce-step">
        <h5>
          {t("cause.next")} <span>{t("cause.staged")}</span>
        </h5>
        <Items items={ce.next} signals={map} empty={t("cause.noInputs")} />
      </div>
    </div>
  );
}

/**
 * Causal inspector for one PLC cycle, rendered from the backend observation:
 * PLC read (scan input image) → logic result (committed values) → output command as
 * received by the field devices → process response → inputs staged for the next scan.
 * Nothing is recomputed here.
 */
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
  if (!ce)
    return (
      <p className="muted">
        Run or step the simulation to see what the PLC read, decided and caused
        in each cycle.
      </p>
    );
  const map = new Map(signals.map((s) => [s.id, s]));
  const scanned = ce.outcome === "SCANNED" || ce.outcome === "FAULTED";
  return (
    <div className="ce" aria-label={`Cycle ${ce.cycle}`}>
      <div className="row muted" style={{ fontSize: 11, marginBottom: 4 }}>
        Cycle #{ce.cycle} · {seconds(ce.startUs)} → {seconds(ce.endUs)}
        <span className="spacer" />
        {ce.outcome === "SCANNED"
          ? "PLC scanned"
          : ce.outcome === "FAULTED"
            ? "PLC FAULT"
            : ce.outcome === "NOT_SCANNED_FAULT"
              ? "PLC in FAULT: no scan"
              : "PLC in STOP: no scan"}
      </div>
      <div className="ce-step">
        <h5>
          1 · PLC read <span>input image</span>
        </h5>
        <Items items={ce.saw} signals={map} empty="No inputs declared." />
      </div>
      <div className="ce-arrow">▼ program logic</div>
      <div className="ce-step">
        <h5>
          2 · PLC logic result{" "}
          <span>{scanned ? "committed" : "outputs held off"}</span>
        </h5>
        {ce.fault ? (
          <div className="notice error">
            {ce.fault.code}: {ce.fault.message}
          </div>
        ) : (
          <Items
            items={ce.decided}
            signals={map}
            empty="No outputs or variables."
          />
        )}
      </div>
      <div className="ce-arrow">▼ output modules switch the field devices</div>
      <div className="ce-step">
        <h5>
          3 · Output command <span>received by the devices</span>
        </h5>
        <Items
          items={ce.applied}
          signals={map}
          empty="No actuator in this machine."
        />
      </div>
      <div className="ce-arrow">▼ the machine runs for one cycle</div>
      <div className="ce-step">
        <h5>
          4 · Process <span>physical change</span>
        </h5>
        {ce.process.length === 0 ? (
          <div className="dim">No physical change in this cycle.</div>
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
      <div className="ce-arrow">▼ sensors sampled at the end of the cycle</div>
      <div className="ce-step">
        <h5>
          5 · Next PLC input <span>staged for the next scan</span>
        </h5>
        <Items items={ce.next} signals={map} empty="No inputs declared." />
      </div>
    </div>
  );
}

/** Parts of the Hardware view: the configured-vs-actual findings, the signal mapping,
 * station editing (pure helpers on a draft configuration) and inspector sections. Every
 * state shown comes from the backend; the editors only change the draft configuration
 * that is downloaded to the PLC. */
import { useState } from "react";
import type {
  DiagnosticEntryDto,
  HardwareDto,
  HardwareFindingDto,
  ModuleTypeDto,
  ModuleViewDto,
  SignalMapRowDto,
  StationDto,
} from "@plc-lab/ipc";
import { formatValue } from "../format";
import { Badge } from "../ui";
import { addModule, freeSlots } from "./stationEdit";

const CATEGORY_LABEL: Record<string, string> = {
  MODULE_MISSING: "Module missing",
  UNEXPECTED_MODULE: "Unexpected module",
  WRONG_MODULE_TYPE: "Wrong module type",
  WRONG_SLOT: "Module in another slot",
  MODULE_POWER_MISSING: "Module not answering",
  FIELD_POWER_MISSING: "Field supply missing",
  CHANNEL_MODE_MISMATCH: "Channel mode mismatch",
  LOOP_SUPPLY_MISMATCH: "Loop supply mismatch",
  ADDRESS_OVERLAP: "Address overlap",
  SIGNAL_BINDING_MISMATCH: "Signal mapping",
};

export function FindingsPanel({
  findings,
}: {
  findings: readonly HardwareFindingDto[];
}) {
  return (
    <section className="panel" aria-label="Configured versus actual">
      <div className="panel-head">
        <h4>Configured versus actual</h4>
        <span className="spacer" />
        <span className="muted">
          project, detected modules and the drawing compared
        </span>
      </div>
      {findings.length === 0 ? (
        <p className="panel-body muted" style={{ margin: 0 }}>
          The detected hardware matches the project configuration and the
          drawing.
        </p>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>Severity</th>
              <th>Finding</th>
              <th>Expected</th>
              <th>Detected</th>
              <th>Details</th>
            </tr>
          </thead>
          <tbody>
            {findings.map((f, i) => (
              <tr key={i}>
                <td>
                  <Badge kind={f.severity === "error" ? "fault" : "warning"}>
                    {f.severity}
                  </Badge>
                </td>
                <td>{CATEGORY_LABEL[f.category] ?? f.category}</td>
                <td>{f.expected ?? "—"}</td>
                <td>{f.detected ?? "—"}</td>
                <td>{f.message}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

export function SignalMapPanel({
  signals,
}: {
  signals: readonly SignalMapRowDto[];
}) {
  return (
    <section className="panel" aria-label="Signal mapping">
      <div className="panel-head">
        <h4>Signal mapping</h4>
        <span className="spacer" />
        <span className="muted">
          PLC symbol → configured channel → device on the drawing
        </span>
      </div>
      <table className="table">
        <thead>
          <tr>
            <th>PLC symbol</th>
            <th>Channel</th>
            <th>Address</th>
            <th>Channel value</th>
            <th>Quality</th>
            <th>Device on the drawing</th>
            <th>Device state</th>
          </tr>
        </thead>
        <tbody>
          {signals.length === 0 && (
            <tr>
              <td colSpan={7} className="muted">
                No I/O mapping is configured.
              </td>
            </tr>
          )}
          {signals.map((s) => (
            <tr key={`${s.tag}-${s.channelId ?? s.address}`}>
              <td>{s.tag}</td>
              <td className="mono">{s.channelId ?? "—"}</td>
              <td className="mono">{s.address}</td>
              <td className="mono">
                {s.output !== null
                  ? s.output
                    ? "ON"
                    : "OFF"
                  : formatValue(s.value)}
              </td>
              <td>
                {s.quality && s.quality !== "GOOD"
                  ? s.quality.replaceAll("_", " ").toLowerCase()
                  : s.quality
                    ? "good"
                    : "—"}
              </td>
              <td>
                {s.drawnDevice
                  ? `${s.drawnDevice} (${s.drawnPort ?? ""})`
                  : "not on the drawing"}
              </td>
              <td>{s.deviceState ?? "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

/** Add a module to the draft configuration (Advanced mode). */
export function StationEditor({
  station,
  catalog,
  onChange,
}: {
  station: StationDto;
  catalog: readonly ModuleTypeDto[];
  onChange: (next: StationDto) => void;
}) {
  const slots = freeSlots(station);
  const [type, setType] = useState(catalog[0]?.id ?? "");
  const [slot, setSlot] = useState<number | null>(null);
  const target = slot ?? slots[0] ?? null;
  const chosen = catalog.find((t) => t.id === type);
  return (
    <div className="row wrap" style={{ gap: 8, padding: "8px 16px" }}>
      <strong style={{ fontSize: 12 }}>Add module</strong>
      <label className="row muted" style={{ fontSize: 12 }}>
        Type
        <select
          className="input"
          value={type}
          onChange={(e) => setType(e.target.value)}
        >
          {catalog.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      </label>
      <label className="row muted" style={{ fontSize: 12 }}>
        Slot
        <select
          className="input"
          value={target ?? ""}
          onChange={(e) => setSlot(Number(e.target.value))}
        >
          {slots.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </label>
      <button
        className="btn btn-sm"
        disabled={!chosen || target === null}
        onClick={() => {
          if (chosen && target !== null)
            onChange(addModule(station, chosen, target, catalog));
          setSlot(null);
        }}
      >
        Add to configuration
      </button>
    </div>
  );
}

/** What the module's supplies show (evidence, not the cause). */
export function PowerSection({ module }: { module: ModuleViewDto }) {
  const electronics =
    module.status === "MISSING"
      ? "no module answers at this slot"
      : module.status === "NO_MODULE_POWER"
        ? "a module is installed but does not answer; its status LEDs are dark"
        : "the module answers the CPU";
  const field =
    module.status === "NO_FIELD_POWER"
      ? "missing: the module reports no field (load) supply"
      : module.status === "OK"
        ? "present"
        : "unknown while the module does not answer correctly";
  return (
    <dl className="kv" style={{ padding: 12 }}>
      <dt>Electronics (backplane)</dt>
      <dd>{electronics}</dd>
      <dt>Field / load supply</dt>
      <dd>{field}</dd>
      <dt>Note</dt>
      <dd className="muted">
        Outputs need the field supply to switch their terminals; inputs need it
        to supply their sensors. Measure the supply terminals to confirm.
      </dd>
    </dl>
  );
}

export function ModuleDiagnostics({
  module,
  data,
}: {
  module: ModuleViewDto;
  data: HardwareDto;
}) {
  const here = data.findings.filter(
    (f) =>
      f.slot?.rack === module.position.rack &&
      f.slot.slot === module.position.slot,
  );
  // Typed identity from the backend; the display text in `source` is never parsed.
  const entries: DiagnosticEntryDto[] = data.diagnostics
    .filter(
      (d) =>
        d.slot?.rack === module.position.rack &&
        d.slot.slot === module.position.slot,
    )
    .slice(-20);
  return (
    <section
      className="col"
      style={{ padding: 12 }}
      aria-label="Module diagnostics"
    >
      {here.length === 0 && entries.length === 0 && (
        <p className="muted" style={{ margin: 0 }}>
          No finding and no diagnostic for this module.
        </p>
      )}
      {here.map((f, i) => (
        <div
          key={i}
          className={`notice ${f.severity === "error" ? "error" : "warning"}`}
        >
          <strong>{CATEGORY_LABEL[f.category] ?? f.category}:</strong>{" "}
          {f.message}
        </div>
      ))}
      {entries.length > 0 && (
        <table className="table">
          <tbody>
            {entries.map((d, i) => (
              <tr key={i}>
                <td className="mono dim">{d.event}</td>
                <td>{d.text}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

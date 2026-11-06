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
import { useInterface } from "../locale";
import {
  backendText,
  diagnosticText,
  moduleText,
  translateCode,
  type Translate,
} from "../locale/messages";
import { Badge } from "../ui";
import { addModule, freeSlots } from "./stationEdit";

function categoryLabel(t: Translate, category: string): string {
  return translateCode(t, `hwp.category.${category}`, category);
}
function qualityWord(t: Translate, quality: string): string {
  return translateCode(
    t,
    `q.${quality}`,
    quality.replaceAll("_", " ").toLowerCase(),
  );
}

/** A finding's identity: its category and the typed hardware it is about. */
function findingKey(f: HardwareFindingDto): string {
  return [
    f.category,
    f.moduleId ?? "",
    f.slot ? `${f.slot.rack}/${f.slot.slot}` : "",
    f.channel ? `${f.channel.channel}` : "",
    f.message,
  ].join("|");
}

export function FindingsPanel({
  findings,
}: {
  findings: readonly HardwareFindingDto[];
}) {
  const { t, preferences } = useInterface();
  const say = (m: HardwareFindingDto["detail"] | null, text: string | null) =>
    backendText(preferences.language, m, text ?? "—") || "—";
  return (
    <section className="panel" aria-label={t("hwp.compare")}>
      <div className="panel-head">
        <h4>{t("hwp.compare")}</h4>
        <span className="spacer" />
        <span className="muted">{t("hwp.compareHelp")}</span>
      </div>
      {findings.length === 0 ? (
        <p className="panel-body muted" style={{ margin: 0 }}>
          {t("hwp.matches")}
        </p>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>{t("hwp.severity")}</th>
              <th>{t("hwp.finding")}</th>
              <th>{t("hwp.expected")}</th>
              <th>{t("hwp.detected")}</th>
              <th>{t("hwp.details")}</th>
            </tr>
          </thead>
          <tbody>
            {findings.map((f) => (
              <tr key={findingKey(f)}>
                <td>
                  <Badge kind={f.severity === "error" ? "fault" : "warning"}>
                    {t(`diag.severity.${f.severity}`)}
                  </Badge>
                </td>
                <td>{categoryLabel(t, f.category)}</td>
                <td>{say(f.expectedMessage, f.expected)}</td>
                <td>{say(f.detectedMessage, f.detected)}</td>
                <td>{say(f.detail, f.message)}</td>
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
  const { t, preferences } = useInterface();
  return (
    <section className="panel" aria-label={t("hwp.signals")}>
      <div className="panel-head">
        <h4>{t("hwp.signals")}</h4>
        <span className="spacer" />
        <span className="muted">{t("hwp.signalsHelp")}</span>
      </div>
      <table className="table">
        <thead>
          <tr>
            <th>{t("hwp.symbol")}</th>
            <th>{t("hwp.channel")}</th>
            <th>{t("hwp.address")}</th>
            <th>{t("hwp.channelValue")}</th>
            <th>{t("hwp.quality")}</th>
            <th>{t("hwp.device")}</th>
            <th>{t("hwp.deviceState")}</th>
          </tr>
        </thead>
        <tbody>
          {signals.length === 0 && (
            <tr>
              <td colSpan={7} className="muted">
                {t("hwp.noMapping")}
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
                    ? t("hw.on")
                    : t("hw.off")
                  : formatValue(s.value)}
              </td>
              <td>{s.quality ? qualityWord(t, s.quality) : "—"}</td>
              <td>
                {s.drawnDevice
                  ? `${s.drawnDevice} (${s.drawnPort ?? ""})`
                  : t("hwp.notDrawn")}
              </td>
              <td>
                {backendText(
                  preferences.language,
                  s.deviceStateMessage,
                  s.deviceState ?? "—",
                )}
              </td>
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
  const chosen = catalog.find((m) => m.id === type);
  const { t } = useInterface();
  return (
    <div className="row wrap" style={{ gap: 8, padding: "8px 16px" }}>
      <strong style={{ fontSize: 12 }}>{t("hwp.add")}</strong>
      <label className="row muted" style={{ fontSize: 12 }}>
        {t("hwp.type")}
        <select
          className="input"
          value={type}
          onChange={(e) => setType(e.target.value)}
        >
          {catalog.map((m) => (
            <option key={m.id} value={m.id}>
              {moduleText(t, m.name)}
            </option>
          ))}
        </select>
      </label>
      <label className="row muted" style={{ fontSize: 12 }}>
        {t("hwp.slot")}
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
        {t("hwp.addButton")}
      </button>
    </div>
  );
}

/** What the module's supplies show (evidence, not the cause). */
export function PowerSection({ module }: { module: ModuleViewDto }) {
  const { t } = useInterface();
  const electronics =
    module.status === "MISSING"
      ? t("hwp.noAnswer")
      : module.status === "NO_MODULE_POWER"
        ? t("hwp.silent")
        : t("hwp.answers");
  const field =
    module.status === "NO_FIELD_POWER"
      ? t("hwp.fieldMissing")
      : module.status === "OK"
        ? t("hwp.present")
        : t("hwp.unknown");
  return (
    <dl className="kv" style={{ padding: 12 }}>
      <dt>{t("hwp.electronics")}</dt>
      <dd>{electronics}</dd>
      <dt>{t("hwp.field")}</dt>
      <dd>{field}</dd>
      <dt>{t("hwp.note")}</dt>
      <dd className="muted">{t("hwp.noteText")}</dd>
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
  const { t, preferences } = useInterface();
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
      aria-label={t("hwp.moduleDiagnostics")}
    >
      {here.length === 0 && entries.length === 0 && (
        <p className="muted" style={{ margin: 0 }}>
          {t("hwp.noDiagnostics")}
        </p>
      )}
      {here.map((f) => (
        <div
          key={findingKey(f)}
          className={`notice ${f.severity === "error" ? "error" : "warning"}`}
        >
          <strong>{categoryLabel(t, f.category)}:</strong>{" "}
          {backendText(preferences.language, f.detail, f.message)}
        </div>
      ))}
      {entries.length > 0 && (
        <table className="table">
          <tbody>
            {entries.map((d, i) => (
              <tr key={i}>
                <td className="mono dim">{t(`hwp.event.${d.event}`)}</td>
                <td title={d.text}>{diagnosticText(t, d)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

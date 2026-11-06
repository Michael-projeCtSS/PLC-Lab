/**
 * Hardware view: the project configuration, the detected (actual) station and their
 * comparison, the signal mapping, live channel values, LEDs, qualities, addresses and a
 * multimeter. Advanced mode edits the project's station configuration (add, remove, move
 * modules, change types and channel modes): every edit is saved and validated by the
 * backend, and the saved project is shown. Download brings it into the PLC; Compare
 * shows the configuration loaded in the PLC against the detected hardware.
 */
import { useState } from "react";
import {
  api,
  type ChannelConfigDto,
  type HardwareDto,
  type MeasurementDto,
  type ModuleViewDto,
  type StationDto,
} from "@plc-lab/ipc";
import { formatValue, valueClass } from "../format";
import { useInterface } from "../locale";
import {
  moduleStatus,
  moduleText,
  qualityLabel,
  type Translate,
} from "../locale/messages";
import { Badge, ErrorNotice, Tabs } from "../ui";
import {
  FindingsPanel,
  ModuleDiagnostics,
  PowerSection,
  SignalMapPanel,
  StationEditor,
} from "./HardwareParts";
import {
  freeSlots,
  moveModule,
  removeModule,
  sameStation,
} from "./stationEdit";
import type { Selection } from "./ProjectTree";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

const LED_CLASS: Record<string, string> = {
  OFF: "dot",
  GREEN: "dot on",
  GREEN_FLASHING: "dot on flash",
  RED: "dot red",
  RED_FLASHING: "dot red flash",
  AMBER: "dot amber",
};
const STATUS_KIND: Record<string, string> = {
  OK: "active",
  MISSING: "fault",
  MISMATCH: "fault",
  NO_FIELD_POWER: "fault",
  NO_MODULE_POWER: "fault",
  UNCONFIGURED: "warning",
};
type View = "compare" | "configured" | "actual" | "signals";
type Section =
  "overview" | "configuration" | "channels" | "power" | "diagnostics";
const MODES: ChannelConfigDto["mode"][] = [
  "current4To20mA",
  "current0To20mA",
  "voltage0To10V",
  "disabled",
];
function modeLabel(t: Translate, mode: string): string {
  switch (mode) {
    case "digital":
      return t("hw.mode.digital");
    case "current4To20mA":
      return "4–20 mA";
    case "current0To20mA":
      return "0–20 mA";
    case "voltage0To10V":
      return "0–10 V";
    case "disabled":
      return t("hw.mode.disabled");
    default:
      return mode;
  }
}
export function HardwareTab({
  lab,
  selection,
  select,
  advanced,
}: {
  lab: Lab;
  selection: Selection;
  select: (s: Selection) => void;
  advanced: boolean;
}) {
  const { t } = useInterface();
  const { ws } = lab;
  const hw = useFrameQuery<HardwareDto>(lab, () =>
    ws ? api.hardware(ws) : Promise.reject(new Error("No workspace")),
  );
  const [measurement, setMeasurement] = useState<{
    channel: string;
    address: string;
    dto: MeasurementDto;
  } | null>(null);
  const [view, setView] = useState<View>("compare");
  const [section, setSection] = useState<Section>("overview");
  useEvidence(lab, "hardware view");
  const data = hw.data;
  if (!data || !ws) return <ErrorNotice error={hw.error} />;
  // The project's (offline) configuration; the PLC holds `data.station` until download.
  const project = lab.project;
  const station = project?.station ?? data.station;
  const pending = !!project && !sameStation(project.station, data.station);
  const edit = (next: StationDto) => void lab.saveStation(next);
  /** The project's module for a module view: by stable id, else by position. */
  const configuredOf = (s: StationDto, m: ModuleViewDto) =>
    s.modules.find((x) =>
      m.moduleId
        ? x.id === m.moduleId
        : x.position.rack === m.position.rack &&
          x.position.slot === m.position.slot,
    );
  const selected =
    selection?.kind === "module"
      ? (data.modules.find(
          (m) =>
            m.position.rack === selection.rack &&
            m.position.slot === selection.slot,
        ) ?? null)
      : null;
  const editMode = (
    m: ModuleViewDto,
    channel: number,
    mode: ChannelConfigDto["mode"],
  ) => {
    const next: StationDto = structuredClone(station);
    const module = configuredOf(next, m);
    if (!module) return;
    const existing = module.channels.find((c) => c.channel === channel);
    if (existing) existing.mode = mode;
    else
      module.channels.push({
        channel,
        mode,
        diagnostics: true,
        stopBehavior: "off",
        loopSupply: true,
      });
    edit(next);
  };
  /** Current inputs: whether the channel supplies the loop (2-wire) or is passive (4-wire). */
  const editLoopSupply = (
    m: ModuleViewDto,
    channel: number,
    supply: boolean,
  ) => {
    const next: StationDto = structuredClone(station);
    const module = configuredOf(next, m);
    const existing = module?.channels.find((c) => c.channel === channel);
    if (!existing) return;
    existing.loopSupply = supply;
    edit(next);
  };
  const editModule = (m: ModuleViewDto, type: string) => {
    const next: StationDto = structuredClone(station);
    const module = configuredOf(next, m);
    if (!module) return;
    module.module = type;
    module.channels = [];
    edit(next);
  };
  return (
    <div style={{ position: "absolute", inset: 0, overflow: "auto" }}>
      <div
        className="row"
        style={{ padding: "8px 16px", borderBottom: "1px solid var(--line)" }}
      >
        <strong>{station.name}</strong>
        <span className="muted">
          {moduleText(t, data.cpuName)}
          {data.cpuProduct
            ? ` · ${t("hw.profile", { product: data.cpuProduct })}`
            : ""}
        </span>
        <span className="spacer" />
        {data.cpuLeds.map((l) => (
          <span key={l.name} className="row" style={{ gap: 4, fontSize: 11 }}>
            <span className={LED_CLASS[l.state] ?? "dot"} /> {l.name}
          </span>
        ))}
        <span
          className="muted"
          style={{ fontSize: 11 }}
          title={t("hw.ledsHelp")}
        >
          {t("hw.leds")}
        </span>
      </div>
      {pending && project && (
        <div
          className="notice warning row"
          style={{ margin: "8px 16px" }}
          role="status"
        >
          <span className="grow">{t("hw.pending")}</span>
          <button
            className="btn btn-sm btn-primary"
            disabled={lab.busy !== null}
            onClick={() => void lab.buildAndLoad(false, project.station)}
          >
            {t("hw.download")}
          </button>
          <button className="btn btn-sm" onClick={() => edit(data.station)}>
            {t("hw.discard")}
          </button>
        </div>
      )}
      <Tabs<View>
        label={t("hw.views")}
        active={view}
        onSelect={setView}
        tabs={[
          {
            id: "compare",
            label: t("hw.view.compare"),
            ...(data.findings.length > 0
              ? {
                  badge: <Badge kind="warning">{data.findings.length}</Badge>,
                }
              : {}),
          },
          { id: "configured", label: t("hw.view.configured") },
          { id: "actual", label: t("hw.view.actual") },
          { id: "signals", label: t("hw.view.signals") },
        ]}
      />
      {advanced && view !== "actual" && view !== "signals" && (
        <StationEditor
          station={station}
          catalog={data.catalog}
          onChange={edit}
        />
      )}
      {view === "signals" && (
        <div style={{ padding: 16 }}>
          <SignalMapPanel signals={data.signals} />
        </div>
      )}
      {view === "configured" && (
        <div
          className="rack"
          role="list"
          aria-label={t("hw.configuredStation")}
        >
          {station.modules.map((m) => {
            const type = data.catalog.find((t) => t.id === m.module);
            return (
              <div
                key={m.id}
                role="listitem"
                className={`module${selected?.position.slot === m.position.slot ? " selected" : ""}`}
                onClick={() =>
                  select({
                    kind: "module",
                    rack: m.position.rack,
                    slot: m.position.slot,
                  })
                }
              >
                <button
                  type="button"
                  className="module-head"
                  aria-pressed={selected?.position.slot === m.position.slot}
                >
                  <span className="slot">
                    {t("hw.slot", { slot: m.position.slot })}
                  </span>
                  <strong style={{ fontSize: 11 }}>{m.id}</strong>
                  <span className="dim">
                    {type ? moduleText(t, type.name) : m.module}
                  </span>
                  <span className="dim">
                    {t("hw.startAddress", { address: m.startAddress })}
                  </span>
                </button>
              </div>
            );
          })}
        </div>
      )}
      {(view === "compare" || view === "actual") && (
        <div
          className="rack"
          role="list"
          aria-label={
            view === "actual" ? t("hw.detectedStation") : t("hw.rack")
          }
        >
          <div
            className={`module cpu${selection === null ? " selected" : ""}`}
            role="listitem"
            onClick={() => select(null)}
          >
            <button
              type="button"
              className="module-head"
              aria-pressed={selection === null}
            >
              <span className="slot">{t("hw.slot", { slot: 0 })}</span>
              <strong>CPU</strong>
              <span className="dim">
                {data.cpuProduct ?? moduleText(t, data.cpuName)}
              </span>
            </button>
            <div className="module-channels">
              {data.cpuLeds.map((l) => (
                <div key={l.name} className="channel">
                  <span className={LED_CLASS[l.state] ?? "dot"} /> {l.name}
                </div>
              ))}
            </div>
          </div>
          {data.modules.map((m) => {
            const fault = m.status !== "OK";
            const isSelected =
              selected?.position.slot === m.position.slot &&
              selected.position.rack === m.position.rack;
            return (
              <div
                key={`${m.position.rack}-${m.position.slot}`}
                role="listitem"
                className={`module${isSelected ? " selected" : ""}${m.status === "MISSING" ? " missing" : ""}${fault ? " fault" : ""}`}
                onClick={(e) => {
                  e.stopPropagation();
                  select({
                    kind: "module",
                    rack: m.position.rack,
                    slot: m.position.slot,
                  });
                }}
              >
                <button
                  type="button"
                  className="module-head"
                  aria-pressed={isSelected}
                >
                  <span className="row slot">
                    {t("hw.slot", { slot: m.position.slot })}
                    <span className="spacer" />
                    <span
                      className={LED_CLASS[m.statusLed] ?? "dot"}
                      title={t("hw.statusTitle", {
                        status: moduleStatus(t, m.status),
                      })}
                    />
                  </span>
                  <strong style={{ fontSize: 11 }}>
                    {view === "actual"
                      ? m.status === "MISSING"
                        ? t("hw.emptySlot")
                        : (m.installed ?? "").toUpperCase()
                      : (m.configured ?? m.installed ?? "").toUpperCase()}
                  </strong>
                  {view === "compare" && m.moduleId && (
                    <span className="dim">{m.moduleId}</span>
                  )}
                  {m.product && <span className="dim">{m.product}</span>}
                  {view === "compare" &&
                    data.findings.some(
                      (f) =>
                        f.slot?.rack === m.position.rack &&
                        f.slot.slot === m.position.slot,
                    ) && <Badge kind="warning">{t("hw.finding")}</Badge>}
                </button>
                <div className="module-channels">
                  {m.channels.map((c) => (
                    <div key={c.channel} className="channel" title={c.address}>
                      <span className={c.led ? "dot on" : "dot"} />
                      <span className="dim">{c.channel}</span>
                      <span className="tag">{c.tag ?? ""}</span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      )}
      {view === "compare" && (
        <div style={{ padding: "0 16px 16px" }}>
          <FindingsPanel findings={data.findings} />
        </div>
      )}
      {selected && view !== "signals" ? (
        <div className="panel" style={{ margin: "0 16px 16px" }}>
          <div className="panel-head">
            <strong>
              {t("hw.slotTitle", {
                slot: selected.position.slot,
                name: selected.configuredName
                  ? moduleText(t, selected.configuredName)
                  : t("hw.notConfigured"),
              })}
            </strong>
            <Badge kind={STATUS_KIND[selected.status] ?? "normal"}>
              {moduleStatus(t, selected.status)}
            </Badge>
            <span className="spacer" />
          </div>
          <div className="panel-body" style={{ padding: 0 }}>
            <Tabs<Section>
              label={t("hw.moduleTabs")}
              active={section}
              onSelect={setSection}
              tabs={[
                { id: "overview", label: t("hw.section.overview") },
                { id: "configuration", label: t("hw.section.configuration") },
                { id: "channels", label: t("hw.section.channels") },
                { id: "power", label: t("hw.section.power") },
                { id: "diagnostics", label: t("hw.section.diagnostics") },
              ]}
            />
            {section === "overview" && (
              <dl className="kv" style={{ padding: 12 }}>
                <dt>{t("hw.moduleId")}</dt>
                <dd className="mono">
                  {selected.moduleId ?? t("hw.notConfigured")}
                </dd>
                <dt>{t("hw.configured")}</dt>
                <dd>
                  {selected.configuredName
                    ? moduleText(t, selected.configuredName)
                    : "—"}
                </dd>
                <dt>{t("hw.detected")}</dt>
                <dd>
                  {selected.status === "MISSING"
                    ? t("hw.nothingAnswers")
                    : selected.status === "NO_MODULE_POWER"
                      ? t("hw.notAnswering")
                      : selected.installedName
                        ? moduleText(t, selected.installedName)
                        : "—"}
                </dd>
                <dt>{t("hw.status")}</dt>
                <dd>{moduleStatus(t, selected.status).toLowerCase()}</dd>
              </dl>
            )}
            {section === "configuration" &&
              (advanced ? (
                <div className="col" style={{ padding: 12, gap: 8 }}>
                  {selected.configured && (
                    <label className="row muted" style={{ fontSize: 12 }}>
                      {t("hw.configuredType")}
                      <select
                        className="input"
                        value={configuredOf(station, selected)?.module ?? ""}
                        onChange={(e) => editModule(selected, e.target.value)}
                      >
                        {data.catalog.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.product ? `${m.product} — ` : ""}
                            {moduleText(t, m.name)}
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                  {selected.moduleId && (
                    <div className="row" style={{ gap: 8 }}>
                      <label className="row muted" style={{ fontSize: 12 }}>
                        {t("hw.moveTo")}
                        <select
                          className="input"
                          value=""
                          onChange={(e) =>
                            edit(
                              moveModule(
                                station,
                                selected.moduleId ?? "",
                                Number(e.target.value),
                              ),
                            )
                          }
                        >
                          <option value="" disabled>
                            {t("hw.choose")}
                          </option>
                          {freeSlots(station).map((slot) => (
                            <option key={slot} value={slot}>
                              {slot}
                            </option>
                          ))}
                        </select>
                      </label>
                      <button
                        className="btn btn-sm"
                        onClick={() =>
                          edit(removeModule(station, selected.moduleId ?? ""))
                        }
                      >
                        {t("hw.remove")}
                      </button>
                    </div>
                  )}
                  <p className="muted" style={{ margin: 0 }}>
                    {t("hw.savedHelp")}
                  </p>
                </div>
              ) : (
                <p className="muted" style={{ padding: 12, margin: 0 }}>
                  {t("hw.advancedEdit")}
                </p>
              ))}
            {section === "power" && <PowerSection module={selected} />}
            {section === "diagnostics" && (
              <ModuleDiagnostics module={selected} data={data} />
            )}
            {section === "channels" && (
              <table className="table">
                <thead>
                  <tr>
                    <th>{t("hw.ch")}</th>
                    <th>{t("hw.led")}</th>
                    <th>{t("hw.mode")}</th>
                    <th>{t("hw.tag")}</th>
                    <th>{t("hw.address")}</th>
                    <th>{t("hw.value")}</th>
                    <th>{t("hw.quality")}</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {selected.channels.map((c) => {
                    const config = configuredOf(
                      station,
                      selected,
                    )?.channels.find((x) => x.channel === c.channel);
                    const configured = config?.mode ?? c.mode;
                    const current =
                      configured === "current4To20mA" ||
                      configured === "current0To20mA";
                    return (
                      <tr key={c.channel}>
                        <td>{c.channel}</td>
                        <td>
                          <span className={c.led ? "dot on" : "dot"} />
                        </td>
                        <td>
                          {advanced && c.mode !== "digital" ? (
                            <select
                              className="input"
                              style={{ height: 22 }}
                              value={configured}
                              onChange={(e) =>
                                editMode(
                                  selected,
                                  c.channel,
                                  e.target.value as ChannelConfigDto["mode"],
                                )
                              }
                            >
                              {MODES.map((m) => (
                                <option key={m} value={m}>
                                  {modeLabel(t, m)}
                                </option>
                              ))}
                            </select>
                          ) : (
                            modeLabel(t, c.mode)
                          )}
                          {advanced && current && config && (
                            <label
                              className="row"
                              style={{ gap: 4, fontSize: 11 }}
                              title={t("hw.loopHelp")}
                            >
                              <input
                                type="checkbox"
                                checked={config.loopSupply}
                                onChange={(e) =>
                                  editLoopSupply(
                                    selected,
                                    c.channel,
                                    e.target.checked,
                                  )
                                }
                              />
                              {t("hw.loop")}
                            </label>
                          )}
                        </td>
                        <td>{c.tag ?? <span className="dim">—</span>}</td>
                        <td className="mono">{c.address}</td>
                        <td className={valueClass(c.value)}>
                          {c.output !== null
                            ? c.output
                              ? t("hw.on")
                              : t("hw.off")
                            : formatValue(c.value)}
                        </td>
                        <td>
                          {c.quality && c.quality !== "GOOD" ? (
                            <Badge kind="bad">
                              {qualityLabel(t, c.quality).toUpperCase()}
                            </Badge>
                          ) : c.quality ? (
                            <span className="dim">{t("hw.good")}</span>
                          ) : (
                            ""
                          )}
                        </td>
                        <td>
                          <button
                            className="btn btn-sm btn-ghost"
                            onClick={() =>
                              void api
                                .measure(ws, {
                                  rack: selected.position.rack,
                                  slot: selected.position.slot,
                                  channel: c.channel,
                                })
                                .then(
                                  (dto) =>
                                    setMeasurement({
                                      channel: `R${selected.position.rack}/S${selected.position.slot}/CH${c.channel}`,
                                      address: c.address,
                                      dto,
                                    }),
                                  (e: unknown) => lab.log("error", String(e)),
                                )
                            }
                            title={t("hw.measureHelp")}
                          >
                            {t("hw.measure")}
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
            {section === "channels" && measurement && (
              <div className="notice" style={{ margin: 12 }} role="status">
                {t("hw.multimeter")}{" "}
                <strong className="mono" title={measurement.dto.text}>
                  {measurementText(t, measurement)}
                </strong>
              </div>
            )}
          </div>
        </div>
      ) : (
        <p className="muted" style={{ padding: "0 16px" }}>
          {t("hw.selectHelp")}
          {!advanced && t("hw.selectAdvanced")}
        </p>
      )}
    </div>
  );
}

/** The multimeter reading from the measured value; the backend text stays the original. */
function measurementText(
  t: Translate,
  m: { channel: string; address: string; dto: MeasurementDto },
): string {
  const value =
    m.dto.kind === "NO_MODULE" ? "" : (m.dto.value / 1000).toFixed(2);
  return t(`hw.measure.${m.dto.kind}`, {
    value,
    channel: m.channel,
    address: m.address,
  });
}

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
const MODE_LABEL: Record<string, string> = {
  digital: "Digital",
  current4To20mA: "4–20 mA",
  current0To20mA: "0–20 mA",
  voltage0To10V: "0–10 V",
  disabled: "Disabled",
};

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
  const { ws } = lab;
  const hw = useFrameQuery<HardwareDto>(lab, () =>
    ws ? api.hardware(ws) : Promise.reject(new Error("No workspace")),
  );
  const [measurement, setMeasurement] = useState<MeasurementDto | null>(null);
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
          {data.cpuName}
          {data.cpuProduct ? ` · profile: ${data.cpuProduct}` : ""}
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
          title="Simulated indicators; they do not reproduce a vendor's LED behavior."
        >
          generic PLC Lab indicators
        </span>
      </div>
      {pending && project && (
        <div
          className="notice warning row"
          style={{ margin: "8px 16px" }}
          role="status"
        >
          <span className="grow">
            The project's hardware configuration differs from the one loaded in
            the PLC. Download it to take effect; Compare shows the PLC&apos;s
            configuration.
          </span>
          <button
            className="btn btn-sm btn-primary"
            disabled={lab.busy !== null}
            onClick={() => void lab.buildAndLoad(false, project.station)}
          >
            Download configuration
          </button>
          <button className="btn btn-sm" onClick={() => edit(data.station)}>
            Discard changes
          </button>
        </div>
      )}
      <Tabs<View>
        label="Station views"
        active={view}
        onSelect={setView}
        tabs={[
          {
            id: "compare",
            label: "Compare",
            ...(data.findings.length > 0
              ? {
                  badge: <Badge kind="warning">{data.findings.length}</Badge>,
                }
              : {}),
          },
          { id: "configured", label: "Configured (project)" },
          { id: "actual", label: "Actual (detected)" },
          { id: "signals", label: "Signals" },
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
        <div className="rack" role="list" aria-label="Configured station">
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
                  <span className="slot">Slot {m.position.slot}</span>
                  <strong style={{ fontSize: 11 }}>{m.id}</strong>
                  <span className="dim">{type?.name ?? m.module}</span>
                  <span className="dim">start address {m.startAddress}</span>
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
          aria-label={view === "actual" ? "Detected station" : "Rack"}
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
              <span className="slot">Slot 0</span>
              <strong>CPU</strong>
              <span className="dim">{data.cpuProduct ?? data.cpuName}</span>
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
                    Slot {m.position.slot}
                    <span className="spacer" />
                    <span
                      className={LED_CLASS[m.statusLed] ?? "dot"}
                      title={`Status ${m.status}`}
                    />
                  </span>
                  <strong style={{ fontSize: 11 }}>
                    {view === "actual"
                      ? m.status === "MISSING"
                        ? "EMPTY SLOT"
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
                    ) && <Badge kind="warning">finding</Badge>}
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
              Slot {selected.position.slot}:{" "}
              {selected.configuredName ?? "not configured"}
            </strong>
            <Badge kind={STATUS_KIND[selected.status] ?? "normal"}>
              {selected.status.replaceAll("_", " ")}
            </Badge>
            <span className="spacer" />
          </div>
          <div className="panel-body" style={{ padding: 0 }}>
            <Tabs<Section>
              label="Module"
              active={section}
              onSelect={setSection}
              tabs={[
                { id: "overview", label: "Overview" },
                { id: "configuration", label: "Configuration" },
                { id: "channels", label: "Channels" },
                { id: "power", label: "Power" },
                { id: "diagnostics", label: "Diagnostics" },
              ]}
            />
            {section === "overview" && (
              <dl className="kv" style={{ padding: 12 }}>
                <dt>Module id</dt>
                <dd className="mono">
                  {selected.moduleId ?? "not configured"}
                </dd>
                <dt>Configured</dt>
                <dd>{selected.configuredName ?? "—"}</dd>
                <dt>Detected</dt>
                <dd>
                  {selected.status === "MISSING"
                    ? "nothing answers at this slot"
                    : selected.status === "NO_MODULE_POWER"
                      ? "installed, not answering"
                      : (selected.installedName ?? "—")}
                </dd>
                <dt>Status</dt>
                <dd>{selected.status.replaceAll("_", " ").toLowerCase()}</dd>
              </dl>
            )}
            {section === "configuration" &&
              (advanced ? (
                <div className="col" style={{ padding: 12, gap: 8 }}>
                  {selected.configured && (
                    <label className="row muted" style={{ fontSize: 12 }}>
                      Configured type
                      <select
                        className="input"
                        value={configuredOf(station, selected)?.module ?? ""}
                        onChange={(e) => editModule(selected, e.target.value)}
                      >
                        {data.catalog.map((t) => (
                          <option key={t.id} value={t.id}>
                            {t.product ? `${t.product} — ` : ""}
                            {t.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                  {selected.moduleId && (
                    <div className="row" style={{ gap: 8 }}>
                      <label className="row muted" style={{ fontSize: 12 }}>
                        Move to slot
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
                            choose
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
                        Remove from configuration
                      </button>
                    </div>
                  )}
                  <p className="muted" style={{ margin: 0 }}>
                    Changes are saved in the project configuration; download it
                    to the PLC to take effect. The installed hardware does not
                    change.
                  </p>
                </div>
              ) : (
                <p className="muted" style={{ padding: 12, margin: 0 }}>
                  Switch to Advanced to change the module type, move or remove
                  the module, or add modules.
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
                    <th>Ch</th>
                    <th>LED</th>
                    <th>Mode</th>
                    <th>Tag</th>
                    <th>Address</th>
                    <th>Value</th>
                    <th>Quality</th>
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
                                  {MODE_LABEL[m]}
                                </option>
                              ))}
                            </select>
                          ) : (
                            (MODE_LABEL[c.mode] ?? c.mode)
                          )}
                          {advanced && current && config && (
                            <label
                              className="row"
                              style={{ gap: 4, fontSize: 11 }}
                              title="On: the input supplies the loop for a 2-wire transmitter. Off: passive input for a separately powered 4-wire transmitter. Follow the module manual: terminal designs differ between products."
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
                              Loop supply (2-wire)
                            </label>
                          )}
                        </td>
                        <td>{c.tag ?? <span className="dim">—</span>}</td>
                        <td className="mono">{c.address}</td>
                        <td className={valueClass(c.value)}>
                          {c.output !== null
                            ? c.output
                              ? "ON"
                              : "OFF"
                            : formatValue(c.value)}
                        </td>
                        <td>
                          {c.quality && c.quality !== "GOOD" ? (
                            <Badge kind="bad">
                              {c.quality.replaceAll("_", " ")}
                            </Badge>
                          ) : c.quality ? (
                            <span className="dim">good</span>
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
                                .then(setMeasurement, (e: unknown) =>
                                  lab.log("error", String(e)),
                                )
                            }
                            title="Measure at the terminal with a multimeter"
                          >
                            Measure
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
                Multimeter: <strong className="mono">{measurement.text}</strong>
              </div>
            )}
          </div>
        </div>
      ) : (
        <p className="muted" style={{ padding: "0 16px" }}>
          Select a module to see its channels, live values and diagnostics. Use
          “Measure” to check a terminal like a technician with a multimeter.
          {!advanced && " Switch to Advanced to edit the configuration."}
        </p>
      )}
    </div>
  );
}

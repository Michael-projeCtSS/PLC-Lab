/**
 * Hardware view: configured station versus installed modules, live channel values,
 * LEDs, qualities, addresses and a multimeter. Advanced mode edits a draft of the
 * station configuration that is downloaded together with the program.
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
import { Badge, ErrorNotice } from "../ui";
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
  UNCONFIGURED: "warning",
};
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
  const [draft, setDraft] = useState<StationDto | null>(null);
  const [measurement, setMeasurement] = useState<MeasurementDto | null>(null);
  useEvidence(lab, "hardware view");
  const data = hw.data;
  if (!data || !ws) return <ErrorNotice error={hw.error} />;
  const station = draft ?? data.station;
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
    const module = next.modules.find(
      (x) =>
        x.position.rack === m.position.rack &&
        x.position.slot === m.position.slot,
    );
    if (!module) return;
    const existing = module.channels.find((c) => c.channel === channel);
    if (existing) existing.mode = mode;
    else
      module.channels.push({
        channel,
        mode,
        diagnostics: true,
        stopBehavior: "off",
      });
    setDraft(next);
  };
  const editModule = (m: ModuleViewDto, type: string) => {
    const next: StationDto = structuredClone(station);
    const module = next.modules.find(
      (x) =>
        x.position.rack === m.position.rack &&
        x.position.slot === m.position.slot,
    );
    if (!module) return;
    module.module = type;
    module.channels = [];
    setDraft(next);
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
      </div>
      {draft && (
        <div className="notice warning row" style={{ margin: "8px 16px" }}>
          <span className="grow">
            The hardware configuration was changed. Download it to the PLC to
            take effect.
          </span>
          <button
            className="btn btn-sm btn-primary"
            disabled={lab.busy !== null}
            onClick={() =>
              void lab
                .buildAndLoad(false, draft)
                .then((r) => r?.outcome === "LOADED" && setDraft(null))
            }
          >
            Download configuration
          </button>
          <button className="btn btn-sm" onClick={() => setDraft(null)}>
            Discard
          </button>
        </div>
      )}
      <div className="rack" role="list" aria-label="Rack">
        <div
          className={`module cpu${selection === null ? " selected" : ""}`}
          role="listitem"
          onClick={() => select(null)}
        >
          <div className="module-head">
            <div className="slot">Slot 0</div>
            <strong>CPU</strong>
            <div className="dim">{data.cpuProduct ?? data.cpuName}</div>
          </div>
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
              <div className="module-head">
                <div className="row slot">
                  Slot {m.position.slot}
                  <span className="spacer" />
                  <span
                    className={LED_CLASS[m.statusLed] ?? "dot"}
                    title={`Status ${m.status}`}
                  />
                </div>
                <strong style={{ fontSize: 11 }}>
                  {(m.configured ?? m.installed ?? "").toUpperCase()}
                </strong>
                {m.product && <div className="dim">{m.product}</div>}
              </div>
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
      {selected ? (
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
            {advanced && selected.configured && (
              <label className="row muted" style={{ fontSize: 12 }}>
                Configured type
                <select
                  className="input"
                  value={
                    station.modules.find(
                      (x) =>
                        x.position.slot === selected.position.slot &&
                        x.position.rack === selected.position.rack,
                    )?.module ?? ""
                  }
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
          </div>
          <div className="panel-body" style={{ padding: 0 }}>
            <dl className="kv" style={{ padding: 12 }}>
              <dt>Configured</dt>
              <dd>{selected.configuredName ?? "—"}</dd>
              <dt>Installed</dt>
              <dd>{selected.installedName ?? "nothing plugged in"}</dd>
            </dl>
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
                  const configured =
                    station.modules
                      .find(
                        (x) =>
                          x.position.slot === selected.position.slot &&
                          x.position.rack === selected.position.rack,
                      )
                      ?.channels.find((x) => x.channel === c.channel)?.mode ??
                    c.mode;
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
            {measurement && (
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

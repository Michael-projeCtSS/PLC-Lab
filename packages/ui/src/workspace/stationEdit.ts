/** Pure edits of a draft station configuration. Moving a module follows the backend rule
 * (`StationConfig::move_module`): the module keeps its id and its I/O mappings move. */
import type { ModuleTypeDto, StationDto } from "@plc-lab/ipc";

const CLASS_PREFIX: Record<string, string> = {
  DI: "di",
  DO: "do",
  AI: "ai",
  AO: "ao",
  PS: "ps",
  CPU: "cpu",
};

/** Highest slot offered for new modules. */
export const MAX_SLOT = 16;

/** A unique stable id for a new module: `<class>-r<rack>s<slot>`, suffixed if taken. */
export function newModuleId(
  station: StationDto,
  type: ModuleTypeDto,
  rack: number,
  slot: number,
): string {
  const base = `${CLASS_PREFIX[type.class] ?? "module"}-r${rack}s${slot}`;
  const taken = new Set(station.modules.map((m) => m.id));
  let id = base;
  for (let n = 2; taken.has(id); n++) id = `${base}-${n}`;
  return id;
}

export function addModule(
  station: StationDto,
  type: ModuleTypeDto,
  slot: number,
  catalog: readonly ModuleTypeDto[],
): StationDto {
  const next = structuredClone(station);
  next.modules.push({
    id: newModuleId(station, type, 0, slot),
    position: { rack: 0, slot },
    module: type.id,
    startAddress: nextStartAddress(station, type, catalog),
    channels: [],
  });
  next.modules.sort((a, b) => a.position.slot - b.position.slot);
  return next;
}

/** The image a module type occupies in a byte-oriented process image (bit per digital
 * channel, word per analog channel); `null` for modules without I/O. */
function image(type: ModuleTypeDto): { input: boolean; bytes: number } | null {
  switch (type.class) {
    case "DI":
    case "DO":
      return {
        input: type.class === "DI",
        bytes: Math.ceil(type.channels / 8),
      };
    case "AI":
    case "AO":
      return { input: type.class === "AI", bytes: 2 * type.channels };
    default:
      return null;
  }
}

/** First fit in the module's image (inputs or outputs), from byte 0 for digital and
 * byte 64 for analog modules, so new modules never overlap configured ones. The
 * backend still reports any overlap as a finding. */
function nextStartAddress(
  station: StationDto,
  type: ModuleTypeDto,
  catalog: readonly ModuleTypeDto[],
): number {
  const own = image(type);
  const analog = type.class === "AI" || type.class === "AO";
  const size = own?.bytes ?? 0;
  const used = station.modules
    .flatMap((m) => {
      const t = catalog.find((c) => c.id === m.module);
      const img = t ? image(t) : null;
      return img && img.input === own?.input && img.bytes > 0
        ? [{ start: m.startAddress, end: m.startAddress + img.bytes }]
        : [];
    })
    .sort((a, b) => a.start - b.start);
  let start = analog ? 64 : 0;
  for (const r of used)
    if (start < r.end && r.start < start + size) start = r.end;
  return start;
}

/** Remove a module and the I/O mappings of its channels. */
export function removeModule(station: StationDto, id: string): StationDto {
  const next = structuredClone(station);
  const module = next.modules.find((m) => m.id === id);
  if (!module) return next;
  next.modules = next.modules.filter((m) => m.id !== id);
  next.ioMap = next.ioMap.filter(
    (map) =>
      map.channel.rack !== module.position.rack ||
      map.channel.slot !== module.position.slot,
  );
  return next;
}

/** Move a module to a free slot; its I/O mappings move with it (same rule as the backend). */
export function moveModule(
  station: StationDto,
  id: string,
  slot: number,
): StationDto {
  const next = structuredClone(station);
  const module = next.modules.find((m) => m.id === id);
  if (!module || next.modules.some((m) => m.position.slot === slot))
    return next;
  const from = module.position;
  module.position = { rack: from.rack, slot };
  for (const map of next.ioMap)
    if (map.channel.rack === from.rack && map.channel.slot === from.slot)
      map.channel = { ...map.channel, slot };
  next.modules.sort((a, b) => a.position.slot - b.position.slot);
  return next;
}

/** Whether two station configurations are the same, independent of key order. */
export function sameStation(a: StationDto, b: StationDto): boolean {
  const canonical = (v: unknown): unknown =>
    Array.isArray(v)
      ? v.map(canonical)
      : v && typeof v === "object"
        ? Object.fromEntries(
            Object.entries(v as Record<string, unknown>)
              .sort(([x], [y]) => x.localeCompare(y))
              .map(([k, x]) => [k, canonical(x)]),
          )
        : v;
  return JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));
}

export function freeSlots(station: StationDto): number[] {
  const used = new Set(station.modules.map((m) => m.position.slot));
  return Array.from({ length: MAX_SLOT }, (_, i) => i + 1).filter(
    (s) => !used.has(s),
  );
}

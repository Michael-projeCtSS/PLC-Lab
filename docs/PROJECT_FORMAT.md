# Project format (`.plclab`)

A project is one JSON file `<id>.plclab` (`project-model`, ADR-0023).

```json
{
  "format": "plclab-project",
  "formatVersion": 2,
  "id": "p-1793140000-0",
  "name": "Tank test bench",
  "createdAt": "2026-10-27T10:00:00Z",
  "updatedAt": "2026-10-27T10:05:00Z",
  "kind": { "type": "freeLab" },
  "platform": "generic.iec",
  "scene": "scene.tank-filling",
  "station": {
    "name": "PLC_1",
    "cpu": "cpu.compact",
    "modules": [],
    "ioMap": []
  },
  "programs": [
    {
      "id": "main",
      "name": "Main",
      "language": "ST",
      "source": "PROGRAM Main ..."
    }
  ],
  "watch": ["plc.LevelRaw", "process.tank.levelPermille"],
  "simulation": { "cycleMs": 100, "speedPercent": 100 },
  "exerciseState": null
}
```

`kind` is `freeLab` or `exercise` (with exercise id and revision). `exerciseState` holds
attempts, best score, pass flag and the troubleshooting investigation (diagnoses,
evidence, repairs).

## What is stored

User work and references: program source, station configuration, scene and platform
references, watch list, simulation settings, exercise progress. Engine state (runtime
memory, plant state, traces) is **not** serialized; opening a project starts a fresh,
deterministic session.

## Versioning and migration

`formatVersion` 2 is current: configured modules carry a stable `id` (ADR-0029).
Version 1 files are migrated on load by giving every module its default id
`<class>-r<rack>s<slot>`; version 0 files (single `source` field) are migrated through
version 1. Progress and recent-project files have their own format versions. Newer versions are rejected with a message asking to update PLC Lab. Unknown fields
are errors. Validation limits: name 1..120 characters, 1..32 Structured Text programs of
at most 256 KiB, at most 64 watch entries, cycle 10..1000 ms, speed 10..1000 %, a valid
station, files up to 4 MiB.

## Storage

Projects live in `<application data>/projects/` (`%APPDATA%\dev.plclab.desktop` on Windows,
or `PLC_LAB_DATA_DIR`). Writes are atomic (temporary file + rename). Unreadable files are
listed with a reason and never break the project list.

Progress (`progress.json`) is never silently replaced: a file that cannot be read,
parsed or that has another format version is renamed to
`progress.json.unreadable-<seconds>` next to it, progress starts empty and Home and
Progress show a notice naming the kept file. If even the rename fails, progress is not
saved for that session so the damaged copy is not overwritten.

Only one PLC Lab instance runs per user: a second launch brings the running window to the
front and exits (`tauri-plugin-single-instance`), so two application states can never
overwrite each other's progress or projects.

## Use cases

New project (free lab: scene + platform; exercise: created on first start), Open, Save
(automatic for program edits and watch changes, debounced), **Save As** (copy with new
identity), Import (`.plclab` text read through the browser file picker, migrated and given
a new identity), Export (download of the `.plclab` text), Delete. Progress is a separate
`progress.json` in the same folder.

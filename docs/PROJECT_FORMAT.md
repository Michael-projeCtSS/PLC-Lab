# Project format (`.plclab`)

A project is one JSON file `<id>.plclab` (`project-model`, ADR-0023).

```json
{
  "format": "plclab-project",
  "formatVersion": 5,
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
  "watchTables": [
    {
      "id": "watch-1",
      "name": "Filling",
      "rows": [
        { "signal": "plc.LevelRaw", "format": "hex" },
        { "signal": "process.tank.levelPermille", "format": "auto" }
      ]
    }
  ],
  "simulation": { "cycleMs": 100, "speedPercent": 100 },
  "exerciseState": null
}
```

`kind` is `freeLab` or `exercise` (with exercise id and revision). `exerciseState` holds
attempts, best score, pass flag and the troubleshooting investigation (diagnoses,
evidence, repairs).

## What is stored

User work and references: program source, station configuration, scene and platform
references, watch tables, simulation settings, exercise progress. Engine state (runtime
memory, plant state, traces) is **not** serialized; opening a project starts a fresh,
deterministic session.

## Versioning and migration

`formatVersion` 5 is current: up to 16 named `watchTables` of up to 64 rows, each a
canonical signal path and an integer display format (ADR-0032); a format 4 `watch`
list becomes the table "Watch 1". Since format 4 `station` is the configuration the
project expects
(Advanced edits are saved there); the optional `downloadedStation` is the configuration
last downloaded into the PLC, which a reopened project starts the PLC with (ADR-0031).
Format 3 files have no `downloadedStation` and open with the PLC holding `station`.
Since format 3 (ADR-0029) configured modules carry a stable `id`, and watch entries name
I/O channels by it (`io.<moduleId>.ch<n>`), never by position.
Version 2 watch entries `io.R<rack>/S<slot>/CH<channel>` are migrated through the
project's own station; an entry whose position has no configured channel makes the file
invalid with that entry named, instead of being guessed. Version 1 files first get every
module's default id `<class>-r<rack>s<slot>`; version 0 files (single `source` field) are
migrated through version 1. Progress and recent-project files have their own format versions. Newer versions are rejected with a message asking to update PLC Lab. Unknown fields
are errors. Validation limits: name 1..120 characters, 1..32 Structured Text programs of
at most 256 KiB, at most 16 watch tables of 64 rows, cycle 10..1000 ms, speed 10..1000 %, a valid
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

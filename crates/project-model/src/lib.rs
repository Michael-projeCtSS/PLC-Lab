//! Versioned local PLC Lab project files (`.plclab`) and learner progress.
//!
//! A project stores configuration and documents only: platform, scene reference, hardware
//! configuration, program sources, watch lists, simulation settings and exercise state.
//! Runtime internals are never persisted. Older formats migrate forward explicitly;
//! newer formats are rejected rather than guessed. Writes are atomic (temp file + rename).

use lab_model::{Identifier, hardware::StationConfig, validate_station};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Project file format. 2: configured modules carry a stable `id`. 3: watch entries name
/// I/O channels by module id (`io.<moduleId>.ch<n>`) instead of position. 4: the station
/// configuration last downloaded into the PLC is stored apart from the expected one.
pub const FORMAT_VERSION: u32 = 4;
/// Progress file format, versioned independently of projects.
pub const PROGRESS_FORMAT_VERSION: u32 = 1;
/// Recent-projects list format, versioned independently of projects.
pub const RECENT_FORMAT_VERSION: u32 = 1;
pub const EXTENSION: &str = "plclab";
pub const MAX_PROJECT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    Parse(String),
    UnsupportedVersion(u32),
    Invalid(String),
    Io(String),
    NotFound(String),
}
impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(m) => write!(f, "The project file cannot be read: {m}"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "The project uses format version {v}, which is newer than this application supports ({FORMAT_VERSION}). Update PLC Lab."
            ),
            Self::Invalid(m) => write!(f, "The project is invalid: {m}"),
            Self::Io(m) => write!(f, "The project could not be stored: {m}"),
            Self::NotFound(id) => write!(f, "Project '{id}' was not found."),
        }
    }
}
impl std::error::Error for ProjectError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectFormat {
    #[serde(rename = "plclab-project")]
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ProjectKind {
    FreeLab,
    Exercise { exercise: Identifier, revision: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgramDocument {
    pub id: Identifier,
    pub name: String,
    pub language: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimulationSettings {
    pub cycle_ms: u32,
    pub speed_percent: u32,
}
impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            cycle_ms: 100,
            speed_percent: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExerciseState {
    pub attempts: u32,
    pub best_score: u32,
    pub passed: bool,
    #[serde(default)]
    pub diagnoses: Vec<String>,
    #[serde(default)]
    pub repairs: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    pub format: ProjectFormat,
    pub format_version: u32,
    pub id: Identifier,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub kind: ProjectKind,
    pub platform: Identifier,
    pub scene: Identifier,
    /// The station configuration the project expects (edited and saved offline).
    pub station: StationConfig,
    /// The station configuration last downloaded into the PLC. Reopening the project
    /// starts the PLC with it, so a saved but not downloaded edit stays pending; `None`
    /// (no download recorded yet) means the PLC starts with `station`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloaded_station: Option<StationConfig>,
    pub programs: Vec<ProgramDocument>,
    #[serde(default)]
    pub watch: Vec<String>,
    #[serde(default)]
    pub simulation: SimulationSettings,
    #[serde(default)]
    pub exercise_state: Option<ExerciseState>,
}

impl Project {
    pub fn main_program(&self) -> Option<&ProgramDocument> {
        self.programs.first()
    }
    pub fn main_program_mut(&mut self) -> Option<&mut ProgramDocument> {
        self.programs.first_mut()
    }
    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.format_version != FORMAT_VERSION {
            return Err(ProjectError::UnsupportedVersion(self.format_version));
        }
        if self.name.trim().is_empty() || self.name.len() > 120 {
            return Err(ProjectError::Invalid(
                "the project name must have 1 to 120 characters".into(),
            ));
        }
        if self.programs.is_empty() || self.programs.len() > 32 {
            return Err(ProjectError::Invalid(
                "a project needs 1 to 32 program documents".into(),
            ));
        }
        if self
            .programs
            .iter()
            .any(|p| p.language != "ST" || p.source.len() > 256 * 1024)
        {
            return Err(ProjectError::Invalid(
                "program documents must be Structured Text of at most 256 KiB".into(),
            ));
        }
        if self.watch.len() > 64 {
            return Err(ProjectError::Invalid(
                "a watch list holds at most 64 signals".into(),
            ));
        }
        if !(10..=1000).contains(&self.simulation.cycle_ms)
            || !(10..=1000).contains(&self.simulation.speed_percent)
        {
            return Err(ProjectError::Invalid(
                "cycle time must be 10..1000 ms and speed 10..1000 %".into(),
            ));
        }
        validate_station(&self.station).map_err(|e| ProjectError::Invalid(e.to_string()))?;
        if let Some(downloaded) = &self.downloaded_station {
            validate_station(downloaded).map_err(|e| {
                ProjectError::Invalid(format!("the downloaded station configuration: {e}"))
            })?;
        }
        Ok(())
    }
    /// The station configuration the PLC starts with when the project is opened.
    pub fn plc_station(&self) -> &StationConfig {
        self.downloaded_station.as_ref().unwrap_or(&self.station)
    }
}

/// Parse a project file, migrating older formats. Newer formats are rejected.
pub fn parse(text: &str) -> Result<Project, ProjectError> {
    let mut value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| ProjectError::Parse(e.to_string()))?;
    let version = value
        .get("formatVersion")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ProjectError::Parse("formatVersion is missing".into()))?;
    let version = u32::try_from(version).map_err(|_| ProjectError::UnsupportedVersion(u32::MAX))?;
    if version > FORMAT_VERSION {
        return Err(ProjectError::UnsupportedVersion(version));
    }
    if version == 0 {
        value = migrate_v0(value)?;
    }
    if version <= 1 {
        value = migrate_v1(value)?;
    }
    if version <= 2 {
        value = migrate_v2(value)?;
    }
    if version <= 3 {
        value = migrate_v3(value)?;
    }
    let project: Project =
        serde_json::from_value(value).map_err(|e| ProjectError::Parse(e.to_string()))?;
    project.validate()?;
    Ok(project)
}

/// Format 0 (pre-release drafts) stored a single `source` string instead of `programs`.
fn migrate_v0(mut value: serde_json::Value) -> Result<serde_json::Value, ProjectError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| ProjectError::Parse("project root must be an object".into()))?;
    let source = object
        .remove("source")
        .and_then(|s| s.as_str().map(str::to_owned))
        .ok_or_else(|| ProjectError::Parse("format 0 project has no source".into()))?;
    object.insert(
        "programs".into(),
        serde_json::json!([{ "id": "main", "name": "Main", "language": "ST", "source": source }]),
    );
    object.insert("formatVersion".into(), serde_json::json!(1));
    Ok(value)
}

/// Format 1 had no module ids. Reading the station assigns each configured module the
/// deterministic id of its class and position (`di-r0s1`); saving then persists it.
fn migrate_v1(mut value: serde_json::Value) -> Result<serde_json::Value, ProjectError> {
    value
        .as_object_mut()
        .ok_or_else(|| ProjectError::Parse("project root must be an object".into()))?
        .insert("formatVersion".into(), serde_json::json!(2));
    Ok(value)
}

/// Format 2 watch entries named I/O channels by position (`io.R0/S2/CH0`). Format 3 names
/// them by the configured module's stable id, resolved through the project's own station.
/// An entry whose position has no configured channel cannot be mapped without guessing and
/// is refused with the entry named.
fn migrate_v2(mut value: serde_json::Value) -> Result<serde_json::Value, ProjectError> {
    let station: lab_model::hardware::StationConfig = value
        .get("station")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| ProjectError::Parse(e.to_string()))?
        .ok_or_else(|| ProjectError::Parse("the project has no station".into()))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| ProjectError::Parse("project root must be an object".into()))?;
    if let Some(watch) = object
        .get_mut("watch")
        .and_then(serde_json::Value::as_array_mut)
    {
        for entry in watch.iter_mut() {
            let Some(channel) = entry.as_str().and_then(legacy_channel_path) else {
                continue;
            };
            let mapped = station
                .module_at(channel.slot_ref())
                .filter(|m| channel.channel < m.module.channel_count())
                .map(|m| format!("io.{}", m.channel_id(channel.channel)));
            let Some(mapped) = mapped else {
                return Err(ProjectError::Invalid(format!(
                    "watch entry '{}' names I/O channel {channel} by its position, and no configured module has that channel; remove the entry or restore the module",
                    entry.as_str().unwrap_or_default()
                )));
            };
            *entry = serde_json::Value::String(mapped);
        }
    }
    object.insert("formatVersion".into(), serde_json::json!(3));
    Ok(value)
}

/// Format 3 recorded no downloaded configuration: such a project opens with the PLC holding
/// the expected configuration, as before.
fn migrate_v3(mut value: serde_json::Value) -> Result<serde_json::Value, ProjectError> {
    value
        .as_object_mut()
        .ok_or_else(|| ProjectError::Parse("project root must be an object".into()))?
        .insert("formatVersion".into(), serde_json::json!(4));
    Ok(value)
}

/// A format 2 I/O channel path `io.R<rack>/S<slot>/CH<channel>`.
fn legacy_channel_path(path: &str) -> Option<lab_model::hardware::ChannelRef> {
    let rest = path.strip_prefix("io.R")?;
    let (rack, rest) = rest.split_once("/S")?;
    let (slot, channel) = rest.split_once("/CH")?;
    Some(lab_model::hardware::ChannelRef {
        rack: rack.parse().ok()?,
        slot: slot.parse().ok()?,
        channel: channel.parse().ok()?,
    })
}

pub fn to_json(project: &Project) -> Result<String, ProjectError> {
    serde_json::to_string_pretty(project)
        .map(|s| s + "\n")
        .map_err(|e| ProjectError::Io(e.to_string()))
}

/// Write via a temporary file in the same directory and rename into place.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), ProjectError> {
    let dir = path
        .parent()
        .ok_or_else(|| ProjectError::Io("the file has no parent folder".into()))?;
    fs::create_dir_all(dir).map_err(|e| ProjectError::Io(e.to_string()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.tmp"));
    fs::write(&temp, contents).map_err(|e| ProjectError::Io(e.to_string()))?;
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        ProjectError::Io(e.to_string())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub id: Identifier,
    pub name: String,
    pub kind: ProjectKind,
    pub platform: Identifier,
    pub scene: Identifier,
    pub updated_at: String,
}

/// Projects stored as `<id>.plclab` files in one directory.
#[derive(Debug, Clone)]
pub struct ProjectStore {
    dir: PathBuf,
}

impl ProjectStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
    pub fn directory(&self) -> &Path {
        &self.dir
    }
    fn path(&self, id: &Identifier) -> PathBuf {
        self.dir.join(format!("{id}.{EXTENSION}"))
    }
    /// The file that stores project `id` in this store.
    pub fn file_of(&self, id: &Identifier) -> PathBuf {
        self.path(id)
    }
    pub fn save(&self, project: &Project) -> Result<(), ProjectError> {
        project.validate()?;
        write_atomic(&self.path(&project.id), &to_json(project)?)
    }
    pub fn load(&self, id: &Identifier) -> Result<Project, ProjectError> {
        let path = self.path(id);
        let meta = fs::metadata(&path).map_err(|_| ProjectError::NotFound(id.to_string()))?;
        if meta.len() > MAX_PROJECT_BYTES {
            return Err(ProjectError::Invalid(
                "the project file is too large".into(),
            ));
        }
        let text = fs::read_to_string(&path).map_err(|e| ProjectError::Io(e.to_string()))?;
        let project = parse(&text)?;
        if &project.id != id {
            return Err(ProjectError::Invalid(
                "the file name does not match the project identifier".into(),
            ));
        }
        Ok(project)
    }
    pub fn delete(&self, id: &Identifier) -> Result<(), ProjectError> {
        fs::remove_file(self.path(id)).map_err(|_| ProjectError::NotFound(id.to_string()))
    }
    /// All readable projects, newest first. Unreadable files are reported, not fatal.
    pub fn list(&self) -> (Vec<ProjectSummary>, Vec<String>) {
        let mut out = Vec::new();
        let mut problems = Vec::new();
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return (out, problems);
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != EXTENSION) {
                continue;
            }
            let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                continue;
            };
            let Ok(id) = Identifier::new(stem.clone()) else {
                problems.push(format!("{stem}: invalid project file name"));
                continue;
            };
            match self.load(&id) {
                Ok(p) => out.push(ProjectSummary {
                    id: p.id,
                    name: p.name,
                    kind: p.kind,
                    platform: p.platform,
                    scene: p.scene,
                    updated_at: p.updated_at,
                }),
                Err(e) => problems.push(format!("{stem}: {e}")),
            }
        }
        out.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        (out, problems)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonProgress {
    pub completed: bool,
    pub checkpoint_correct: u16,
    pub checkpoint_total: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExerciseProgress {
    pub attempts: u32,
    pub best_score: u32,
    pub max_score: u32,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Progress {
    pub format_version: u32,
    #[serde(default)]
    pub lessons: BTreeMap<String, LessonProgress>,
    #[serde(default)]
    pub exercises: BTreeMap<String, ExerciseProgress>,
}

/// Number of entries kept in [`RecentProjects`].
pub const MAX_RECENT: usize = 8;

/// Projects opened recently, newest first. Trusted application metadata (identifier, name,
/// time) that the root launcher reads to offer the last project; it holds no project
/// content. It is a convenience list: an unreadable file starts a new, empty list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecentProjects {
    pub format_version: u32,
    #[serde(default)]
    pub projects: Vec<RecentProject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecentProject {
    pub id: Identifier,
    pub name: String,
    pub opened_at: String,
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self {
            format_version: RECENT_FORMAT_VERSION,
            projects: Vec::new(),
        }
    }
}

impl RecentProjects {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<RecentProjects>(&t).ok())
            .filter(|r| r.format_version == RECENT_FORMAT_VERSION)
            .map(|mut r| {
                r.projects.truncate(MAX_RECENT);
                r
            })
            .unwrap_or_default()
    }
    /// Put `id` first, replacing an older entry for it, and keep at most [`MAX_RECENT`].
    pub fn record(&mut self, id: &Identifier, name: &str, opened_at: &str) {
        self.projects.retain(|p| &p.id != id);
        self.projects.insert(
            0,
            RecentProject {
                id: id.clone(),
                name: name.to_owned(),
                opened_at: opened_at.to_owned(),
            },
        );
        self.projects.truncate(MAX_RECENT);
    }
    /// Forget projects that no longer exist.
    pub fn forget(&mut self, id: &Identifier) {
        self.projects.retain(|p| &p.id != id);
    }
    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| ProjectError::Io(e.to_string()))?;
        write_atomic(path, &text)
    }
}

/// Result of loading the progress file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressLoad {
    pub progress: Progress,
    /// Set when the file could not be used; says where the original was kept.
    pub notice: Option<String>,
    /// False when an unusable file could not be moved aside: saving would destroy it.
    pub writable: bool,
}

impl Progress {
    fn fresh() -> Self {
        Progress {
            format_version: PROGRESS_FORMAT_VERSION,
            ..Default::default()
        }
    }

    /// Load progress. A missing file starts fresh. A file that cannot be read, parsed or
    /// that has another format version is never overwritten: it is renamed to
    /// `<name>.unreadable-<seconds>` next to it and progress starts fresh with a notice.
    pub fn load(path: &Path) -> ProgressLoad {
        let reason = match fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return ProgressLoad {
                    progress: Self::fresh(),
                    notice: None,
                    writable: true,
                };
            }
            Err(e) => e.to_string(),
            Ok(text) => match serde_json::from_str::<Progress>(&text) {
                Ok(p) if p.format_version == PROGRESS_FORMAT_VERSION => {
                    return ProgressLoad {
                        progress: p,
                        notice: None,
                        writable: true,
                    };
                }
                Ok(p) => format!("format version {} is not supported", p.format_version),
                Err(e) => e.to_string(),
            },
        };
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "progress.json".into());
        let kept = path.with_file_name(format!("{name}.unreadable-{secs}"));
        match fs::rename(path, &kept) {
            Ok(()) => ProgressLoad {
                progress: Self::fresh(),
                notice: Some(format!(
                    "Your saved progress could not be read ({reason}). The file was kept as {} and progress starts empty.",
                    kept.file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                )),
                writable: true,
            },
            Err(e) => ProgressLoad {
                progress: Self::fresh(),
                notice: Some(format!(
                    "Your saved progress could not be read ({reason}) or moved aside ({e}). Progress is not saved in this session so that the file is not overwritten."
                )),
                writable: false,
            },
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| ProjectError::Io(e.to_string()))?;
        write_atomic(path, &text)
    }
    pub fn record_exercise(&mut self, id: &str, score: u32, max_score: u32, passed: bool) {
        let entry = self.exercises.entry(id.to_owned()).or_default();
        entry.attempts += 1;
        entry.best_score = entry.best_score.max(score);
        entry.max_score = max_score;
        entry.passed |= passed;
    }
    pub fn record_lesson(&mut self, id: &str, correct: u16, total: u16) {
        let entry = self.lessons.entry(id.to_owned()).or_default();
        entry.completed = true;
        entry.checkpoint_correct = entry.checkpoint_correct.max(correct);
        entry.checkpoint_total = total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn station() -> serde_json::Value {
        serde_json::json!({"name": "PLC_1", "cpu": "cpu.compact", "modules": [], "ioMap": []})
    }
    fn project_json(version: u32) -> String {
        serde_json::json!({
            "format": "plclab-project", "formatVersion": version, "id": "p-1", "name": "My lab",
            "createdAt": "2026-10-26T10:00:00Z", "updatedAt": "2026-10-26T10:00:00Z",
            "kind": {"type": "freeLab"}, "platform": "generic.iec", "scene": "scene.io-basics",
            "station": station(),
            "programs": [{"id": "main", "name": "Main", "language": "ST", "source": "PROGRAM Main END_PROGRAM"}]
        })
        .to_string()
    }

    #[test]
    fn current_format_round_trips_exactly() {
        let p = parse(&project_json(1)).unwrap();
        let text = to_json(&p).unwrap();
        assert_eq!(parse(&text).unwrap(), p);
        assert_eq!(p.simulation, SimulationSettings::default());
    }

    #[test]
    fn version_one_stations_get_stable_module_ids_that_survive_moves_and_round_trips() {
        let v1 = serde_json::json!({
            "format": "plclab-project", "formatVersion": 1, "id": "p-1", "name": "My lab",
            "createdAt": "x", "updatedAt": "x", "kind": {"type": "freeLab"},
            "platform": "generic.iec", "scene": "scene.io-basics",
            "station": {"name": "PLC_1", "cpu": "cpu.modular", "modules": [
                {"position": {"rack": 0, "slot": 1}, "module": "di8.24vdc.sink", "startAddress": 0},
                {"position": {"rack": 0, "slot": 2}, "module": "ai4.ui", "startAddress": 64,
                 "channels": [{"channel": 0, "mode": "current4To20mA", "loopSupply": true}]}
            ], "ioMap": [{"channel": {"rack": 0, "slot": 2, "channel": 0}, "tag": "LevelRaw"}]},
            "programs": [{"id": "main", "name": "Main", "language": "ST", "source": "PROGRAM Main END_PROGRAM"}]
        })
        .to_string();
        let mut p = parse(&v1).unwrap();
        assert_eq!(p.format_version, FORMAT_VERSION);
        let ids: Vec<&str> = p.station.modules.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["di-r0s1", "ai-r0s2"]);
        // Moving a module keeps its identity and moves its I/O mappings.
        let ai = lab_model::hardware::SlotRef { rack: 0, slot: 5 };
        p.station.move_module("ai-r0s2", ai).unwrap();
        assert_eq!(p.station.module_by_id("ai-r0s2").unwrap().position, ai);
        assert_eq!(p.station.io_map[0].channel.slot, 5);
        assert_eq!(
            p.station.channel_id(p.station.io_map[0].channel).as_deref(),
            Some("ai-r0s2.ch0")
        );
        assert!(
            p.station.move_module("di-r0s1", ai).is_err(),
            "occupied slot"
        );
        let text = to_json(&p).unwrap();
        assert!(text.contains("\"id\": \"ai-r0s2\""));
        let back = parse(&text).unwrap();
        assert_eq!(
            back, p,
            "ids, slots, channel configuration and mappings round-trip"
        );
    }

    #[test]
    fn version_two_watch_entries_follow_module_ids_or_are_refused() {
        let v2 = |watch: &[&str]| {
            serde_json::json!({
                "format": "plclab-project", "formatVersion": 2, "id": "p-2", "name": "My lab",
                "createdAt": "x", "updatedAt": "x", "kind": {"type": "freeLab"},
                "platform": "generic.iec", "scene": "scene.io-basics",
                "station": {"name": "PLC_1", "cpu": "cpu.modular", "modules": [
                    {"id": "di-r0s1", "position": {"rack": 0, "slot": 1}, "module": "di8.24vdc.sink", "startAddress": 0},
                    {"id": "ai-r0s2", "position": {"rack": 0, "slot": 5}, "module": "ai4.ui", "startAddress": 64}
                ], "ioMap": []},
                "programs": [{"id": "main", "name": "Main", "language": "ST", "source": "PROGRAM Main END_PROGRAM"}],
                "watch": watch
            })
            .to_string()
        };
        // The moved analog module (id ai-r0s2, now in slot 5) keeps its identity.
        let legacy = [
            "plc.LevelRaw",
            "io.R0/S1/CH3",
            "io.R0/S5/CH0",
            "process.tank.levelMl",
        ];
        let mapped = [
            "plc.LevelRaw",
            "io.di-r0s1.ch3",
            "io.ai-r0s2.ch0",
            "process.tank.levelMl",
        ];
        let p = parse(&v2(&legacy)).unwrap();
        assert_eq!(p.format_version, FORMAT_VERSION);
        assert_eq!(p.watch, mapped);
        let back = parse(&to_json(&p).unwrap()).unwrap();
        assert_eq!(back, p, "the current format round-trips");
        // No module at that position, or no such channel: refused, never guessed.
        for stale in ["io.R0/S9/CH0", "io.R0/S1/CH8"] {
            let refused = parse(&v2(&[stale])).unwrap_err();
            assert!(
                matches!(&refused, ProjectError::Invalid(m) if m.contains(stale)),
                "{refused}"
            );
        }
    }

    #[test]
    fn the_downloaded_station_is_kept_apart_from_the_expected_one() {
        // Format 3 had none: the PLC starts with the expected configuration.
        let mut p = parse(&project_json(3)).unwrap();
        assert_eq!(p.downloaded_station, None);
        assert_eq!(p.plc_station(), &p.station);
        assert!(!to_json(&p).unwrap().contains("downloadedStation"));
        // A saved edit after a download: the PLC keeps what was downloaded.
        let downloaded = p.station.clone();
        p.downloaded_station = Some(downloaded.clone());
        p.station.name = "PLC_2".into();
        let back = parse(&to_json(&p).unwrap()).unwrap();
        assert_eq!(back, p, "both configurations round-trip");
        assert_eq!(back.plc_station(), &downloaded);
        assert_eq!(back.station.name, "PLC_2");
        // An invalid downloaded configuration is refused like the expected one.
        let text = to_json(&p).unwrap();
        let mut broken: serde_json::Value = serde_json::from_str(&text).unwrap();
        broken["downloadedStation"]["cpu"] = serde_json::json!("di8.24vdc.sink");
        assert!(parse(&broken.to_string()).is_err());
    }

    #[test]
    fn version_zero_migrates_and_future_versions_are_rejected() {
        let v0 = serde_json::json!({
            "format": "plclab-project", "formatVersion": 0, "id": "p-0", "name": "Old",
            "createdAt": "x", "updatedAt": "x", "kind": {"type": "freeLab"}, "platform": "generic.iec",
            "scene": "scene.io-basics", "station": station(), "source": "PROGRAM Old END_PROGRAM"
        })
        .to_string();
        let migrated = parse(&v0).unwrap();
        assert_eq!(migrated.format_version, FORMAT_VERSION);
        assert_eq!(
            migrated.main_program().unwrap().source,
            "PROGRAM Old END_PROGRAM"
        );
        assert_eq!(
            parse(&project_json(5)),
            Err(ProjectError::UnsupportedVersion(5))
        );
        assert!(parse(&project_json(1).replace("\"name\":\"My lab\"", "\"name\":\"\"")).is_err());
        assert!(parse("{not json").is_err());
        assert!(
            parse(&project_json(1).replace("\"kind\"", "\"extra\": 1, \"kind\"")).is_err(),
            "unknown fields are rejected"
        );
    }

    #[test]
    fn store_saves_atomically_lists_and_reports_bad_files() {
        let dir = std::env::temp_dir().join(format!("plclab-store-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = ProjectStore::new(dir.clone());
        let mut p = parse(&project_json(1)).unwrap();
        store.save(&p).unwrap();
        p.name = "Renamed".into();
        p.updated_at = "2026-10-27T00:00:00Z".into();
        store.save(&p).unwrap();
        assert_eq!(store.load(&p.id).unwrap().name, "Renamed");
        fs::write(dir.join("broken.plclab"), "{").unwrap();
        let (list, problems) = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(problems.len(), 1);
        assert!(!dir.join(".p-1.plclab.tmp").exists());
        store.delete(&p.id).unwrap();
        assert!(matches!(store.load(&p.id), Err(ProjectError::NotFound(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    fn progress_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("plclab-progress-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn kept_files(dir: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains(".unreadable-"))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn missing_and_valid_progress_load_without_notice() {
        let dir = progress_dir("valid");
        let path = dir.join("progress.json");
        let fresh = Progress::load(&path);
        assert_eq!(fresh.notice, None);
        assert!(fresh.writable && fresh.progress.lessons.is_empty());
        let mut p = fresh.progress;
        p.record_lesson("lesson.one", 2, 3);
        p.save(&path).unwrap();
        let loaded = Progress::load(&path);
        assert_eq!(loaded.notice, None);
        assert!(loaded.progress.lessons.contains_key("lesson.one"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_progress_is_kept_aside_and_never_overwritten() {
        let dir = progress_dir("corrupt");
        let path = dir.join("progress.json");
        let damaged = "{ \"formatVersion\": 1, \"lessons\": { truncated";
        fs::write(&path, damaged).unwrap();
        let loaded = Progress::load(&path);
        let notice = loaded.notice.expect("a notice explains the reset");
        assert!(notice.contains("progress.json.unreadable-"), "{notice}");
        assert!(loaded.writable && loaded.progress.lessons.is_empty());
        let kept = kept_files(&dir);
        assert_eq!(kept.len(), 1);
        assert_eq!(fs::read_to_string(&kept[0]).unwrap(), damaged);
        assert!(!path.exists());
        // Saving new progress creates a new file and leaves the kept copy untouched.
        let mut p = loaded.progress;
        p.record_lesson("lesson.one", 1, 1);
        p.save(&path).unwrap();
        assert_eq!(fs::read_to_string(&kept[0]).unwrap(), damaged);
        assert!(
            Progress::load(&path)
                .progress
                .lessons
                .contains_key("lesson.one")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_from_a_newer_format_is_kept_aside() {
        let dir = progress_dir("newer");
        let path = dir.join("progress.json");
        let newer = "{\"formatVersion\": 99, \"lessons\": {}, \"exercises\": {}}";
        fs::write(&path, newer).unwrap();
        let loaded = Progress::load(&path);
        assert!(loaded.notice.unwrap().contains("format version 99"));
        assert_eq!(fs::read_to_string(&kept_files(&dir)[0]).unwrap(), newer);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn recent_projects_keep_the_newest_first_without_duplicates() {
        let dir = progress_dir("recent");
        let path = dir.join("recent.json");
        assert!(RecentProjects::load(&path).projects.is_empty());
        let mut recent = RecentProjects::default();
        for n in 0..10 {
            let id = Identifier::new(format!("p-{n}")).unwrap();
            recent.record(&id, &format!("Project {n}"), "2026-10-28T00:00:00Z");
        }
        let again = Identifier::new("p-3").unwrap();
        recent.record(&again, "Project 3", "2026-10-29T00:00:00Z");
        recent.save(&path).unwrap();
        let loaded = RecentProjects::load(&path);
        assert_eq!(loaded.projects.len(), MAX_RECENT);
        assert_eq!(loaded.projects[0].id, again);
        assert_eq!(loaded.projects.iter().filter(|p| p.id == again).count(), 1);
        fs::write(&path, "{ broken").unwrap();
        assert!(RecentProjects::load(&path).projects.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_records_best_scores() {
        let mut progress = Progress {
            format_version: PROGRESS_FORMAT_VERSION,
            ..Default::default()
        };
        progress.record_exercise("ex", 40, 100, false);
        progress.record_exercise("ex", 100, 100, true);
        progress.record_exercise("ex", 10, 100, false);
        let e = &progress.exercises["ex"];
        assert_eq!((e.attempts, e.best_score, e.passed), (3, 100, true));
    }
}

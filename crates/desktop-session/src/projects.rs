//! Project use cases: creation from exercises or free-lab scenes, DTO mapping and
//! starter program generation. Storage and format versioning live in `project-model`.

use crate::{
    convert,
    error::{AppError, AppResult},
    library,
};
use app_contract::*;
use knowledge_core::KnowledgeLibrary;
use lab_model::{
    Identifier,
    scene::{SceneDefinition, SignalDirection, TagType},
};
use project_model::{
    FORMAT_VERSION, ProgramDocument, Project, ProjectFormat, ProjectKind, ProjectStore,
    ProjectSummary, SimulationSettings,
};

/// UTC wall-clock timestamp for project metadata only (never simulation time).
pub fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil-from-days (Howard Hinnant), proleptic Gregorian calendar.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn kind_name(kind: &ProjectKind) -> (&'static str, Option<String>) {
    match kind {
        ProjectKind::FreeLab => ("freeLab", None),
        ProjectKind::Exercise { exercise, .. } => ("exercise", Some(exercise.to_string())),
    }
}

pub fn to_dto(p: &Project) -> AppResult<ProjectDto> {
    let (kind, exercise) = kind_name(&p.kind);
    Ok(ProjectDto {
        id: p.id.to_string(),
        name: p.name.clone(),
        kind: kind.into(),
        exercise,
        platform: p.platform.to_string(),
        scene: p.scene.to_string(),
        station: convert::station_to_dto(&p.station)
            .map_err(|e| AppError::InvalidRequest(e.message))?,
        programs: p
            .programs
            .iter()
            .map(|d| ProgramDocumentDto {
                id: d.id.to_string(),
                name: d.name.clone(),
                language: d.language.clone(),
                source: d.source.clone(),
            })
            .collect(),
        watch: p.watch.clone(),
        simulation: SimulationSettingsDto {
            cycle_ms: p.simulation.cycle_ms,
            speed_percent: p.simulation.speed_percent,
        },
        created_at: p.created_at.clone(),
        updated_at: p.updated_at.clone(),
    })
}

pub fn summary_dto(s: &ProjectSummary) -> ProjectSummaryDto {
    let (kind, exercise) = kind_name(&s.kind);
    ProjectSummaryDto {
        id: s.id.to_string(),
        name: s.name.clone(),
        kind: kind.into(),
        exercise,
        platform: s.platform.to_string(),
        scene: s.scene.to_string(),
        updated_at: s.updated_at.clone(),
    }
}

pub fn identifier(text: &str, what: &'static str) -> AppResult<Identifier> {
    Identifier::new(text).map_err(|_| AppError::NotFound {
        what,
        id: text.to_owned(),
    })
}

/// Apply user-editable fields of a DTO onto the stored project. Identity, kind and (for
/// exercises) scene and platform are owned by the backend and cannot be changed here.
pub fn apply_edit(
    stored: &mut Project,
    dto: &ProjectDto,
    library: &KnowledgeLibrary,
) -> AppResult<()> {
    if dto.id != stored.id.as_str() {
        return Err(AppError::InvalidRequest(
            "The project identifier cannot be changed.".into(),
        ));
    }
    stored.name = dto.name.trim().to_owned();
    stored.station =
        convert::station_from_dto(&dto.station).map_err(|e| AppError::InvalidRequest(e.message))?;
    stored.programs = dto
        .programs
        .iter()
        .map(|d| {
            Ok(ProgramDocument {
                id: identifier(&d.id, "program")?,
                name: d.name.clone(),
                language: d.language.clone(),
                source: d.source.clone(),
            })
        })
        .collect::<AppResult<_>>()?;
    stored.watch = dto.watch.clone();
    stored.simulation = SimulationSettings {
        cycle_ms: dto.simulation.cycle_ms,
        speed_percent: dto.simulation.speed_percent,
    };
    if stored.kind == ProjectKind::FreeLab {
        let scene = identifier(&dto.scene, "scene")?;
        let platform = identifier(&dto.platform, "platform")?;
        if library.scene(scene.as_str()).is_none() {
            return Err(AppError::NotFound {
                what: "scene",
                id: dto.scene.clone(),
            });
        }
        if library.profile(platform.as_str()).is_none() {
            return Err(AppError::NotFound {
                what: "platform",
                id: dto.platform.clone(),
            });
        }
        stored.scene = scene;
        stored.platform = platform;
    }
    stored.updated_at = timestamp();
    stored.validate()?;
    Ok(())
}

fn type_name(t: TagType) -> &'static str {
    match t {
        TagType::Bool => "BOOL",
        TagType::Int => "INT",
        TagType::Real => "REAL",
        TagType::Time => "TIME",
    }
}

/// A compilable Structured Text skeleton declaring the scene's recommended interface.
pub fn starter_program(scene: &SceneDefinition) -> String {
    let mut out = String::from("PROGRAM Main\n");
    for (keyword, direction) in [
        ("VAR_INPUT", SignalDirection::Input),
        ("VAR_OUTPUT", SignalDirection::Output),
    ] {
        let tags: Vec<_> = scene
            .interface
            .iter()
            .filter(|s| s.direction == direction)
            .collect();
        if tags.is_empty() {
            continue;
        }
        out.push_str(keyword);
        out.push('\n');
        for t in tags {
            out.push_str(&format!(
                "    {} : {}; (* {} *)\n",
                t.tag,
                type_name(t.tag_type),
                t.description.replace("*)", "* )")
            ));
        }
        out.push_str("END_VAR\n");
    }
    out.push_str("\n(* Write your logic here. *)\n\nEND_PROGRAM\n");
    out
}

fn unique_id(store: &ProjectStore) -> AppResult<Identifier> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    for n in 0..1000u32 {
        let id = identifier(&format!("p-{secs}-{n}"), "project")?;
        if store.load(&id).is_err()
            && !store
                .directory()
                .join(format!("{id}.{}", project_model::EXTENSION))
                .exists()
        {
            return Ok(id);
        }
    }
    Err(AppError::InvalidRequest(
        "A unique project identifier could not be generated.".into(),
    ))
}

pub fn create(
    store: &ProjectStore,
    library: &KnowledgeLibrary,
    request: &ProjectCreateRequest,
) -> AppResult<Project> {
    let name = request.name.trim();
    let now = timestamp();
    let id = unique_id(store)?;
    let project = if let Some(exercise_id) = &request.exercise {
        let e = library
            .exercise(exercise_id)
            .ok_or_else(|| AppError::NotFound {
                what: "exercise",
                id: exercise_id.clone(),
            })?;
        let scene = library
            .scene(e.scene.as_str())
            .ok_or_else(|| AppError::NotFound {
                what: "scene",
                id: e.scene.to_string(),
            })?;
        let platform = library::platform_of(e);
        if library.profile(&platform).is_none() {
            return Err(AppError::NotFound {
                what: "platform",
                id: platform,
            });
        }
        Project {
            format: ProjectFormat::Project,
            format_version: FORMAT_VERSION,
            id,
            name: if name.is_empty() {
                e.title.clone()
            } else {
                name.to_owned()
            },
            created_at: now.clone(),
            updated_at: now,
            kind: ProjectKind::Exercise {
                exercise: e.id.clone(),
                revision: e.revision,
            },
            platform: identifier(&platform, "platform")?,
            scene: e.scene.clone(),
            station: e
                .starter_station
                .clone()
                .unwrap_or_else(|| scene.reference_station.clone()),
            programs: vec![ProgramDocument {
                id: identifier("main", "program")?,
                name: "Main".into(),
                language: "ST".into(),
                source: e.starter_program.clone(),
            }],
            watch: Vec::new(),
            simulation: SimulationSettings::default(),
            exercise_state: None,
        }
    } else {
        let scene_id = request
            .scene
            .as_deref()
            .ok_or_else(|| AppError::InvalidRequest("Choose a scene for the free lab.".into()))?;
        let scene = library.scene(scene_id).ok_or_else(|| AppError::NotFound {
            what: "scene",
            id: scene_id.to_owned(),
        })?;
        let platform = request
            .platform
            .clone()
            .unwrap_or_else(|| "generic.iec".into());
        if library.profile(&platform).is_none() {
            return Err(AppError::NotFound {
                what: "platform",
                id: platform,
            });
        }
        Project {
            format: ProjectFormat::Project,
            format_version: FORMAT_VERSION,
            id,
            name: if name.is_empty() {
                scene.title.clone()
            } else {
                name.to_owned()
            },
            created_at: now.clone(),
            updated_at: now,
            kind: ProjectKind::FreeLab,
            platform: identifier(&platform, "platform")?,
            scene: scene.id.clone(),
            station: scene.reference_station.clone(),
            programs: vec![ProgramDocument {
                id: identifier("main", "program")?,
                name: "Main".into(),
                language: "ST".into(),
                source: starter_program(scene),
            }],
            watch: Vec::new(),
            simulation: SimulationSettings::default(),
            exercise_state: None,
        }
    };
    project.validate()?;
    store.save(&project)?;
    Ok(project)
}

/// Copy a project under a new identity and name ("Save As").
pub fn duplicate(store: &ProjectStore, source: &Project, name: &str) -> AppResult<Project> {
    let mut copy = source.clone();
    copy.id = unique_id(store)?;
    copy.name = name.trim().to_owned();
    copy.created_at = timestamp();
    copy.updated_at = copy.created_at.clone();
    copy.validate()?;
    store.save(&copy)?;
    Ok(copy)
}

/// Import a `.plclab` document (migrating older formats) as a new project.
/// Resolve a `.plclab` file chosen outside the application (the root launcher or a
/// second launch with `--open-project`). A file that belongs to this store opens in
/// place; any other file is validated and imported as a copy, so the original is never
/// written. Only regular `.plclab` files up to the project size limit are read.
pub fn open_file(
    store: &ProjectStore,
    library: &KnowledgeLibrary,
    path: &std::path::Path,
) -> AppResult<Project> {
    let shown = path.display().to_string();
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(project_model::EXTENSION))
    {
        return Err(AppError::InvalidRequest(format!(
            "{shown} is not a PLC Lab project (.plclab) file."
        )));
    }
    let meta = std::fs::metadata(path).map_err(|_| AppError::NotFound {
        what: "project file",
        id: shown.clone(),
    })?;
    if !meta.is_file() {
        return Err(AppError::InvalidRequest(format!("{shown} is not a file.")));
    }
    if meta.len() > project_model::MAX_PROJECT_BYTES {
        return Err(AppError::InvalidRequest(
            "The project file is too large.".into(),
        ));
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| AppError::NotFound {
        what: "project file",
        id: shown.clone(),
    })?;
    if let Ok(dir) = std::fs::canonicalize(store.directory())
        && canonical.parent() == Some(dir.as_path())
        && let Some(id) = canonical
            .file_stem()
            .and_then(|s| Identifier::new(s.to_string_lossy().into_owned()).ok())
        && let Ok(project) = store.load(&id)
    {
        return Ok(project);
    }
    let text = std::fs::read_to_string(&canonical)
        .map_err(|e| AppError::InvalidRequest(format!("{shown} could not be read: {e}")))?;
    import(store, library, &text)
}

pub fn import(store: &ProjectStore, library: &KnowledgeLibrary, text: &str) -> AppResult<Project> {
    if text.len() as u64 > project_model::MAX_PROJECT_BYTES {
        return Err(AppError::InvalidRequest(
            "The project file is too large.".into(),
        ));
    }
    let mut project = project_model::parse(text)?;
    if library.scene(project.scene.as_str()).is_none() {
        return Err(AppError::NotFound {
            what: "scene",
            id: project.scene.to_string(),
        });
    }
    if library.profile(project.platform.as_str()).is_none() {
        return Err(AppError::NotFound {
            what: "platform",
            id: project.platform.to_string(),
        });
    }
    project.id = unique_id(store)?;
    project.updated_at = timestamp();
    store.save(&project)?;
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_rfc3339_utc() {
        let t = timestamp();
        assert_eq!(t.len(), 20);
        assert!(t.ends_with('Z') && t.as_bytes()[10] == b'T');
        assert!(t.as_str() >= "2024-01-01T00:00:00Z");
    }
}

//! Desktop application adapter: application use cases over the headless lab engine,
//! the knowledge library and the project store, exposed as IPC contract DTOs.
//!
//! This crate owns no PLC, process, network or grading semantics. It sequences use cases
//! (open a workspace, build and load, run the simulation, investigate a fault, check an
//! exercise), owns the single open workspace and maps typed errors to transport errors
//! at its public boundary. It has no Tauri or user-interface dependency.

mod convert;
mod error;
mod library;
mod projects;
mod workspace;

pub use error::AppError;
pub use workspace::{RunLease, RunStart, TickOutcome};

use app_contract::*;
use error::AppResult;
use knowledge_core::KnowledgeLibrary;
use lab_engine::AssessmentOutcome;
use project_model::{ExerciseState, Progress, ProjectKind, ProjectStore, RecentProjects};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};
use workspace::Workspace;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Trusted locations chosen by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Knowledge pack roots, searched in order.
    pub knowledge_roots: Vec<PathBuf>,
    pub projects_dir: PathBuf,
    pub progress_file: PathBuf,
    /// Recently opened projects (trusted metadata read by the root launcher).
    pub recent_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    pub app_version: String,
    pub build_context: String,
}

struct AppState {
    library: KnowledgeLibrary,
    store: ProjectStore,
    progress: Progress,
    /// False when an unreadable progress file could not be moved aside.
    progress_writable: bool,
    /// Non-fatal startup problems shown to the learner.
    notices: Vec<String>,
    recent: RecentProjects,
    /// A project another launch asked to open, until the view takes it.
    pending_open: Option<OpenRequestDto>,
    workspace: Option<Workspace>,
    next_workspace: u64,
    next_run: u64,
}

pub struct DesktopApp {
    paths: AppPaths,
    info: AppInfo,
    state: Mutex<AppState>,
}

fn version(v: u32) -> AppResult<()> {
    if v == CONTRACT_VERSION {
        Ok(())
    } else {
        Err(AppError::ContractVersion(v))
    }
}

/// The boundary: typed application errors become transport errors here and only here.
fn boundary<T>(result: AppResult<T>) -> Result<T, IpcError> {
    result.map_err(IpcError::from)
}

impl AppState {
    fn workspace(&mut self, id: ExactU64) -> AppResult<&mut Workspace> {
        self.workspace
            .as_mut()
            .filter(|w| w.id == id.0)
            .ok_or(AppError::StaleWorkspace)
    }
    fn save_project(&self, project: &project_model::Project) -> AppResult<()> {
        self.store.save(project).map_err(AppError::from)
    }
    fn save_progress(&self, path: &std::path::Path) -> AppResult<()> {
        if self.progress_writable {
            self.progress.save(path)?;
        }
        Ok(())
    }
}

impl DesktopApp {
    pub fn new(paths: AppPaths, info: AppInfo) -> Self {
        let library = KnowledgeLibrary::load(&paths.knowledge_roots);
        let loaded = Progress::load(&paths.progress_file);
        let recent = RecentProjects::load(&paths.recent_file);
        let store = ProjectStore::new(paths.projects_dir.clone());
        Self {
            paths,
            info,
            state: Mutex::new(AppState {
                library,
                store,
                progress: loaded.progress,
                progress_writable: loaded.writable,
                notices: loaded.notice.into_iter().collect(),
                recent,
                pending_open: None,
                workspace: None,
                next_workspace: 0,
                next_run: 0,
            }),
        }
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, AppState>> {
        self.state.lock().map_err(|_| AppError::Unavailable)
    }

    fn search_paths(&self) -> Vec<String> {
        self.paths
            .knowledge_roots
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    // ---- application ------------------------------------------------------------------

    pub fn core_info(&self, request: VersionedRequest) -> Result<CoreInfo, IpcError> {
        boundary(version(request.contract_version).map(|()| CoreInfo {
            application_name: "PLC Lab".into(),
            app_version: self.info.app_version.clone(),
            core_version: lab_engine::VERSION.into(),
            contract_version: CONTRACT_VERSION,
            execution: "manual-and-continuous".into(),
            build_context: self.info.build_context.clone(),
        }))
    }

    // ---- knowledge and curriculum ----------------------------------------------------------

    pub fn knowledge_status(
        &self,
        request: VersionedRequest,
    ) -> Result<KnowledgeStatusDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            Ok(library::status(&self.lock()?.library, &self.search_paths()))
        })())
    }

    pub fn knowledge_search(
        &self,
        request: KnowledgeSearchRequest,
    ) -> Result<Vec<SearchHitDto>, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            library::search(&self.lock()?.library, &request)
        })())
    }

    pub fn knowledge_article(&self, request: IdRequest) -> Result<ArticleDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            library::article(&self.lock()?.library, &request.id)
        })())
    }

    pub fn knowledge_catalog(&self, request: VersionedRequest) -> Result<CatalogDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            Ok(library::catalog(&self.lock()?.library))
        })())
    }

    pub fn library(&self, request: VersionedRequest) -> Result<LibraryDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            Ok(library::library(
                &state.library,
                &state.progress,
                &state.notices,
            ))
        })())
    }

    pub fn lesson(&self, request: IdRequest) -> Result<LessonDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            library::lesson(&self.lock()?.library, &request.id)
        })())
    }

    pub fn complete_lesson(&self, request: LessonCompleteRequest) -> Result<LibraryDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            if state.library.lesson(&request.id).is_none() {
                return Err(AppError::NotFound {
                    what: "lesson",
                    id: request.id.clone(),
                });
            }
            if request.correct > request.total {
                return Err(AppError::InvalidRequest(
                    "More correct answers than questions.".into(),
                ));
            }
            state
                .progress
                .record_lesson(&request.id, request.correct, request.total);
            state.save_progress(&self.paths.progress_file)?;
            Ok(library::library(
                &state.library,
                &state.progress,
                &state.notices,
            ))
        })())
    }

    pub fn exercise(&self, request: IdRequest) -> Result<ExerciseDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            library::exercise_by_id(&self.lock()?.library, &request.id)
        })())
    }

    // ---- projects ----------------------------------------------------------------------

    pub fn projects(&self, request: VersionedRequest) -> Result<ProjectListDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            let (list, problems) = state.store.list();
            Ok(ProjectListDto {
                projects: list.iter().map(projects::summary_dto).collect(),
                problems,
                directory: state.store.directory().to_string_lossy().into_owned(),
            })
        })())
    }

    pub fn create_project(&self, request: ProjectCreateRequest) -> Result<ProjectDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            projects::to_dto(&projects::create(&state.store, &state.library, &request)?)
        })())
    }

    pub fn project(&self, request: IdRequest) -> Result<ProjectDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            projects::to_dto(
                &state
                    .store
                    .load(&projects::identifier(&request.id, "project")?)?,
            )
        })())
    }

    pub fn save_project(&self, request: ProjectSaveRequest) -> Result<ProjectDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let id = projects::identifier(&request.project.id, "project")?;
            let mut stored = match state.workspace.as_ref().filter(|w| w.project.id == id) {
                Some(w) => w.project.clone(),
                None => state.store.load(&id)?,
            };
            projects::apply_edit(&mut stored, &request.project, &state.library)?;
            state.save_project(&stored)?;
            if let Some(w) = state.workspace.as_mut().filter(|w| w.project.id == id) {
                // Keep live-only fields (exercise state) owned by the workspace.
                let exercise_state = w.project.exercise_state.clone();
                w.project = stored.clone();
                w.project.exercise_state = exercise_state;
            }
            projects::to_dto(&stored)
        })())
    }

    pub fn save_project_as(&self, request: ProjectSaveAsRequest) -> Result<ProjectDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            let id = projects::identifier(&request.project_id, "project")?;
            let source = match state.workspace.as_ref().filter(|w| w.project.id == id) {
                Some(w) => w.project.clone(),
                None => state.store.load(&id)?,
            };
            projects::to_dto(&projects::duplicate(&state.store, &source, &request.name)?)
        })())
    }

    pub fn update_source(
        &self,
        request: SourceUpdateRequest,
    ) -> Result<ProjectSummaryDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            if request.source.len() > 256 * 1024 {
                return Err(AppError::InvalidRequest(
                    "Program documents are limited to 256 KiB.".into(),
                ));
            }
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let id = projects::identifier(&request.project_id, "project")?;
            let mut project = match state.workspace.as_ref().filter(|w| w.project.id == id) {
                Some(w) => w.project.clone(),
                None => state.store.load(&id)?,
            };
            let doc = project
                .programs
                .iter_mut()
                .find(|p| p.id.as_str() == request.program_id)
                .ok_or_else(|| AppError::NotFound {
                    what: "program",
                    id: request.program_id.clone(),
                })?;
            doc.source = request.source.clone();
            project.updated_at = projects::timestamp();
            state.save_project(&project)?;
            if let Some(w) = state.workspace.as_mut().filter(|w| w.project.id == id)
                && let Some(doc) = w
                    .project
                    .programs
                    .iter_mut()
                    .find(|p| p.id.as_str() == request.program_id)
            {
                doc.source = request.source;
                w.project.updated_at = project.updated_at.clone();
            }
            Ok(projects::summary_dto(&project_model::ProjectSummary {
                id: project.id,
                name: project.name,
                kind: project.kind,
                platform: project.platform,
                scene: project.scene,
                updated_at: project.updated_at,
            }))
        })())
    }

    pub fn delete_project(&self, request: IdRequest) -> Result<ProjectListDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            {
                let mut state = self.lock()?;
                let id = projects::identifier(&request.id, "project")?;
                if state.workspace.as_ref().is_some_and(|w| w.project.id == id) {
                    state.workspace = None;
                }
                state.store.delete(&id)?;
                state.recent.forget(&id);
                // Advisory list, see open_workspace.
                let _ = state.recent.save(&self.paths.recent_file);
            }
            self.projects(VersionedRequest {
                contract_version: CONTRACT_VERSION,
            })
            .map_err(|e| AppError::InvalidRequest(e.message))
        })())
    }

    // ---- projects requested by other launches ------------------------------------------

    /// Resolve a project file that a launch asked to open (`--open-project`) and keep the
    /// outcome until the view takes it. Called by the trusted host only.
    pub fn request_open(&self, path: &Path) -> OpenRequestDto {
        let outcome = self.lock().map_err(IpcError::from).and_then(|state| {
            projects::open_file(&state.store, &state.library, path).map_err(IpcError::from)
        });
        let request = match outcome {
            Ok(project) => OpenRequestDto {
                project_id: Some(project.id.to_string()),
                name: Some(project.name),
                problem: None,
            },
            Err(e) => OpenRequestDto {
                project_id: None,
                name: None,
                problem: Some(e.message),
            },
        };
        if let Ok(mut state) = self.lock() {
            state.pending_open = Some(request.clone());
        }
        request
    }

    /// The pending open request, once.
    pub fn take_open_request(
        &self,
        request: VersionedRequest,
    ) -> Result<Option<OpenRequestDto>, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            Ok(self.lock()?.pending_open.take())
        })())
    }

    pub fn import_project(&self, request: ProjectImportRequest) -> Result<ProjectDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            projects::to_dto(&projects::import(
                &state.store,
                &state.library,
                &request.text,
            )?)
        })())
    }

    pub fn export_project(&self, request: IdRequest) -> Result<ProjectExportDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let state = self.lock()?;
            let id = projects::identifier(&request.id, "project")?;
            let project = match state.workspace.as_ref().filter(|w| w.project.id == id) {
                Some(w) => w.project.clone(),
                None => state.store.load(&id)?,
            };
            let safe: String = project
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            Ok(ProjectExportDto {
                file_name: format!("{safe}.{}", project_model::EXTENSION),
                text: project_model::to_json(&project)?,
            })
        })())
    }

    // ---- workspace lifecycle ------------------------------------------------------------

    pub fn open_workspace(&self, request: WorkspaceOpenRequest) -> Result<WorkspaceDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let id = projects::identifier(&request.project_id, "project")?;
            let project = state.store.load(&id)?;
            let scene = state
                .library
                .scene(project.scene.as_str())
                .cloned()
                .ok_or_else(|| AppError::NotFound {
                    what: "scene",
                    id: project.scene.to_string(),
                })?;
            let profile = state
                .library
                .profile(project.platform.as_str())
                .ok_or_else(|| AppError::NotFound {
                    what: "platform",
                    id: project.platform.to_string(),
                })?;
            let exercise = match &project.kind {
                ProjectKind::Exercise { exercise, .. } => Some(
                    state
                        .library
                        .exercise(exercise.as_str())
                        .cloned()
                        .ok_or_else(|| AppError::NotFound {
                            what: "exercise",
                            id: exercise.to_string(),
                        })?,
                ),
                ProjectKind::FreeLab => None,
            };
            // Persist the previous workspace's live-only state before replacing it.
            if let Some(previous) = state.workspace.take() {
                state.save_project(&previous.project)?;
            }
            state.next_workspace += 1;
            let workspace =
                Workspace::open(state.next_workspace, project, scene, profile, exercise)?;
            let exercise_dto = workspace
                .exercise
                .as_ref()
                .map(|e| library::exercise_view(&state.library, e, workspace.diagnosed()));
            let dto = workspace.workspace_dto(&request.project_id, exercise_dto);
            state
                .recent
                .record(&id, &workspace.project.name, &projects::timestamp());
            // The recent list only offers a shortcut in the launcher; a failed write must
            // not prevent opening the project, and the next open rewrites the list.
            let _ = state.recent.save(&self.paths.recent_file);
            state.workspace = Some(workspace);
            Ok(dto)
        })())
    }

    pub fn close_workspace(&self, request: WorkspaceRequest) -> Result<(), IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            state.workspace(request.workspace_id)?;
            if let Some(w) = state.workspace.take() {
                state.save_project(&w.project)?;
            }
            Ok(())
        })())
    }

    pub fn compile(&self, request: CompileRequest) -> Result<CompileResultDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            if request.source.len() > 256 * 1024 {
                return Err(AppError::InvalidRequest(
                    "Program documents are limited to 256 KiB.".into(),
                ));
            }
            Ok(match lab_engine::compile(&request.source) {
                Ok(compiled) => CompileResultDto {
                    success: true,
                    program_name: Some(compiled.program_name().to_owned()),
                    diagnostics: Vec::new(),
                    declarations: compiled
                        .declarations()
                        .iter()
                        .map(|d| DeclarationDto {
                            name: d.name.clone(),
                            data_type: d.type_name.clone(),
                            role: d.role.clone(),
                        })
                        .collect(),
                },
                Err(d) => CompileResultDto {
                    success: false,
                    program_name: None,
                    diagnostics: d.iter().map(convert::diagnostic).collect(),
                    declarations: Vec::new(),
                },
            })
        })())
    }

    /// Basic "Build & Load to PLC": compile, validate, link to the I/O configuration and
    /// load into the CPU in one backend transaction. With `run_after_load` the PLC is
    /// switched to RUN and the simulation started; the host paces the returned run.
    pub fn build_and_load(
        &self,
        request: BuildLoadRequest,
    ) -> Result<(BuildLoadResultDto, Option<RunStart>), IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let station = request
                .station
                .as_ref()
                .map(convert::station_from_dto)
                .transpose()
                .map_err(|e| AppError::InvalidRequest(e.message))?;
            let next_run = &mut state.next_run;
            let w = state
                .workspace
                .as_mut()
                .filter(|w| w.id == request.workspace_id.0)
                .ok_or(AppError::StaleWorkspace)?;
            let mut result = w.build_and_load(&request.source, request.revision.0, station)?;
            let mut start = None;
            if result.outcome == "LOADED" {
                w.project.updated_at = projects::timestamp();
                let project = w.project.clone();
                if request.run_after_load {
                    w.set_plc_mode("RUN")?;
                    start = w.simulation("RUN", next_run)?;
                    result.frame = w.frame(&[]);
                }
                state.store.save(&project)?;
            }
            Ok((result, start))
        })())
    }

    pub fn plc_mode(&self, request: PlcModeRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            w.set_plc_mode(&request.mode)?;
            Ok(w.frame(&[]))
        })())
    }

    pub fn plc_reset(&self, request: WorkspaceRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            w.reset_plc()?;
            Ok(w.frame(&[]))
        })())
    }

    pub fn simulation(
        &self,
        request: SimulationRequest,
    ) -> Result<(LiveFrameDto, Option<RunStart>), IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let next_run = &mut state.next_run;
            let w = state
                .workspace
                .as_mut()
                .filter(|w| w.id == request.workspace_id.0)
                .ok_or(AppError::StaleWorkspace)?;
            let start = w.simulation(&request.command, next_run)?;
            Ok((w.frame(&[]), start))
        })())
    }

    pub fn set_speed(&self, request: SpeedRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            w.set_speed(request.speed_percent)?;
            Ok(w.frame(&[]))
        })())
    }

    /// One paced cycle for the host. Typed outcome: the host never interprets error codes.
    pub fn tick(&self, lease: RunLease) -> Result<TickOutcome, IpcError> {
        boundary((|| {
            let mut state = self.lock()?;
            match state.workspace.as_mut() {
                Some(w) if w.id == lease.workspace => w.tick(lease),
                _ => Ok(TickOutcome::Stale),
            }
        })())
    }

    /// The host could not complete a pacing step (worker or notification failure).
    pub fn driver_failed(&self, lease: RunLease, message: String) {
        if let Ok(mut state) = self.state.lock()
            && let Some(w) = state.workspace.as_mut()
        {
            w.driver_failed(lease, message);
        }
    }

    // ---- operator and engineering actions -------------------------------------------------

    pub fn operate(&self, request: OperateRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            w.operate(&request.component, &request.action)?;
            Ok(w.frame(&[]))
        })())
    }

    pub fn force(&self, request: ForceRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            w.force(request.signal, request.value)?;
            Ok(w.frame(&[]))
        })())
    }

    pub fn diagnostic_buffer(
        &self,
        request: DiagnosticBufferRequest,
    ) -> Result<DiagnosticBufferDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            Ok(state
                .workspace(request.workspace_id)?
                .diagnostic_buffer(&request))
        })())
    }

    pub fn forces(&self, request: WorkspaceRequest) -> Result<ForceListDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            Ok(state.workspace(request.workspace_id)?.forces())
        })())
    }

    /// Remove every active force in one request; returns the frame without them.
    pub fn remove_all_forces(&self, request: WorkspaceRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            w.remove_all_forces()?;
            Ok(w.frame(&[]))
        })())
    }

    // ---- observation -----------------------------------------------------------------------

    pub fn frame(&self, request: FrameRequest) -> Result<LiveFrameDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            Ok(state.workspace(request.workspace_id)?.frame(&request.watch))
        })())
    }

    pub fn cycle(&self, request: CycleRequest) -> Result<Option<CauseEffectDto>, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            let w = state.workspace(request.workspace_id)?;
            Ok(w.session
                .cause_effect_at(request.cycle.0)
                .as_ref()
                .map(convert::cause_effect))
        })())
    }

    pub fn watch(&self, request: WatchRequest) -> Result<WatchDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            if request.signals.len() > project_model::MAX_WATCH_ROWS {
                return Err(AppError::InvalidRequest(format!(
                    "A watch table holds at most {} signals.",
                    project_model::MAX_WATCH_ROWS
                )));
            }
            let mut state = self.lock()?;
            Ok(state
                .workspace(request.workspace_id)?
                .watch(&request.signals))
        })())
    }

    pub fn variables(&self, request: VariablesRequest) -> Result<VariablesDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            if request.filter.as_ref().is_some_and(|f| f.len() > 64) {
                return Err(AppError::InvalidRequest(
                    "The variable filter is limited to 64 characters.".into(),
                ));
            }
            let mut state = self.lock()?;
            Ok(state.workspace(request.workspace_id)?.variables(
                request.filter.as_deref(),
                request.offset,
                request.limit,
            ))
        })())
    }

    pub fn trend(&self, request: TrendRequest) -> Result<TrendDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            state.workspace(request.workspace_id)?.trend(
                &request.signals,
                request.from_us.0,
                request.to_us.0,
                request.max_points,
            )
        })())
    }

    pub fn events(&self, request: EventsRequest) -> Result<EventsPageDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            Ok(state
                .workspace(request.workspace_id)?
                .events(request.after.map(|a| a.0), request.limit))
        })())
    }

    pub fn hardware(&self, request: WorkspaceRequest) -> Result<HardwareDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            state.workspace(request.workspace_id)?.hardware()
        })())
    }

    /// The commissioning checklist of a workspace, evaluated from the simulated installation.
    pub fn commissioning(&self, request: WorkspaceRequest) -> Result<CommissioningDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            Ok(state.workspace(request.workspace_id)?.commissioning())
        })())
    }

    pub fn network(&self, request: WorkspaceRequest) -> Result<NetworkDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            Ok(state.workspace(request.workspace_id)?.network())
        })())
    }

    // ---- troubleshooting ---------------------------------------------------------------------

    pub fn measure(&self, request: MeasureRequest) -> Result<MeasurementDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            state
                .workspace(request.workspace_id)?
                .measure(&request.channel)
        })())
    }

    pub fn ping(&self, request: PingRequest) -> Result<PingDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            state.workspace(request.workspace_id)?.ping(&request.ip)
        })())
    }

    pub fn note_evidence(&self, request: EvidenceRequest) -> Result<(), IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut state = self.lock()?;
            state
                .workspace(request.workspace_id)?
                .note_evidence(&request.evidence)
        })())
    }

    pub fn diagnose(&self, request: DiagnosisRequest) -> Result<DiagnosisResultDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let w = state
                .workspace
                .as_mut()
                .filter(|w| w.id == request.workspace_id.0)
                .ok_or(AppError::StaleWorkspace)?;
            let result = w.diagnose(&request.option)?;
            let project = w.project.clone();
            state.store.save(&project)?;
            Ok(result)
        })())
    }

    pub fn repair(&self, request: RepairRequest) -> Result<RepairResultDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let mut guard = self.lock()?;
            let state = &mut *guard;
            let w = state
                .workspace
                .as_mut()
                .filter(|w| w.id == request.workspace_id.0)
                .ok_or(AppError::StaleWorkspace)?;
            let message = w.repair(&request.repair)?;
            let frame = w.frame(&[]);
            let project = w.project.clone();
            state.store.save(&project)?;
            Ok(RepairResultDto { message, frame })
        })())
    }

    /// Grade the exercise in an isolated headless assessment. The live session is never
    /// used for grading; troubleshooting verification runs against the faults the student
    /// has not repaired yet.
    pub fn check(&self, request: CheckRequest) -> Result<GradeReportDto, IpcError> {
        boundary((|| {
            version(request.contract_version)?;
            let (exercise, scene, profile, station, faults, investigation, exercise_id, references) = {
                let mut state = self.lock()?;
                let w = state.workspace(request.workspace_id)?;
                let exercise = w.exercise.clone().ok_or_else(|| {
                    AppError::InvalidRequest("Free labs have no exercise to check.".into())
                })?;
                let station = match &request.station {
                    Some(s) => convert::station_from_dto(s)
                        .map_err(|e| AppError::InvalidRequest(e.message))?,
                    None => w.project.station.clone(),
                };
                let (scene, profile, faults, investigation, exercise_id) = (
                    w.scene.clone(),
                    w.profile.clone(),
                    w.remaining_faults(),
                    w.investigation.clone(),
                    w.exercise_id(),
                );
                let references = library::debrief_references(&state.library, &exercise);
                (
                    exercise,
                    scene,
                    profile,
                    station,
                    faults,
                    investigation,
                    exercise_id,
                    references,
                )
            };
            // Assessment runs without holding the application lock.
            let filter = exercise
                .troubleshooting
                .as_ref()
                .map(|t| t.verification.clone());
            let outcome = lab_engine::assess(
                &exercise,
                &scene,
                &station,
                &profile,
                &request.source,
                &faults,
                filter.as_deref(),
            )
            .map_err(|e| AppError::InvalidRequest(format!("{e:?}")))?;
            let report = match outcome {
                AssessmentOutcome::Graded(report) => {
                    let report = match (&exercise.kind, &exercise.troubleshooting) {
                        (kind, Some(t)) if kind.is_fault_finding() => {
                            grading_core::grade_troubleshooting(
                                &t.root_causes
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>(),
                                &grading_core::InvestigationSummary {
                                    diagnoses: investigation.diagnoses.clone(),
                                    evidence_viewed: investigation.evidence.clone(),
                                    repairs: investigation.repairs.clone(),
                                    remaining_faults: faults
                                        .iter()
                                        .map(|f| f.id.to_string())
                                        .collect(),
                                },
                                report,
                            )
                        }
                        _ => report,
                    };
                    // The explanation is revealed only once the repaired system passes.
                    let debrief = exercise
                        .troubleshooting
                        .as_ref()
                        .and_then(|t| t.debrief.as_ref())
                        .filter(|_| report.passed)
                        .map(|d| DebriefDto {
                            root_cause: d.root_cause.clone(),
                            evidence: d.evidence.clone(),
                            repair: d.repair.clone(),
                            reasoning: d.reasoning.clone(),
                            references: references.clone(),
                        });
                    GradeReportDto {
                        outcome: "GRADED".into(),
                        score: report.score,
                        max_score: report.max_score,
                        passed: report.passed,
                        criteria: report
                            .criteria
                            .into_iter()
                            .map(|c| CriterionResultDto {
                                id: c.id,
                                title: c.title,
                                points: c.points,
                                earned: c.earned,
                                passed: c.passed,
                                feedback: c.feedback,
                                detail: c.detail,
                            })
                            .collect(),
                        diagnostics: Vec::new(),
                        problems: Vec::new(),
                        debrief,
                    }
                }
                AssessmentOutcome::CompileFailed(d) => not_graded(
                    "COMPILE_FAILED",
                    d.iter().map(convert::diagnostic).collect(),
                    Vec::new(),
                ),
                AssessmentOutcome::InterfaceMismatch(p) => {
                    not_graded("INTERFACE_MISMATCH", Vec::new(), p)
                }
                AssessmentOutcome::BindingFailed(b) => not_graded(
                    "BINDING_FAILED",
                    Vec::new(),
                    b.into_iter()
                        .filter(|i| i.blocking)
                        .map(|i| format!("{}: {}", i.tag, i.message))
                        .collect(),
                ),
            };
            let mut guard = self.lock()?;
            let state = &mut *guard;
            if let Some(id) = &exercise_id {
                let max = if report.outcome == "GRADED" {
                    report.max_score
                } else {
                    0
                };
                state
                    .progress
                    .record_exercise(id, report.score, max, report.passed);
                state.save_progress(&self.paths.progress_file)?;
                if let Some(w) = state
                    .workspace
                    .as_mut()
                    .filter(|w| w.id == request.workspace_id.0)
                {
                    let s = w
                        .project
                        .exercise_state
                        .get_or_insert_with(ExerciseState::default);
                    s.attempts += 1;
                    s.best_score = s.best_score.max(report.score);
                    s.passed |= report.passed;
                    let project = w.project.clone();
                    state.store.save(&project)?;
                }
            }
            Ok(report)
        })())
    }
}

fn not_graded(
    outcome: &str,
    diagnostics: Vec<DiagnosticDto>,
    problems: Vec<String>,
) -> GradeReportDto {
    GradeReportDto {
        outcome: outcome.into(),
        score: 0,
        max_score: 0,
        passed: false,
        criteria: Vec::new(),
        diagnostics,
        problems,
        debrief: None,
    }
}

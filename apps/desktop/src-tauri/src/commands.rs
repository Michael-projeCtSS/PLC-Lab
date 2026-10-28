//! Tauri command adapters. Each command runs one application use case on a blocking
//! worker and returns contract DTOs. No PLC, process, network or grading logic here.
use app_contract::*;
use desktop_session::DesktopApp;
use std::sync::Arc;

type App<'a> = tauri::State<'a, Arc<DesktopApp>>;

async fn work<T: Send + 'static>(
    app: App<'_>,
    operation: impl FnOnce(&DesktopApp) -> Result<T, IpcError> + Send + 'static,
) -> Result<T, IpcError> {
    let app = Arc::clone(app.inner());
    tauri::async_runtime::spawn_blocking(move || operation(&app))
        .await
        .map_err(|_| IpcError::new("APP-WORKER", "The request could not finish."))?
}

macro_rules! commands {
    ($($name:ident($request:ty) -> $response:ty => $method:ident;)*) => {$(
        #[tauri::command]
        pub async fn $name(app: App<'_>, request: $request) -> Result<$response, IpcError> {
            work(app, move |a| a.$method(request)).await
        }
    )*};
}

commands! {
    core_info(VersionedRequest) -> CoreInfo => core_info;
    knowledge_status(VersionedRequest) -> KnowledgeStatusDto => knowledge_status;
    knowledge_search(KnowledgeSearchRequest) -> Vec<SearchHitDto> => knowledge_search;
    knowledge_article(IdRequest) -> ArticleDto => knowledge_article;
    knowledge_catalog(VersionedRequest) -> CatalogDto => knowledge_catalog;
    library(VersionedRequest) -> LibraryDto => library;
    lesson(IdRequest) -> LessonDto => lesson;
    complete_lesson(LessonCompleteRequest) -> LibraryDto => complete_lesson;
    exercise(IdRequest) -> ExerciseDto => exercise;
    projects(VersionedRequest) -> ProjectListDto => projects;
    create_project(ProjectCreateRequest) -> ProjectDto => create_project;
    project(IdRequest) -> ProjectDto => project;
    save_project(ProjectSaveRequest) -> ProjectDto => save_project;
    save_project_as(ProjectSaveAsRequest) -> ProjectDto => save_project_as;
    update_source(SourceUpdateRequest) -> ProjectSummaryDto => update_source;
    delete_project(IdRequest) -> ProjectListDto => delete_project;
    import_project(ProjectImportRequest) -> ProjectDto => import_project;
    export_project(IdRequest) -> ProjectExportDto => export_project;
    open_workspace(WorkspaceOpenRequest) -> WorkspaceDto => open_workspace;
    close_workspace(WorkspaceRequest) -> () => close_workspace;
    compile(CompileRequest) -> CompileResultDto => compile;
    plc_mode(PlcModeRequest) -> LiveFrameDto => plc_mode;
    plc_reset(WorkspaceRequest) -> LiveFrameDto => plc_reset;
    set_speed(SpeedRequest) -> LiveFrameDto => set_speed;
    operate(OperateRequest) -> LiveFrameDto => operate;
    force(ForceRequest) -> LiveFrameDto => force;
    frame(FrameRequest) -> LiveFrameDto => frame;
    cycle(CycleRequest) -> Option<CauseEffectDto> => cycle;
    variables(VariablesRequest) -> VariablesDto => variables;
    trend(TrendRequest) -> TrendDto => trend;
    events(EventsRequest) -> EventsPageDto => events;
    hardware(WorkspaceRequest) -> HardwareDto => hardware;
    commissioning(WorkspaceRequest) -> CommissioningDto => commissioning;
    network(WorkspaceRequest) -> NetworkDto => network;
    measure(MeasureRequest) -> MeasurementDto => measure;
    ping(PingRequest) -> PingDto => ping;
    note_evidence(EvidenceRequest) -> () => note_evidence;
    diagnose(DiagnosisRequest) -> DiagnosisResultDto => diagnose;
    repair(RepairRequest) -> RepairResultDto => repair;
    check(CheckRequest) -> GradeReportDto => check;
    take_open_request(VersionedRequest) -> Option<OpenRequestDto> => take_open_request;
}

/// Build & Load; starts the paced simulation when the use case asks for it.
#[tauri::command]
pub async fn build_and_load(
    app: App<'_>,
    handle: tauri::AppHandle,
    request: BuildLoadRequest,
) -> Result<BuildLoadResultDto, IpcError> {
    let service = Arc::clone(app.inner());
    let (result, start) = work(app, move |a| a.build_and_load(request)).await?;
    if let Some(start) = start {
        crate::execution_driver::spawn(handle, service, start);
    }
    Ok(result)
}

/// Simulation Run/Pause/Step/Stop; a Run starts the host pacing loop.
#[tauri::command]
pub async fn simulation(
    app: App<'_>,
    handle: tauri::AppHandle,
    request: SimulationRequest,
) -> Result<LiveFrameDto, IpcError> {
    let service = Arc::clone(app.inner());
    let (frame, start) = work(app, move |a| a.simulation(request)).await?;
    if let Some(start) = start {
        crate::execution_driver::spawn(handle, service, start);
    }
    Ok(frame)
}

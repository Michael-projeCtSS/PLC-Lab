use app_contract::{CoreInfo, CoreInfoRequest, IpcError};
use desktop_session::RuntimeSessionService;
use std::sync::Arc;

#[tauri::command]
pub async fn start_execution(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    let owned = Arc::clone(service.inner());
    let started = session_work(service, move |s| s.start_execution(request)).await?;
    if let Some(lease) = started.driver {
        crate::execution_driver::spawn(
            app,
            owned,
            lease,
            started.snapshot.cycle_period_micros.as_micros(),
        );
    }
    Ok(started.snapshot)
}
#[tauri::command]
pub async fn pause_execution(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.pause_execution(request)).await
}
#[tauri::command]
pub async fn stop_execution(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.stop_execution(request)).await
}

#[tauri::command]
pub async fn get_trace_page(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::TracePageRequest,
) -> Result<app_contract::TracePageDto, IpcError> {
    session_work(service, move |s| s.trace_page(request)).await
}

#[tauri::command]
pub async fn attach_process_scene(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::AttachProcessSceneRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.attach_process(request)).await
}
#[tauri::command]
pub async fn detach_process_scene(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.detach_process(request)).await
}
#[tauri::command]
pub async fn reset_process_scene(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.reset_process(request)).await
}

async fn session_work<T: Send + 'static>(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    operation: impl FnOnce(&RuntimeSessionService) -> Result<T, IpcError> + Send + 'static,
) -> Result<T, IpcError> {
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || operation(&service))
        .await
        .map_err(|_| IpcError {
            code: "APP-RUNTIME-WORKER".into(),
            message: "The runtime request could not finish.".into(),
        })?
}

#[tauri::command]
pub async fn load_st_program(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::LoadStProgramRequest,
) -> Result<app_contract::LoadProgramResponse, IpcError> {
    session_work(service, move |s| s.load(request)).await
}
#[tauri::command]
pub async fn set_runtime_input(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::SetRuntimeInputRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.set_input(request)).await
}
#[tauri::command]
pub async fn single_scan(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.single_scan(request)).await
}
#[tauri::command]
pub async fn reset_runtime(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.reset(request)).await
}
#[tauri::command]
pub async fn unload_program(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::UnloadProgramResponse, IpcError> {
    session_work(service, move |s| s.unload(request)).await
}
#[tauri::command]
pub async fn get_runtime_snapshot(
    service: tauri::State<'_, Arc<RuntimeSessionService>>,
    request: app_contract::RuntimeSessionRequest,
) -> Result<app_contract::RuntimeSnapshotDto, IpcError> {
    session_work(service, move |s| s.snapshot(request)).await
}

#[tauri::command]
pub async fn compile_st(
    request: app_contract::CompileStRequest,
) -> Result<app_contract::CompileResponse, IpcError> {
    tauri::async_runtime::spawn_blocking(move || app_contract::compile_st(request))
        .await
        .map_err(|_| IpcError {
            code: "APP-IPC-005".into(),
            message: "The compilation worker could not finish.".into(),
        })?
}

#[tauri::command]
pub fn get_core_info(request: CoreInfoRequest) -> Result<CoreInfo, IpcError> {
    app_contract::get_core_info(request, env!("CARGO_PKG_VERSION"))
}

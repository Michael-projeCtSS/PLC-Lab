use app_contract::{CoreInfo, CoreInfoRequest, IpcError};

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

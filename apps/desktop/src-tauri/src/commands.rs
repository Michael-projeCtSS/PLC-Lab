use app_contract::{CoreInfo, CoreInfoRequest, IpcError};

#[tauri::command]
pub fn get_core_info(request: CoreInfoRequest) -> Result<CoreInfo, IpcError> {
    app_contract::get_core_info(request, env!("CARGO_PKG_VERSION"))
}

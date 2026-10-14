mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_core_info])
        .run(tauri::generate_context!())
        .expect("PLC Lab desktop host failed to start");
}

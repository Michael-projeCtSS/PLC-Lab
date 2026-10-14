mod commands;

pub fn run() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(
            desktop_session::RuntimeSessionService::default(),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_core_info,
            commands::compile_st,
            commands::load_st_program,
            commands::set_runtime_input,
            commands::single_scan,
            commands::reset_runtime,
            commands::unload_program,
            commands::get_runtime_snapshot,
            commands::attach_process_scene,
            commands::detach_process_scene,
            commands::reset_process_scene
        ])
        .run(tauri::generate_context!())
        .expect("PLC Lab desktop host failed to start");
}

//! PLC Lab desktop host: application lifecycle, trusted storage locations, host pacing
//! and command adapters. Semantics live in the Rust core crates.
mod commands;
mod execution_driver;

use desktop_session::{AppInfo, AppPaths, DesktopApp};
use std::path::PathBuf;
use tauri::Manager;

/// Knowledge pack roots, in priority order: packs shipped next to the executable, packs
/// the user installed into the application data folder, and (development builds only)
/// the repository's canonical pack sources.
fn knowledge_roots(app_data: &std::path::Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        roots.push(dir.join("knowledge-packs"));
    }
    roots.push(app_data.join("knowledge-packs"));
    #[cfg(debug_assertions)]
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../knowledge"));
    roots
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(app_data.join("projects"))?;
            std::fs::create_dir_all(app_data.join("knowledge-packs"))?;
            let paths = AppPaths {
                knowledge_roots: knowledge_roots(&app_data),
                projects_dir: app_data.join("projects"),
                progress_file: app_data.join("progress.json"),
            };
            let info = AppInfo {
                app_version: env!("CARGO_PKG_VERSION").into(),
                build_context: if cfg!(debug_assertions) {
                    "development"
                } else {
                    "release"
                }
                .into(),
            };
            app.manage(std::sync::Arc::new(DesktopApp::new(paths, info)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::core_info,
            commands::knowledge_status,
            commands::knowledge_search,
            commands::knowledge_article,
            commands::knowledge_catalog,
            commands::library,
            commands::lesson,
            commands::complete_lesson,
            commands::exercise,
            commands::projects,
            commands::create_project,
            commands::project,
            commands::save_project,
            commands::save_project_as,
            commands::update_source,
            commands::delete_project,
            commands::import_project,
            commands::export_project,
            commands::open_workspace,
            commands::close_workspace,
            commands::compile,
            commands::build_and_load,
            commands::plc_mode,
            commands::plc_reset,
            commands::simulation,
            commands::set_speed,
            commands::operate,
            commands::force,
            commands::frame,
            commands::cycle,
            commands::variables,
            commands::trend,
            commands::events,
            commands::hardware,
            commands::network,
            commands::measure,
            commands::ping,
            commands::note_evidence,
            commands::diagnose,
            commands::repair,
            commands::check
        ])
        .run(tauri::generate_context!())
        .expect("PLC Lab desktop host failed to start");
}

//! PLC Lab desktop host: application lifecycle, trusted storage locations, host pacing
//! and command adapters. Semantics live in the Rust core crates.
mod commands;
mod execution_driver;
mod startup;

use desktop_session::{AppInfo, AppPaths, DesktopApp};
use std::{path::PathBuf, sync::Arc};
use tauri::{Emitter, Manager};

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

/// Bring the main window to the front (restoring it when minimized).
fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Resolve a launch's typed open request and tell the view that one is waiting.
fn open_requested(app: &tauri::AppHandle, args: &[String], cwd: &std::path::Path) {
    let Some(path) = startup::open_request(args, cwd) else {
        return;
    };
    if let Some(service) = app.try_state::<Arc<DesktopApp>>() {
        service.request_open(&path);
        let _ = app.emit(
            app_contract::OPEN_REQUEST_EVENT,
            app_contract::OpenRequestEventDto {
                contract_version: app_contract::CONTRACT_VERSION,
            },
        );
    }
}

pub fn run() {
    tauri::Builder::default()
        // One application state per user: a second launch (double-click, root launcher or
        // BAT) must not load its own copy of progress and projects and later overwrite the
        // first. It hands over to the running instance, which comes to the front, and exits.
        // Its arguments pass only through the typed `startup::open_request` parser.
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            open_requested(app, &args, std::path::Path::new(&cwd));
            focus_main_window(app);
        }))
        .setup(|app| {
            // PLC_LAB_DATA_DIR lets a test harness or a portable installation choose the
            // data folder; it is read once by the trusted host, never by student code.
            let app_data = match std::env::var_os("PLC_LAB_DATA_DIR") {
                Some(dir) if !dir.is_empty() => PathBuf::from(dir),
                _ => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(app_data.join("projects"))?;
            std::fs::create_dir_all(app_data.join("knowledge-packs"))?;
            let paths = AppPaths {
                knowledge_roots: knowledge_roots(&app_data),
                projects_dir: app_data.join("projects"),
                progress_file: app_data.join("progress.json"),
                recent_file: app_data.join("recent.json"),
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
            app.manage(Arc::new(DesktopApp::new(paths, info)));
            // Lossy: an argument that is not valid Unicode is not a project path we can open.
            let args: Vec<String> = std::env::args_os()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            let cwd = std::env::current_dir().unwrap_or_default();
            if let Some(path) = startup::open_request(&args, &cwd) {
                // The view takes the request when it starts; no event is needed yet.
                app.state::<Arc<DesktopApp>>().request_open(&path);
            }
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
            commands::check,
            commands::take_open_request
        ])
        .run(tauri::generate_context!())
        .expect("PLC Lab desktop host failed to start");
}

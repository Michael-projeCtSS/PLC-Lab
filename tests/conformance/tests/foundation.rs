//! Cross-crate conformance: every core module links without a desktop host, and the
//! desktop adapter reports the real engine version over the current contract.
use app_contract::{CONTRACT_VERSION, VersionedRequest};
use desktop_session::{AppInfo, AppPaths, DesktopApp};

#[test]
fn adapter_reports_the_engine_version_and_rejects_other_contract_versions() {
    let dir = std::env::temp_dir().join(format!("plclab-conformance-{}", std::process::id()));
    let app = DesktopApp::new(
        AppPaths {
            knowledge_roots: Vec::new(),
            projects_dir: dir.join("projects"),
            progress_file: dir.join("progress.json"),
            recent_file: dir.join("recent.json"),
        },
        AppInfo {
            app_version: "0.1.0".into(),
            build_context: "test".into(),
        },
    );
    let info = app
        .core_info(VersionedRequest {
            contract_version: CONTRACT_VERSION,
        })
        .unwrap();
    assert_eq!(info.core_version, lab_engine::VERSION);
    assert_eq!(info.execution, "manual-and-continuous");
    let value = serde_json::to_value(&info).unwrap();
    assert_eq!(value["contractVersion"], CONTRACT_VERSION);
    let error = app
        .core_info(VersionedRequest {
            contract_version: CONTRACT_VERSION - 1,
        })
        .unwrap_err();
    assert_eq!(error.code, "APP-CONTRACT-VERSION");
}

#[test]
fn core_modules_are_linkable_without_a_desktop_host() {
    for version in [
        plc_types::VERSION,
        plc_ir::VERSION,
        plc_runtime::VERSION,
        simulation_core::VERSION,
        trace_core::VERSION,
        network_core::VERSION,
        grading_core::VERSION,
        st_frontend::VERSION,
        lab_model::VERSION,
        hardware_model::VERSION,
        process_simulation::VERSION,
        lab_engine::VERSION,
        knowledge_core::VERSION,
        project_model::VERSION,
    ] {
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }
}

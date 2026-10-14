use app_contract::{CONTRACT_VERSION, CoreInfoRequest, ExecutionAvailability, get_core_info};

#[test]
fn backend_reports_real_core_package_version_and_unavailable_execution() {
    let info = get_core_info(
        CoreInfoRequest {
            contract_version: CONTRACT_VERSION,
        },
        "0.1.0",
    );
    let info = info.unwrap();
    assert_eq!(info.core_version, plc_runtime::VERSION);
    assert_eq!(info.execution, ExecutionAvailability::Unavailable);
    let value = serde_json::to_value(info).unwrap();
    assert_eq!(value["contractVersion"], CONTRACT_VERSION);
    assert_eq!(value["execution"], "unavailable");
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
    ] {
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }
}

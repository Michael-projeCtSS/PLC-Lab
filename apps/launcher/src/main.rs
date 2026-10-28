//! `PLC Lab.exe`: compatibility alias in the installation root. It starts the staged
//! application next to it and exits; `Launcher.exe` is the primary entry point with status,
//! projects and diagnostics. Paths are resolved from the executable's own location.
#![cfg_attr(windows, windows_subsystem = "windows")]

use plc_lab_launcher::{Installation, start};

fn main() {
    let result = Installation::of_running_launcher().and_then(|i| start(&i, None));
    if let Err(message) = result {
        report(&message);
        std::process::exit(1);
    }
}

/// Show the problem in a native message box (and on stderr for scripted use).
fn report(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    if std::env::var_os("PLC_LAB_LAUNCHER_NO_DIALOG").is_none() {
        use winsafe::prelude::Handle;
        let _ = winsafe::HWND::NULL.MessageBox(message, "PLC Lab", winsafe::co::MB::ICONERROR);
    }
}

//! `PLC Lab.exe` in the repository (or distribution) root.
//!
//! Its only job is to start the staged release application that sits next to it in
//! `releases/current/`. It resolves paths from its own location (never from the current
//! working directory), opens no console window, needs no administrator rights, does
//! not use the network and contains no PLC Lab product logic.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// The staged application, relative to the launcher's folder.
const APPLICATION: [&str; 3] = ["releases", "current", APPLICATION_FILE];
#[cfg(windows)]
const APPLICATION_FILE: &str = "PLC Lab Desktop.exe";
#[cfg(not(windows))]
const APPLICATION_FILE: &str = "PLC Lab Desktop";

fn application_path(launcher_dir: &Path) -> PathBuf {
    APPLICATION
        .iter()
        .fold(launcher_dir.to_path_buf(), |path, part| path.join(part))
}

fn main() {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    else {
        report("PLC Lab could not determine the folder it was started from.");
        std::process::exit(1);
    };
    let application = application_path(&dir);
    if !application.is_file() {
        report(&format!(
            "PLC Lab has not been built yet.\n\nExpected the application at:\n{}\n\nRun BUILD_PLC_LAB_RELEASE.bat in this folder first.",
            application.display()
        ));
        std::process::exit(1);
    }
    let working_dir = application.parent().unwrap_or(&dir).to_path_buf();
    if let Err(error) = Command::new(&application).current_dir(working_dir).spawn() {
        report(&format!(
            "PLC Lab could not be started:\n{}\n\n{error}",
            application.display()
        ));
        std::process::exit(1);
    }
}

/// Show a native message box. The text is passed through an environment variable so it
/// is never interpreted as a command.
fn report(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let system = std::env::var_os("SystemRoot")
            .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        let _ = Command::new(system.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"))
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Add-Type -AssemblyName PresentationFramework; [void][System.Windows.MessageBox]::Show($env:PLC_LAB_LAUNCHER_MESSAGE, 'PLC Lab', 'OK', 'Error')",
            ])
            .env("PLC_LAB_LAUNCHER_MESSAGE", message)
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_application_is_resolved_relative_to_the_launcher() {
        let path = application_path(Path::new("base"));
        assert_eq!(
            path,
            Path::new("base")
                .join("releases")
                .join("current")
                .join(APPLICATION_FILE)
        );
    }
}

//! The release's data next to `PLC Lab.exe` (`resources/`, ADR-0030) and startup problems
//! that must be shown before the window exists. Nothing here parses or executes launch
//! arguments (see `startup`).
use std::path::{Path, PathBuf};

/// The release's data folder next to the executable.
pub const RESOURCES: &str = "resources";
/// Knowledge packs shipped with the release, inside [`RESOURCES`].
pub const KNOWLEDGE_PACKS: &str = "knowledge-packs";

/// The folder that contains the running executable; resources are found from here, never
/// from the working directory.
pub fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
}

/// Knowledge packs shipped next to the executable.
pub fn shipped_packs(executable_dir: &Path) -> PathBuf {
    executable_dir.join(RESOURCES).join(KNOWLEDGE_PACKS)
}

/// A release reads its knowledge pack from `resources/knowledge-packs` next to the
/// executable. Without it PLC Lab has no lessons, machines or labs: say so instead of
/// starting an empty application.
pub fn check_resources(executable_dir: &Path) -> Result<(), String> {
    let packs = shipped_packs(executable_dir);
    let found = std::fs::read_dir(&packs).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.path().join("pack.json").is_file())
    });
    if found {
        Ok(())
    } else {
        Err(format!(
            "PLC Lab cannot find its knowledge pack.\n\nExpected it in:\n{}\n\nKeep the resources folder next to PLC Lab.exe (copy the whole PLC Lab folder), or run BUILD_PLC_LAB_RELEASE.bat again.",
            packs.display()
        ))
    }
}

/// The text shown when the application cannot start, with the WebView2 hint when the
/// runtime is not installed (the most common reason a window cannot be created).
pub fn failure_text(detail: &str) -> String {
    let mut text = format!("PLC Lab could not start.\n\n{detail}");
    if !webview2_installed() {
        text.push_str(
            "\n\nThe Microsoft Edge WebView2 Runtime was not found. PLC Lab needs it to show its window; install it from Microsoft (it is preinstalled on current Windows).",
        );
    }
    text
}

/// Show a startup problem in a native message box (and on stderr for scripted use): a
/// windowed release has no console, and the view cannot report it. `PLC_LAB_NO_DIALOG`
/// suppresses the box for unattended tests.
pub fn report_problem(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    if std::env::var_os("PLC_LAB_NO_DIALOG").is_none() {
        use winsafe::prelude::Handle;
        let _ = winsafe::HWND::NULL.MessageBox(message, "PLC Lab", winsafe::co::MB::ICONERROR);
    }
}

/// The installed Microsoft Edge WebView2 Runtime, as documented by Microsoft: the `pv`
/// value of the runtime's EdgeUpdate client key (machine-wide or per user).
#[cfg(windows)]
fn webview2_installed() -> bool {
    use winsafe::{self as w, co};
    const KEY: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let candidates = [
        (
            w::HKEY::LOCAL_MACHINE,
            format!(r"SOFTWARE\WOW6432Node\{KEY}"),
        ),
        (w::HKEY::LOCAL_MACHINE, format!(r"SOFTWARE\{KEY}")),
        (w::HKEY::CURRENT_USER, format!(r"Software\{KEY}")),
    ];
    candidates.into_iter().any(|(hive, key)| {
        matches!(
            hive.RegGetValue(Some(&key), Some("pv"), co::RRF::RT_REG_SZ),
            Ok(w::RegistryValue::Sz(version)) if !version.is_empty() && version != "0.0.0.0"
        )
    })
}

#[cfg(not(windows))]
fn webview2_installed() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_without_its_knowledge_pack_is_reported_with_the_expected_folder() {
        let dir = std::env::temp_dir().join(format!(
            "plc-lab-resources-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let problem = check_resources(&dir).unwrap_err();
        assert!(
            problem.contains(&shipped_packs(&dir).display().to_string()),
            "{problem}"
        );
        let pack = shipped_packs(&dir).join("PLC_Lab_Knowledge_Pack_v1");
        std::fs::create_dir_all(&pack).unwrap();
        assert!(check_resources(&dir).is_err(), "a folder without pack.json");
        std::fs::write(pack.join("pack.json"), "{}").unwrap();
        assert_eq!(check_resources(&dir), Ok(()));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

//! PLC Lab launcher logic: where the staged release lives (always relative to the
//! launcher's own folder), what state the installation is in, and how the application is
//! started. Distribution infrastructure only: no PLC, simulation, grading or knowledge
//! semantics live here; knowledge packs and projects are inspected through their own
//! crates.
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

/// The canonical product version (the Cargo workspace version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Application identifier; the data folder is named after it (see the Tauri host).
pub const IDENTIFIER: &str = "dev.plclab.desktop";
#[cfg(windows)]
pub const APPLICATION_FILE: &str = "PLC Lab Desktop.exe";
#[cfg(not(windows))]
pub const APPLICATION_FILE: &str = "PLC Lab Desktop";
/// The only argument the application accepts from a launch.
pub const OPEN_PROJECT: &str = "--open-project";

/// The folder that contains the launcher (`Launcher.exe` or `PLC Lab.exe`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub root: PathBuf,
}

impl Installation {
    /// The installation the running launcher belongs to. Never the working directory.
    pub fn of_running_launcher() -> Result<Self, String> {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .map(|root| Self { root })
            .ok_or_else(|| "PLC Lab could not determine the folder it was started from.".into())
    }
    pub fn release_dir(&self) -> PathBuf {
        self.root.join("releases").join("current")
    }
    pub fn application(&self) -> PathBuf {
        self.release_dir().join(APPLICATION_FILE)
    }
    pub fn manifest(&self) -> PathBuf {
        self.release_dir().join("release-manifest.json")
    }
    pub fn bundled_packs(&self) -> PathBuf {
        self.release_dir().join("knowledge-packs")
    }
}

/// The application data folder: `PLC_LAB_DATA_DIR` when set (portable installations and
/// tests), otherwise `%APPDATA%\dev.plclab.desktop` as used by the application.
pub fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("PLC_LAB_DATA_DIR") {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
        _ => std::env::var_os("APPDATA")
            .filter(|d| !d.is_empty())
            .map(|d| PathBuf::from(d).join(IDENTIFIER)),
    }
}

/// Outcome of one installation check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Ok(String),
    Warning(String),
    Missing(String),
}

impl Check {
    pub fn is_ok(&self) -> bool {
        matches!(self, Check::Ok(_))
    }
    pub fn text(&self) -> &str {
        match self {
            Check::Ok(t) | Check::Warning(t) | Check::Missing(t) => t,
        }
    }
    fn mark(&self) -> &'static str {
        match self {
            Check::Ok(_) => "OK",
            Check::Warning(_) => "WARNING",
            Check::Missing(_) => "MISSING",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackInfo {
    pub title: String,
    pub version: String,
    pub status: String,
    pub files: usize,
    pub path: PathBuf,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentInfo {
    pub name: String,
    pub file: PathBuf,
    pub exists: bool,
}

/// Everything the launcher shows about the installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub application: Check,
    pub release_version: Option<String>,
    pub manifest: Check,
    pub knowledge: Check,
    pub packs: Vec<PackInfo>,
    pub webview2: Check,
    pub data: Check,
    pub data_dir: Option<PathBuf>,
    pub recent: Option<RecentInfo>,
    /// A PLC Lab application is already open; starting it again brings that window forward.
    pub running: bool,
}

impl Status {
    /// Whether the application can be started at all.
    pub fn startable(&self) -> bool {
        self.application.is_ok()
    }
    /// One line for the status display.
    pub fn summary(&self) -> String {
        if !self.application.is_ok() {
            return "Not installed: the application release is missing.".into();
        }
        for check in [&self.webview2, &self.knowledge, &self.manifest, &self.data] {
            if !check.is_ok() {
                return format!("Ready with warnings: {}", check.text());
            }
        }
        if self.running {
            return "PLC Lab is already running: Start brings its window to the front.".into();
        }
        "Ready".into()
    }
}

/// Inspect the installation next to the launcher and the user's data folder.
pub fn inspect(installation: &Installation, data_dir: Option<&Path>) -> Status {
    let application = installation.application();
    let application = if application.is_file() {
        Check::Ok(format!("Application found: {}", application.display()))
    } else {
        Check::Missing(format!(
            "The application is missing: {}. Run BUILD_PLC_LAB_RELEASE.bat in this folder.",
            application.display()
        ))
    };
    let (manifest, release_version) = read_manifest(&installation.manifest());
    let mut roots = vec![installation.bundled_packs()];
    if let Some(dir) = data_dir {
        roots.push(dir.join("knowledge-packs"));
    }
    let packs = knowledge_packs(&roots);
    let knowledge = if packs.is_empty() {
        Check::Missing("No knowledge pack is installed.".into())
    } else if packs.iter().any(|p| p.status != "loaded") {
        Check::Warning("A knowledge pack has problems; see Diagnostics.".into())
    } else {
        Check::Ok(format!("{} knowledge pack(s) available.", packs.len()))
    };
    let data = match data_dir {
        None => Check::Missing("The data folder location is unknown (APPDATA is not set).".into()),
        Some(dir) => match std::fs::create_dir_all(dir.join("projects")) {
            Ok(()) => Check::Ok(format!(
                "Project folder: {}",
                dir.join("projects").display()
            )),
            Err(e) => Check::Missing(format!(
                "The project folder {} cannot be created: {e}",
                dir.join("projects").display()
            )),
        },
    };
    let recent = data_dir.and_then(recent_project);
    Status {
        application,
        release_version,
        manifest,
        knowledge,
        packs,
        webview2: webview2(),
        data,
        data_dir: data_dir.map(Path::to_path_buf),
        recent,
        running: application_running(),
    }
}

/// Whether a PLC Lab application runs in this session: its single-instance plugin keeps a
/// hidden window named after the application identifier (`<id>-sic` / `<id>-siw`). Only a
/// status hint; the application itself guarantees that one instance runs.
#[cfg(windows)]
pub fn application_running() -> bool {
    use winsafe::{AtomStr, HWND};
    HWND::FindWindow(
        Some(AtomStr::from_str(&format!("{IDENTIFIER}-sic"))),
        Some(&format!("{IDENTIFIER}-siw")),
    )
    .is_ok_and(|w| w.is_some())
}
#[cfg(not(windows))]
pub fn application_running() -> bool {
    false
}

fn read_manifest(path: &Path) -> (Check, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            return (
                Check::Missing(format!("Release manifest not found: {}", path.display())),
                None,
            );
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return (
                Check::Warning(format!("The release manifest is unreadable: {e}")),
                None,
            );
        }
    };
    match value.get("appVersion").and_then(|v| v.as_str()) {
        Some(version) => (
            Check::Ok(format!("Release {version}")),
            Some(version.to_owned()),
        ),
        None => (
            Check::Warning("The release manifest has no application version.".into()),
            None,
        ),
    }
}

fn knowledge_packs(roots: &[PathBuf]) -> Vec<PackInfo> {
    let existing: Vec<PathBuf> = roots.iter().filter(|r| r.is_dir()).cloned().collect();
    let library = knowledge_core::KnowledgeLibrary::load(&existing);
    library
        .packs()
        .iter()
        .map(|p| PackInfo {
            title: p
                .manifest
                .as_ref()
                .map_or_else(|| "Unnamed knowledge pack".into(), |m| m.title.clone()),
            version: p
                .manifest
                .as_ref()
                .map_or_else(String::new, |m| m.version.clone()),
            status: match p.status {
                knowledge_core::PackStatus::Loaded => "loaded",
                knowledge_core::PackStatus::Partial => "partially loaded",
                knowledge_core::PackStatus::Invalid => "invalid",
            }
            .into(),
            files: count_files(Path::new(&p.path)),
            path: PathBuf::from(&p.path),
            problems: p.errors.iter().chain(&p.warnings).cloned().collect(),
        })
        .collect()
}

fn count_files(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .map(|e| {
                let path = e.path();
                match std::fs::symlink_metadata(&path) {
                    Ok(m) if m.is_dir() => count_files(&path),
                    Ok(m) if m.is_file() => 1,
                    _ => 0,
                }
            })
            .sum()
    })
}

fn recent_project(data: &Path) -> Option<RecentInfo> {
    let recent = project_model::RecentProjects::load(&data.join("recent.json"));
    let first = recent.projects.first()?;
    let file = project_model::ProjectStore::new(data.join("projects")).file_of(&first.id);
    Some(RecentInfo {
        name: first.name.clone(),
        exists: file.is_file(),
        file,
    })
}

/// The installed Microsoft Edge WebView2 Runtime, as documented by Microsoft: the `pv`
/// value of the runtime's EdgeUpdate client key (machine-wide or per user).
#[cfg(windows)]
fn webview2() -> Check {
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
    for (hive, key) in candidates {
        if let Ok(w::RegistryValue::Sz(version)) =
            hive.RegGetValue(Some(&key), Some("pv"), co::RRF::RT_REG_SZ)
            && !version.is_empty()
            && version != "0.0.0.0"
        {
            return Check::Ok(format!("WebView2 Runtime {version}"));
        }
    }
    Check::Missing(
        "The Microsoft Edge WebView2 Runtime was not found. Install it from Microsoft before starting PLC Lab.".into(),
    )
}

#[cfg(not(windows))]
fn webview2() -> Check {
    Check::Warning("WebView2 applies to Windows only.".into())
}

/// Validate a project file before handing it to the application: a `.plclab` file of at
/// most the project size limit that parses as a supported PLC Lab project.
pub fn check_project_file(path: &Path) -> Result<(), String> {
    let shown = path.display();
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(project_model::EXTENSION))
    {
        return Err(format!("{shown} is not a PLC Lab project (.plclab) file."));
    }
    let meta = std::fs::metadata(path).map_err(|_| format!("{shown} no longer exists."))?;
    if !meta.is_file() {
        return Err(format!("{shown} is not a file."));
    }
    if meta.len() > project_model::MAX_PROJECT_BYTES {
        return Err(format!("{shown} is too large to be a PLC Lab project."));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{shown} cannot be read: {e}"))?;
    project_model::parse(&text)
        .map(|_| ())
        .map_err(|e| format!("{shown} is not a valid PLC Lab project: {e}"))
}

/// Start the application, optionally asking it to open `project`. Arguments are passed
/// as separate process arguments; no shell is involved.
pub fn start(installation: &Installation, project: Option<&Path>) -> Result<(), String> {
    let application = installation.application();
    if !application.is_file() {
        return Err(format!(
            "PLC Lab has not been built yet.\n\nExpected the application at:\n{}\n\nRun BUILD_PLC_LAB_RELEASE.bat in this folder first.",
            application.display()
        ));
    }
    let mut command = Command::new(&application);
    command.current_dir(installation.release_dir());
    if let Some(project) = project {
        let absolute = std::fs::canonicalize(project)
            .map_err(|e| format!("{} cannot be opened: {e}", project.display()))?;
        command.arg(OPEN_PROJECT).arg(absolute);
    }
    command.spawn().map(|_| ()).map_err(|e| {
        format!(
            "PLC Lab could not be started:\n{}\n\n{e}",
            application.display()
        )
    })
}

/// Plain-text diagnostics for support requests. Nothing is sent anywhere.
pub fn diagnostics(installation: &Installation, status: &Status) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "PLC Lab launcher diagnostics");
    let _ = writeln!(out, "Launcher version: {VERSION}");
    let _ = writeln!(
        out,
        "Release version: {}",
        status.release_version.as_deref().unwrap_or("unknown")
    );
    if let Some(release) = &status.release_version
        && release != VERSION
    {
        let _ = writeln!(
            out,
            "Note: the launcher and the release have different versions; rebuild the release."
        );
    }
    let _ = writeln!(out, "Installation folder: {}", installation.root.display());
    let _ = writeln!(out, "Application: {}", installation.application().display());
    let _ = writeln!(
        out,
        "Application running: {}",
        if status.running { "yes" } else { "no" }
    );
    let _ = writeln!(
        out,
        "Data folder: {}",
        status
            .data_dir
            .as_deref()
            .map_or_else(|| "unknown".into(), |d| d.display().to_string())
    );
    let _ = writeln!(out);
    for (name, check) in [
        ("Application", &status.application),
        ("Release manifest", &status.manifest),
        ("Knowledge", &status.knowledge),
        ("WebView2", &status.webview2),
        ("Project folder", &status.data),
    ] {
        let _ = writeln!(out, "[{}] {name}: {}", check.mark(), check.text());
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "Knowledge packs:");
    if status.packs.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for p in &status.packs {
        let _ = writeln!(
            out,
            "  {} {} - {} - {} files - {}",
            p.title,
            p.version,
            p.status,
            p.files,
            p.path.display()
        );
        for problem in &p.problems {
            let _ = writeln!(out, "    {problem}");
        }
    }
    let _ = writeln!(out);
    match &status.recent {
        Some(r) => {
            let _ = writeln!(
                out,
                "Recent project: {} ({}{})",
                r.name,
                r.file.display(),
                if r.exists { "" } else { ", missing" }
            );
        }
        None => {
            let _ = writeln!(out, "Recent project: none");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("plclab-launcher {name} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn paths_are_relative_to_the_launcher_folder() {
        let i = Installation {
            root: PathBuf::from("X:/some where/PLC Lab"),
        };
        assert_eq!(
            i.application(),
            PathBuf::from("X:/some where/PLC Lab")
                .join("releases")
                .join("current")
                .join(APPLICATION_FILE)
        );
        assert!(i.bundled_packs().ends_with("knowledge-packs"));
    }

    #[test]
    fn a_missing_release_is_reported_and_not_startable() {
        let root = temp("missing");
        let data = root.join("data");
        let i = Installation { root: root.clone() };
        let status = inspect(&i, Some(&data));
        assert!(!status.startable());
        assert!(status.summary().starts_with("Not installed"));
        assert!(matches!(status.manifest, Check::Missing(_)));
        assert!(matches!(status.knowledge, Check::Missing(_)));
        assert!(status.data.is_ok(), "the project folder is created");
        let error = start(&i, None).unwrap_err();
        assert!(error.contains("BUILD_PLC_LAB_RELEASE.bat"), "{error}");
        let text = diagnostics(&i, &status);
        assert!(text.contains("[MISSING] Application"), "{text}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_bundled_pack_and_a_corrupt_manifest_are_reported() {
        let root = temp("pack");
        let i = Installation { root: root.clone() };
        std::fs::create_dir_all(i.release_dir()).unwrap();
        std::fs::write(i.application(), b"stand-in").unwrap();
        std::fs::write(i.manifest(), "{ not json").unwrap();
        let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../knowledge/PLC_Lab_Knowledge_Pack_v1");
        let target = i.bundled_packs().join("PLC_Lab_Knowledge_Pack_v1");
        copy_dir(&pack, &target);
        let status = inspect(&i, Some(&root.join("data")));
        assert!(status.startable());
        assert!(
            matches!(status.manifest, Check::Warning(_)),
            "{:?}",
            status.manifest
        );
        assert_eq!(status.packs.len(), 1);
        assert_eq!(status.packs[0].status, "loaded");
        assert_eq!(status.packs[0].files, count_files(&pack));
        assert!(status.knowledge.is_ok());
        std::fs::write(i.manifest(), r#"{"appVersion": "9.9.9"}"#).unwrap();
        let status = inspect(&i, Some(&root.join("data")));
        assert_eq!(status.release_version.as_deref(), Some("9.9.9"));
        assert!(diagnostics(&i, &status).contains("different versions"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn project_files_are_checked_before_they_are_handed_over() {
        let root = temp("projects");
        let txt = root.join("notes.txt");
        std::fs::write(&txt, "{}").unwrap();
        assert!(
            check_project_file(&txt)
                .unwrap_err()
                .contains("not a PLC Lab project")
        );
        let missing = root.join("gone.plclab");
        assert!(
            check_project_file(&missing)
                .unwrap_err()
                .contains("no longer exists")
        );
        let broken = root.join("broken.plclab");
        std::fs::write(&broken, "{ nope").unwrap();
        assert!(
            check_project_file(&broken)
                .unwrap_err()
                .contains("not a valid")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recent_project_comes_from_the_trusted_list() {
        let root = temp("recent");
        let mut recent = project_model::RecentProjects::default();
        let id = lab_model_id("p-1");
        recent.record(&id, "Bench", "2026-10-28T00:00:00Z");
        recent.save(&root.join("recent.json")).unwrap();
        let info = recent_project(&root).unwrap();
        assert_eq!(info.name, "Bench");
        assert!(!info.exists);
        assert!(info.file.ends_with("p-1.plclab"));
        let _ = std::fs::remove_dir_all(&root);
    }

    fn lab_model_id(s: &str) -> lab_model::Identifier {
        lab_model::Identifier::new(s).unwrap()
    }

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let target = to.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy_dir(&e.path(), &target);
            } else {
                std::fs::copy(e.path(), target).unwrap();
            }
        }
    }
}

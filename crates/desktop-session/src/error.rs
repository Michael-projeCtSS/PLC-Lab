//! Typed application errors. Use cases decide behavior on these variants; transport codes
//! are produced only when an error crosses the IPC boundary (`From<AppError> for IpcError`).

use app_contract::IpcError;
use lab_engine::EngineError;
use project_model::ProjectError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The request was written for another IPC contract version.
    ContractVersion(u32),
    /// The workspace identifier does not name the open workspace.
    StaleWorkspace,
    NotFound {
        what: &'static str,
        id: String,
    },
    InvalidRequest(String),
    /// The operation needs a paused or stopped simulation.
    SimulationRunning,
    /// The workspace failed closed after an internal engine error.
    WorkspaceFailed(String),
    Engine(EngineError),
    Project(ProjectError),
    /// The application state lock is poisoned.
    Unavailable,
}

impl From<EngineError> for AppError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}
impl From<ProjectError> for AppError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

fn engine_code(e: &EngineError) -> &'static str {
    match e {
        EngineError::Compile(_) => "PLC-COMPILE",
        EngineError::Binding(_) => "PLC-BINDING",
        EngineError::Hardware(_) => "PLC-HARDWARE",
        EngineError::Scene(_) => "LAB-SCENE",
        EngineError::NoProgram => "PLC-NO-PROGRAM",
        EngineError::PlcFaulted => "PLC-FAULTED",
        EngineError::UnknownComponent(_) => "LAB-UNKNOWN-COMPONENT",
        EngineError::UnknownSignal(_) => "LAB-UNKNOWN-SIGNAL",
        EngineError::UnknownChannel(_) => "LAB-UNKNOWN-CHANNEL",
        EngineError::InvalidAction(_) => "LAB-INVALID-ACTION",
        EngineError::Internal(_) => "APP-INTERNAL",
    }
}

impl From<AppError> for IpcError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::ContractVersion(v) => IpcError::new(
                "APP-CONTRACT-VERSION",
                format!(
                    "IPC contract version {v} is not supported; expected {}.",
                    app_contract::CONTRACT_VERSION
                ),
            ),
            AppError::StaleWorkspace => IpcError::new(
                "APP-WORKSPACE-STALE",
                "This workspace is no longer open. Reopen the project.",
            ),
            AppError::NotFound { what, id } => {
                IpcError::new("APP-NOT-FOUND", format!("The {what} '{id}' was not found."))
            }
            AppError::InvalidRequest(m) => IpcError::new("APP-INVALID-REQUEST", m),
            AppError::SimulationRunning => IpcError::new(
                "APP-SIMULATION-RUNNING",
                "Pause the simulation before performing this action.",
            ),
            AppError::WorkspaceFailed(m) => IpcError::new(
                "APP-WORKSPACE-FAILED",
                format!(
                    "The simulation stopped after an internal error and was closed safely: {m}"
                ),
            ),
            AppError::Engine(e) => IpcError::new(engine_code(&e), e.to_string()),
            AppError::Project(e) => IpcError::new(
                match e {
                    ProjectError::Parse(_) => "APP-PROJECT-PARSE",
                    ProjectError::UnsupportedVersion(_) => "APP-PROJECT-VERSION",
                    ProjectError::Invalid(_) => "APP-PROJECT-INVALID",
                    ProjectError::Io(_) => "APP-PROJECT-IO",
                    ProjectError::NotFound(_) => "APP-NOT-FOUND",
                },
                e.to_string(),
            ),
            AppError::Unavailable => IpcError::new(
                "APP-UNAVAILABLE",
                "The application state is unavailable after an internal failure. Restart PLC Lab.",
            ),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

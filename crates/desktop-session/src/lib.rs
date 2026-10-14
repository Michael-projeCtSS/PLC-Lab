//! Desktop application orchestration. No host/UI dependency or PLC semantics.
use app_contract::*;
use plc_runtime::{FunctionBlockStateSnapshot, MemoryRole, Runtime, RuntimeConfig, RuntimeState};
use plc_types::{PlcTime, diagnostics::RelatedIdentity, identity::SymbolId};
use std::sync::{Mutex, MutexGuard};

pub const DEFAULT_CYCLE_MICROS: u64 = 100_000;
pub const DEFAULT_OPERATION_BUDGET: u64 = 10_000;

#[derive(Default)]
pub struct RuntimeSessionService {
    inner: Mutex<SessionSlot>,
}
#[derive(Default)]
struct SessionSlot {
    generation: u64,
    loaded: Option<LoadedSession>,
}
struct LoadedSession {
    id: RuntimeSessionId,
    program_name: String,
    runtime: Runtime,
    config: RuntimeConfig,
    variables: Vec<VariableDescriptor>,
    instance_names:
        std::collections::BTreeMap<plc_types::identity::FunctionBlockInstanceId, String>,
}
struct VariableDescriptor {
    symbol: SymbolId,
    name: String,
    role: RuntimeRole,
    type_name: plc_types::PlcType,
}

fn application_error(code: &str, message: impl Into<String>) -> IpcError {
    IpcError {
        code: code.into(),
        message: message.into(),
    }
}
fn version(value: u32) -> Result<(), IpcError> {
    if value == CONTRACT_VERSION {
        Ok(())
    } else {
        Err(application_error(
            "APP-IPC-001",
            format!("IPC contract version {value} is unsupported; expected {CONTRACT_VERSION}."),
        ))
    }
}
fn invariant() -> IpcError {
    application_error(
        "APP-RUNTIME-INTERNAL",
        "The runtime session could not produce a consistent observation.",
    )
}

impl RuntimeSessionService {
    fn lock(&self) -> Result<MutexGuard<'_, SessionSlot>, IpcError> {
        self.inner.lock().map_err(|_| {
            application_error(
                "APP-RUNTIME-LOCK",
                "The runtime session is unavailable after an application failure.",
            )
        })
    }
    pub fn load(&self, request: LoadStProgramRequest) -> Result<LoadProgramResponse, IpcError> {
        version(request.contract_version)?;
        let compiled = match st_frontend::compile_st(
            &request.source,
            st_frontend::CompilerConfig::default(),
        ) {
            Ok(compiled) => compiled,
            Err(errors) => {
                return Ok(LoadProgramResponse::CompileFailed {
                    diagnostics: errors.iter().map(CompilerDiagnostic::from).collect(),
                });
            }
        };
        let validated = compiled.validated().map_err(|_| invariant())?;
        let plan = plc_runtime::lower(&validated)
            .map_err(|e| application_error("APP-RUNTIME-LOAD", e.to_string()))?;
        let config = RuntimeConfig::new(
            PlcTime::from_micros(DEFAULT_CYCLE_MICROS),
            DEFAULT_OPERATION_BUDGET,
        )
        .map_err(|e| application_error("APP-RUNTIME-LOAD", e.to_string()))?;
        let variables = compiled
            .program()
            .symbols()
            .symbols()
            .filter_map(|symbol| {
                plan.memory_role(symbol.id())
                    .map(|role| VariableDescriptor {
                        symbol: symbol.id(),
                        name: symbol.name().into(),
                        type_name: symbol.plc_type(),
                        role: match role {
                            MemoryRole::ExternalInput => RuntimeRole::Input,
                            MemoryRole::ExternalOutput => RuntimeRole::Output,
                            MemoryRole::Internal => RuntimeRole::Local,
                        },
                    })
            })
            .collect();
        let instance_names = compiled
            .program()
            .function_blocks()
            .instances()
            .filter(|i| i.owner() == compiled.program().entry())
            .map(|i| (i.id(), i.name().into()))
            .collect();
        let mut candidate = LoadedSession {
            id: RuntimeSessionId::new(1).map_err(|_| invariant())?,
            program_name: compiled.program_name().into(),
            runtime: Runtime::new(plan, config),
            config,
            variables,
            instance_names,
        };
        // No session mutation until preparation has completely succeeded.
        let mut slot = self.lock()?;
        let generation = slot.generation.checked_add(1).ok_or_else(|| {
            application_error(
                "APP-RUNTIME-GENERATION",
                "The session generation range is exhausted.",
            )
        })?;
        candidate.id = RuntimeSessionId::new(generation).map_err(|_| invariant())?;
        let snapshot = candidate.snapshot()?;
        slot.generation = generation;
        slot.loaded = Some(candidate);
        Ok(LoadProgramResponse::Loaded {
            snapshot: Box::new(snapshot),
        })
    }
    fn with_session(
        &self,
        request: RuntimeSessionRequest,
        action: impl FnOnce(&mut LoadedSession) -> Result<RuntimeSnapshotDto, IpcError>,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        version(request.contract_version)?;
        let mut slot = self.lock()?;
        let session = slot.target(request.session_id)?;
        action(session)
    }
    pub fn snapshot(&self, request: RuntimeSessionRequest) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| s.snapshot())
    }
    pub fn set_input(
        &self,
        request: SetRuntimeInputRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        let target = RuntimeSessionRequest {
            contract_version: request.contract_version,
            session_id: request.session_id,
        };
        self.with_session(target, |s| {
            let symbol = SymbolId::new(request.variable_id.get()).map_err(|_| invariant())?;
            s.runtime
                .set_input(symbol, request.value.into())
                .map_err(|e| application_error("APP-RUNTIME-INPUT", e.to_string()))?;
            s.snapshot()
        })
    }
    pub fn single_scan(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| {
            // Faults are authoritative domain data in the post-attempt snapshot.
            match s.runtime.single_scan() {
                Ok(_)
                | Err(plc_runtime::ScanError::Fault(_))
                | Err(plc_runtime::ScanError::Internal(_))
                | Err(plc_runtime::ScanError::Faulted) => s.snapshot(),
            }
        })
    }
    pub fn reset(&self, request: RuntimeSessionRequest) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| {
            s.runtime.reset();
            s.snapshot()
        })
    }
    pub fn unload(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<UnloadProgramResponse, IpcError> {
        version(request.contract_version)?;
        let mut slot = self.lock()?;
        slot.target(request.session_id)?;
        slot.loaded = None;
        Ok(UnloadProgramResponse {
            contract_version: CONTRACT_VERSION,
            session_id: request.session_id,
        })
    }
}
impl SessionSlot {
    fn target(&mut self, id: RuntimeSessionId) -> Result<&mut LoadedSession, IpcError> {
        self.loaded.as_mut().filter(|s| s.id == id).ok_or_else(|| {
            application_error(
                "APP-RUNTIME-STALE-SESSION",
                "This program session was replaced or unloaded. Load the current source again.",
            )
        })
    }
}
impl LoadedSession {
    fn snapshot(&self) -> Result<RuntimeSnapshotDto, IpcError> {
        let observed = self.runtime.snapshot();
        let variables = self
            .variables
            .iter()
            .map(|v| {
                let values = if v.role == RuntimeRole::Input {
                    observed.staged_inputs()
                } else {
                    observed.committed()
                };
                Ok(RuntimeVariable {
                    variable_id: RuntimeVariableId::new(v.symbol.get()).map_err(|_| invariant())?,
                    name: v.name.clone(),
                    type_name: v.type_name,
                    role: v.role,
                    value: (*values.get(&v.symbol).ok_or_else(invariant)?).into(),
                })
            })
            .collect::<Result<Vec<_>, IpcError>>()?;
        let function_blocks = observed
            .function_blocks()
            .iter()
            .map(|(id, state)| {
                let name = self.instance_names.get(id).ok_or_else(invariant)?.clone();
                match state {
                    FunctionBlockStateSnapshot::Ton { q, et } => Ok(RuntimeFunctionBlock::Ton {
                        instance_id: RuntimeInstanceId::new(id.get()).map_err(|_| invariant())?,
                        name,
                        q: *q,
                        et_micros: *et,
                    }),
                }
            })
            .collect::<Result<Vec<_>, IpcError>>()?;
        let fault = observed.fault().map(|diagnostic| {
            let related = match diagnostic.related() {
                Some(RelatedIdentity::Symbol(id)) => variables
                    .iter()
                    .find(|v| v.variable_id.get() == id.get())
                    .map(|v| RuntimeRelatedVariable {
                        variable_id: v.variable_id,
                        name: v.name.clone(),
                    }),
                _ => None,
            };
            RuntimeDiagnostic::from_core(diagnostic, related)
        });
        Ok(RuntimeSnapshotDto {
            contract_version: CONTRACT_VERSION,
            session_id: self.id,
            program_name: self.program_name.clone(),
            state: match observed.state() {
                RuntimeState::Ready => SessionRuntimeState::Ready,
                RuntimeState::Faulted => SessionRuntimeState::Faulted,
            },
            completed_scans: ExactU64::new(observed.completed_scans()).map_err(|_| invariant())?,
            simulation_time_micros: observed.time(),
            cycle_period_micros: self.config.cycle_period(),
            operation_budget: ExactU64::new(self.config.execution_budget())
                .map_err(|_| invariant())?,
            variables,
            function_blocks,
            fault,
        })
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    fn request() -> LoadStProgramRequest {
        LoadStProgramRequest {
            contract_version: CONTRACT_VERSION,
            source: "PROGRAM Main END_PROGRAM".into(),
        }
    }
    #[test]
    fn exhausted_generation_preserves_loaded_session() {
        let service = RuntimeSessionService::default();
        let LoadProgramResponse::Loaded { snapshot } = service.load(request()).unwrap() else {
            panic!()
        };
        service.inner.lock().unwrap().generation = u64::MAX;
        assert_eq!(
            service.load(request()).unwrap_err().code,
            "APP-RUNTIME-GENERATION"
        );
        assert_eq!(
            service
                .snapshot(RuntimeSessionRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: snapshot.session_id
                })
                .unwrap(),
            *snapshot
        );
    }
    #[test]
    fn poisoned_mutex_fails_closed_as_application_error() {
        let service = RuntimeSessionService::default();
        let failure = std::panic::catch_unwind(|| {
            let _guard = service.inner.lock().unwrap();
            panic!("Test-only application failure.");
        });
        assert!(failure.is_err());
        assert_eq!(
            service.load(request()).unwrap_err().code,
            "APP-RUNTIME-LOCK"
        );
    }
}

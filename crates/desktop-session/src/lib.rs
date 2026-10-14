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
    latest_load_intent: u64,
    loaded: Option<LoadedSession>,
}
/// Backend-only request ordering, distinct from a committed runtime identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LoadIntentId(u64);
enum PreparedLoad {
    Ready(Box<LoadedSession>),
    CompileFailed(Vec<CompilerDiagnostic>),
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
        let intent = self.reserve_load_intent()?;
        match Self::prepare_load(&request.source)? {
            PreparedLoad::Ready(candidate) => self.commit_prepared_load(intent, candidate),
            PreparedLoad::CompileFailed(diagnostics) => {
                Ok(LoadProgramResponse::CompileFailed { diagnostics })
            }
        }
    }
    fn reserve_load_intent(&self) -> Result<LoadIntentId, IpcError> {
        let mut slot = self.lock()?;
        let next = slot.latest_load_intent.checked_add(1).ok_or_else(|| {
            application_error(
                "APP-RUNTIME-LOAD-INTENT-EXHAUSTED",
                "The program load intent range is exhausted.",
            )
        })?;
        slot.latest_load_intent = next;
        Ok(LoadIntentId(next))
    }
    // No service reference: expensive preparation cannot acquire its session lock.
    fn prepare_load(source: &str) -> Result<PreparedLoad, IpcError> {
        let compiled = match st_frontend::compile_st(source, st_frontend::CompilerConfig::default())
        {
            Ok(compiled) => compiled,
            Err(errors) => {
                return Ok(PreparedLoad::CompileFailed(
                    errors.iter().map(CompilerDiagnostic::from).collect(),
                ));
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
        let candidate = LoadedSession {
            id: RuntimeSessionId::new(1).map_err(|_| invariant())?,
            program_name: compiled.program_name().into(),
            runtime: Runtime::new(plan, config),
            config,
            variables,
            instance_names,
        };
        Ok(PreparedLoad::Ready(Box::new(candidate)))
    }
    fn commit_prepared_load(
        &self,
        intent: LoadIntentId,
        mut candidate: Box<LoadedSession>,
    ) -> Result<LoadProgramResponse, IpcError> {
        let mut slot = self.lock()?;
        // A newer intent supersedes this candidate even if its preparation failed.
        if intent.0 != slot.latest_load_intent {
            return Err(application_error(
                "APP-RUNTIME-STALE-LOAD",
                "A newer program load superseded this request. Load the current source again.",
            ));
        }
        let generation = slot.generation.checked_add(1).ok_or_else(|| {
            application_error(
                "APP-RUNTIME-GENERATION",
                "The session generation range is exhausted.",
            )
        })?;
        candidate.id = RuntimeSessionId::new(generation).map_err(|_| invariant())?;
        let snapshot = candidate.snapshot()?;
        slot.generation = generation;
        slot.loaded = Some(*candidate);
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
    fn prepared(name: &str) -> Box<LoadedSession> {
        let PreparedLoad::Ready(candidate) = RuntimeSessionService::prepare_load(&format!(
            "PROGRAM {name} VAR Count:INT; END_VAR Count := Count + 1; END_PROGRAM"
        ))
        .unwrap() else {
            panic!("Test source must prepare successfully")
        };
        candidate
    }
    fn loaded(response: LoadProgramResponse) -> RuntimeSnapshotDto {
        let LoadProgramResponse::Loaded { snapshot } = response else {
            panic!("Test source must load successfully")
        };
        *snapshot
    }
    fn target(snapshot: &RuntimeSnapshotDto) -> RuntimeSessionRequest {
        RuntimeSessionRequest {
            contract_version: CONTRACT_VERSION,
            session_id: snapshot.session_id,
        }
    }
    #[test]
    fn inverted_load_completion_rejects_older_intent_and_preserves_newest_session() {
        let service = RuntimeSessionService::default();
        let a = service.reserve_load_intent().unwrap();
        let b = service.reserve_load_intent().unwrap();
        assert_eq!(a, LoadIntentId(1));
        assert_eq!(b, LoadIntentId(2));
        let candidate_b = prepared("B");
        let newest = loaded(service.commit_prepared_load(b, candidate_b).unwrap());
        // A finishes preparing after B committed; no sleeps or timing assumptions.
        let candidate_a = prepared("A");
        assert_eq!(newest.program_name, "B");
        assert_eq!(newest.session_id.get(), 1);
        assert_eq!(
            service
                .commit_prepared_load(a, candidate_a)
                .unwrap_err()
                .code,
            "APP-RUNTIME-STALE-LOAD"
        );
        assert_eq!(service.snapshot(target(&newest)).unwrap(), newest);
        assert_eq!(service.inner.lock().unwrap().generation, 1);
        let c = service.reserve_load_intent().unwrap();
        let next = loaded(service.commit_prepared_load(c, prepared("C")).unwrap());
        assert_eq!(c, LoadIntentId(3));
        assert_eq!(next.session_id.get(), 2);
    }
    #[test]
    fn failed_newest_load_supersedes_older_intent_preserves_existing_and_allows_next_load() {
        let service = RuntimeSessionService::default();
        let x = service.reserve_load_intent().unwrap();
        let initial = loaded(service.commit_prepared_load(x, prepared("X")).unwrap());
        let before = service.single_scan(target(&initial)).unwrap();
        let a = service.reserve_load_intent().unwrap();
        let candidate_a = prepared("A");
        // Public Load must reserve B before real compilation reports failure.
        let failed = service
            .load(LoadStProgramRequest {
                contract_version: CONTRACT_VERSION,
                source: "PROGRAM B Missing := TRUE; END_PROGRAM".into(),
            })
            .unwrap();
        assert!(matches!(failed, LoadProgramResponse::CompileFailed { .. }));
        assert_eq!(service.inner.lock().unwrap().latest_load_intent, 3);
        assert_eq!(
            service
                .commit_prepared_load(a, candidate_a)
                .unwrap_err()
                .code,
            "APP-RUNTIME-STALE-LOAD"
        );
        assert_eq!(service.snapshot(target(&before)).unwrap(), before);
        let next = loaded(
            service
                .load(LoadStProgramRequest {
                    contract_version: CONTRACT_VERSION,
                    source: "PROGRAM C END_PROGRAM".into(),
                })
                .unwrap(),
        );
        assert_eq!(next.program_name, "C");
        assert_eq!(next.session_id.get(), 2);
        assert_eq!(service.inner.lock().unwrap().latest_load_intent, 4);
    }
    #[test]
    fn exhausted_load_intent_fails_closed_without_reusing_identity_or_mutating_session() {
        let service = RuntimeSessionService::default();
        let before = loaded(service.load(request()).unwrap());
        service.inner.lock().unwrap().latest_load_intent = u64::MAX;
        for _ in 0..2 {
            assert_eq!(
                service.load(request()).unwrap_err().code,
                "APP-RUNTIME-LOAD-INTENT-EXHAUSTED"
            );
            assert_eq!(service.snapshot(target(&before)).unwrap(), before);
            let slot = service.inner.lock().unwrap();
            assert_eq!(slot.latest_load_intent, u64::MAX);
            assert_eq!(slot.generation, 1);
        }
    }
    #[test]
    fn invalid_contract_version_does_not_reserve_or_supersede_valid_load_intent() {
        let service = RuntimeSessionService::default();
        let intent = service.reserve_load_intent().unwrap();
        let mut invalid = request();
        invalid.contract_version = CONTRACT_VERSION - 1;
        assert_eq!(service.load(invalid).unwrap_err().code, "APP-IPC-001");
        assert_eq!(service.inner.lock().unwrap().latest_load_intent, intent.0);
        let committed = loaded(service.commit_prepared_load(intent, prepared("A")).unwrap());
        assert_eq!(committed.session_id.get(), 1);
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

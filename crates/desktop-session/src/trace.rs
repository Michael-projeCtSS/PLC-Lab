//! Typed session history. No DTO is retained as canonical trace state.
use super::*;
use plc_types::{PlcType, PlcValue, diagnostics::Diagnostic, identity::FunctionBlockInstanceId};
use process_simulation::TankSnapshot;
use std::num::NonZeroUsize;
use trace_core::{TraceBuffer, TracePoint};

pub const TRACE_CAPACITY: u16 = 4096;
pub const TRACE_PAGE_MAX: u16 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventKind {
    SessionLoaded,
    InputStaged,
    ProcessAttached,
    ScanCompleted,
    ScanFaulted,
    RuntimeReset,
    ProcessReset,
    ProcessDetached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Authority {
    User,
    Process,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedVariable {
    id: SymbolId,
    name: String,
    kind: PlcType,
    role: MemoryRole,
    authority: Option<Authority>,
    value: PlcValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedBlock {
    id: FunctionBlockInstanceId,
    name: String,
    state: FunctionBlockStateSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Observation {
    point: TracePoint,
    state: RuntimeState,
    variables: Vec<ObservedVariable>,
    blocks: Vec<ObservedBlock>,
    fault: Option<Diagnostic>,
    process: Option<TankSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Event {
    kind: EventKind,
    staged_input: Option<SymbolId>,
    observation: Observation,
}
pub(super) fn recorder() -> TraceBuffer<Event> {
    TraceBuffer::new(
        NonZeroUsize::new(usize::from(TRACE_CAPACITY)).expect("Fixed trace capacity is positive."),
    )
}
fn sequence(id: trace_core::TraceSequenceId) -> TraceSequenceId {
    TraceSequenceId::new(id.get()).expect("Core trace identity is nonzero.")
}
impl Observation {
    fn capture(session: &LoadedSession) -> Result<Self, IpcError> {
        let core = session.runtime.snapshot();
        let variables = session
            .variables
            .iter()
            .map(|v| {
                let values = if v.role == RuntimeRole::Input {
                    core.staged_inputs()
                } else {
                    core.committed()
                };
                Ok(ObservedVariable {
                    id: v.symbol,
                    name: v.name.clone(),
                    kind: v.type_name,
                    role: match v.role {
                        RuntimeRole::Input => MemoryRole::ExternalInput,
                        RuntimeRole::Output => MemoryRole::ExternalOutput,
                        RuntimeRole::Local => MemoryRole::Internal,
                    },
                    authority: if v.role == RuntimeRole::Input {
                        Some(
                            if session.process.as_ref().is_some_and(|p| p.owns(v.symbol)) {
                                Authority::Process
                            } else {
                                Authority::User
                            },
                        )
                    } else {
                        None
                    },
                    value: *values.get(&v.symbol).ok_or_else(invariant)?,
                })
            })
            .collect::<Result<Vec<_>, IpcError>>()?;
        let blocks = core
            .function_blocks()
            .iter()
            .map(|(id, state)| {
                Ok(ObservedBlock {
                    id: *id,
                    name: session
                        .instance_names
                        .get(id)
                        .ok_or_else(invariant)?
                        .clone(),
                    state: *state,
                })
            })
            .collect::<Result<Vec<_>, IpcError>>()?;
        Ok(Self {
            point: TracePoint {
                completed_scans: core.completed_scans(),
                simulation_time: core.time(),
            },
            state: core.state(),
            variables,
            blocks,
            fault: core.fault().cloned(),
            process: session.process.as_ref().map(|p| p.observation()),
        })
    }
    fn state_dto(&self) -> SessionRuntimeState {
        match self.state {
            RuntimeState::Ready => SessionRuntimeState::Ready,
            RuntimeState::Faulted => SessionRuntimeState::Faulted,
        }
    }
    fn variables_dto(&self) -> Vec<RuntimeVariable> {
        self.variables
            .iter()
            .map(|v| RuntimeVariable {
                variable_id: RuntimeVariableId::new(v.id.get())
                    .expect("Symbol identity is nonzero."),
                name: v.name.clone(),
                type_name: v.kind,
                role: match v.role {
                    MemoryRole::ExternalInput => RuntimeRole::Input,
                    MemoryRole::ExternalOutput => RuntimeRole::Output,
                    MemoryRole::Internal => RuntimeRole::Local,
                },
                input_authority: v.authority.map(|a| match a {
                    Authority::User => InputAuthority::User,
                    Authority::Process => InputAuthority::Process,
                }),
                value: v.value.into(),
            })
            .collect()
    }
    fn blocks_dto(&self) -> Vec<RuntimeFunctionBlock> {
        self.blocks
            .iter()
            .map(|b| match b.state {
                FunctionBlockStateSnapshot::Ton { q, et } => RuntimeFunctionBlock::Ton {
                    instance_id: RuntimeInstanceId::new(b.id.get())
                        .expect("Instance identity is nonzero."),
                    name: b.name.clone(),
                    q,
                    et_micros: et,
                },
            })
            .collect()
    }
    fn related(&self, id: SymbolId) -> Option<RuntimeRelatedVariable> {
        self.variables
            .iter()
            .find(|v| v.id == id)
            .map(|v| RuntimeRelatedVariable {
                variable_id: RuntimeVariableId::new(v.id.get())
                    .expect("Symbol identity is nonzero."),
                name: v.name.clone(),
            })
    }
    fn fault_dto(&self) -> Option<RuntimeDiagnostic> {
        self.fault.as_ref().map(|d| {
            RuntimeDiagnostic::from_core(
                d,
                match d.related() {
                    Some(RelatedIdentity::Symbol(id)) => self.related(id),
                    _ => None,
                },
            )
        })
    }
    fn snapshot_dto(&self, session: &LoadedSession) -> RuntimeSnapshotDto {
        RuntimeSnapshotDto {
            contract_version: CONTRACT_VERSION,
            session_id: session.id,
            program_name: session.program_name.clone(),
            state: self.state_dto(),
            completed_scans: ExactU64::new(self.point.completed_scans)
                .expect("Every u64 is exact."),
            simulation_time_micros: self.point.simulation_time,
            cycle_period_micros: session.config.cycle_period(),
            operation_budget: ExactU64::new(session.config.execution_budget())
                .expect("Every u64 is exact."),
            variables: self.variables_dto(),
            function_blocks: self.blocks_dto(),
            fault: self.fault_dto(),
            process: self.process.map(process::present),
        }
    }
}
impl LoadedSession {
    pub(super) fn observe_snapshot(&self) -> Result<RuntimeSnapshotDto, IpcError> {
        Ok(Observation::capture(self)?.snapshot_dto(self))
    }
    pub(super) fn record(
        &mut self,
        kind: EventKind,
        staged_input: Option<SymbolId>,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        let observation = Observation::capture(self)?;
        let snapshot = observation.snapshot_dto(self);
        // Same mutex as the mutation. Saturation/eviction cannot fail execution.
        self.trace.append(
            observation.point,
            Event {
                kind,
                staged_input,
                observation,
            },
        );
        Ok(snapshot)
    }
}
impl RuntimeSessionService {
    pub fn trace_page(&self, request: TracePageRequest) -> Result<TracePageDto, IpcError> {
        version(request.contract_version)?;
        if request.limit == 0 || request.limit > TRACE_PAGE_MAX {
            return Err(application_error(
                "APP-TRACE-LIMIT",
                "Trace page limit must be between 1 and 256.",
            ));
        }
        let mut slot = self.lock()?;
        let session = slot.target(request.session_id)?;
        if session.process_failed {
            return Err(application_error(
                "APP-PROCESS-INTERNAL",
                "The process session is unavailable after an application failure. Load the program again.",
            ));
        }
        let page = session
            .trace
            .page(
                request
                    .after_sequence
                    .and_then(|s| trace_core::TraceSequenceId::new(s.get())),
                NonZeroUsize::new(usize::from(request.limit))
                    .expect("Validated page limit is positive."),
            )
            .map_err(|_| {
                application_error(
                    "APP-TRACE-CURSOR",
                    "The trace cursor is newer than this session history.",
                )
            })?;
        let entries = page
            .entries
            .iter()
            .map(|e| {
                let o = &e.payload.observation;
                TraceEntryDto {
                    sequence: sequence(e.sequence),
                    kind: match e.payload.kind {
                        EventKind::SessionLoaded => TraceEventKind::SessionLoaded,
                        EventKind::InputStaged => TraceEventKind::InputStaged,
                        EventKind::ProcessAttached => TraceEventKind::ProcessAttached,
                        EventKind::ScanCompleted => TraceEventKind::ScanCompleted,
                        EventKind::ScanFaulted => TraceEventKind::ScanFaulted,
                        EventKind::RuntimeReset => TraceEventKind::RuntimeReset,
                        EventKind::ProcessReset => TraceEventKind::ProcessReset,
                        EventKind::ProcessDetached => TraceEventKind::ProcessDetached,
                    },
                    completed_scans: ExactU64::new(e.point.completed_scans)
                        .expect("Every u64 is exact."),
                    simulation_time_micros: e.point.simulation_time,
                    runtime_state: o.state_dto(),
                    variables: o.variables_dto(),
                    function_blocks: o.blocks_dto(),
                    process: o.process.map(process::present),
                    fault: o.fault_dto(),
                    staged_input: e.payload.staged_input.and_then(|id| o.related(id)),
                }
            })
            .collect();
        Ok(TracePageDto {
            contract_version: CONTRACT_VERSION,
            session_id: session.id,
            entries,
            capacity: u16::try_from(page.metadata.capacity)
                .expect("Session trace capacity is bounded."),
            oldest_sequence: page.metadata.oldest_sequence.map(sequence),
            latest_sequence: page.metadata.latest_sequence.map(sequence),
            dropped_count: ExactU64::new(page.metadata.dropped_count).expect("Every u64 is exact."),
            saturated: page.metadata.saturated,
            gap: page.gap,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn loaded(s: &RuntimeSessionService) -> RuntimeSnapshotDto {
        let LoadProgramResponse::Loaded { snapshot } = s
            .load(LoadStProgramRequest {
                contract_version: CONTRACT_VERSION,
                source: include_str!("../../../scenes/tank-filling/example.st").into(),
            })
            .unwrap()
        else {
            panic!("Source must load")
        };
        *snapshot
    }
    fn request(id: RuntimeSessionId, after: Option<u64>) -> TracePageRequest {
        TracePageRequest {
            contract_version: CONTRACT_VERSION,
            session_id: id,
            after_sequence: after.map(|n| TraceSequenceId::new(n).unwrap()),
            limit: 256,
        }
    }
    #[test]
    fn newer_failed_load_preserves_trace_and_supersedes_older_prepared_candidate() {
        let s = RuntimeSessionService::default();
        let x = loaded(&s);
        s.single_scan(RuntimeSessionRequest {
            contract_version: CONTRACT_VERSION,
            session_id: x.session_id,
        })
        .unwrap();
        let before = s.trace_page(request(x.session_id, None)).unwrap();
        let a = s.reserve_load_intent().unwrap();
        let PreparedLoad::Ready(candidate) =
            RuntimeSessionService::prepare_load("PROGRAM A END_PROGRAM").unwrap()
        else {
            panic!("Candidate")
        };
        assert!(matches!(
            s.load(LoadStProgramRequest {
                contract_version: CONTRACT_VERSION,
                source: "invalid".into()
            })
            .unwrap(),
            LoadProgramResponse::CompileFailed { .. }
        ));
        assert_eq!(
            s.commit_prepared_load(a, candidate).unwrap_err().code,
            "APP-RUNTIME-STALE-LOAD"
        );
        assert_eq!(s.trace_page(request(x.session_id, None)).unwrap(), before);
    }
    #[test]
    fn small_retention_history_exposes_exact_cursor_gaps_without_altering_scans() {
        let s = RuntimeSessionService::default();
        let x = loaded(&s);
        {
            let mut slot = s.inner.lock().unwrap();
            let session = slot.loaded.as_mut().unwrap();
            session.trace = TraceBuffer::new(NonZeroUsize::new(3).unwrap());
            session.record(EventKind::SessionLoaded, None).unwrap();
        }
        let target = RuntimeSessionRequest {
            contract_version: CONTRACT_VERSION,
            session_id: x.session_id,
        };
        for _ in 0..5 {
            s.single_scan(target.clone()).unwrap();
        }
        let p = s.trace_page(request(x.session_id, Some(2))).unwrap();
        assert_eq!(p.capacity, 3);
        assert_eq!(p.dropped_count.get(), 3);
        assert_eq!(p.oldest_sequence.unwrap().get(), 4);
        assert_eq!(p.latest_sequence.unwrap().get(), 6);
        assert!(p.gap);
        assert_eq!(
            p.entries
                .iter()
                .map(|e| e.sequence.get())
                .collect::<Vec<_>>(),
            vec![4, 5, 6]
        );
        assert_eq!(p.entries[2].completed_scans.get(), 5);
        assert_eq!(p.entries[2].simulation_time_micros.as_micros(), 500000);
        assert!(!s.trace_page(request(x.session_id, Some(3))).unwrap().gap);
        assert!(
            s.trace_page(request(x.session_id, Some(6)))
                .unwrap()
                .entries
                .is_empty()
        );
    }
    #[test]
    fn incoherent_process_internal_failure_retains_history_without_fabricated_fault_event() {
        let s = RuntimeSessionService::default();
        let x = loaded(&s);
        s.attach_process(AttachProcessSceneRequest {
            contract_version: CONTRACT_VERSION,
            session_id: x.session_id,
            scene_kind: ProcessSceneKind::TankFilling,
        })
        .unwrap();
        let before = {
            let mut slot = s.inner.lock().unwrap();
            let session = slot.loaded.as_mut().unwrap();
            session.variables[0].symbol = SymbolId::new(999).unwrap();
            session.trace.metadata()
        };
        assert_eq!(
            s.attach_process(AttachProcessSceneRequest {
                contract_version: CONTRACT_VERSION,
                session_id: x.session_id,
                scene_kind: ProcessSceneKind::TankFilling
            })
            .unwrap_err()
            .code,
            "APP-PROCESS-INTERNAL"
        );
        assert_eq!(
            s.inner
                .lock()
                .unwrap()
                .loaded
                .as_ref()
                .unwrap()
                .trace
                .metadata(),
            before
        );
        assert_eq!(
            s.trace_page(request(x.session_id, None)).unwrap_err().code,
            "APP-PROCESS-INTERNAL"
        );
    }
}

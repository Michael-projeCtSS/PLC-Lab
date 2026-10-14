//! Application bindings only. Physical integration lives in process-simulation.
use super::*;
use plc_types::{PlcType, PlcValue};
use process_simulation::{TankActuators, TankFillingProcess};

pub(super) struct AttachedTank {
    bindings: TankBindings,
    model: TankFillingProcess,
}
struct TankBindings {
    low: SymbolId,
    high: SymbolId,
    percent: SymbolId,
    valve: SymbolId,
    pump: SymbolId,
}
fn internal() -> IpcError {
    application_error(
        "APP-PROCESS-INTERNAL",
        "The process session could not complete a consistent operation. Load the program again.",
    )
}
fn role_name(role: RuntimeRole) -> &'static str {
    match role {
        RuntimeRole::Input => "VAR_INPUT",
        RuntimeRole::Output => "VAR_OUTPUT",
        RuntimeRole::Local => "VAR",
    }
}
fn type_name(kind: PlcType) -> &'static str {
    match kind {
        PlcType::Bool => "BOOL",
        PlcType::Int => "INT",
        PlcType::Real => "REAL",
        PlcType::Time => "TIME",
    }
}
impl TankBindings {
    fn resolve(variables: &[VariableDescriptor]) -> Result<Self, IpcError> {
        let find = |name: &str, expected_type: PlcType, expected_role: RuntimeRole| {
            let expected = format!(
                "{name} : {} {}",
                type_name(expected_type),
                role_name(expected_role)
            );
            let found = variables
                .iter()
                .find(|v| v.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    application_error(
                        "PLC-PROCESS-001",
                        format!("Tank scene requires {expected}. The variable is missing."),
                    )
                })?;
            let description = format!(
                "{} : {} {}",
                found.name,
                type_name(found.type_name),
                role_name(found.role)
            );
            if found.type_name != expected_type {
                return Err(application_error(
                    "PLC-PROCESS-002",
                    format!("Tank scene requires {expected}. Found {description}."),
                ));
            }
            if found.role != expected_role {
                return Err(application_error(
                    "PLC-PROCESS-003",
                    format!("Tank scene requires {expected}. Found {description}."),
                ));
            }
            Ok(found.symbol)
        };
        Ok(Self {
            low: find("LevelLow", PlcType::Bool, RuntimeRole::Input)?,
            high: find("LevelHigh", PlcType::Bool, RuntimeRole::Input)?,
            percent: find("LevelPercent", PlcType::Real, RuntimeRole::Input)?,
            valve: find("InletValve", PlcType::Bool, RuntimeRole::Output)?,
            pump: find("DrainPump", PlcType::Bool, RuntimeRole::Output)?,
        })
    }
    fn preflight(&self, runtime: &Runtime) -> Result<(), IpcError> {
        let snapshot = runtime.snapshot();
        for (id, kind) in [
            (self.low, PlcType::Bool),
            (self.high, PlcType::Bool),
            (self.percent, PlcType::Real),
        ] {
            if snapshot.staged_inputs().get(&id).map(|v| v.plc_type()) != Some(kind) {
                return Err(internal());
            }
        }
        if [self.low, self.high, self.percent, self.valve, self.pump]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 5
        {
            return Err(internal());
        }
        self.actuators(runtime)?;
        Ok(())
    }
    fn actuators(&self, runtime: &Runtime) -> Result<TankActuators, IpcError> {
        let observed = runtime.snapshot();
        let read = |id| match observed.outputs().get(&id) {
            Some(PlcValue::Bool(value)) => Ok(*value),
            _ => Err(internal()),
        };
        Ok(TankActuators {
            inlet_valve: read(self.valve)?,
            drain_pump: read(self.pump)?,
        })
    }
}
impl AttachedTank {
    pub(super) fn prepare(
        runtime: &Runtime,
        variables: &[VariableDescriptor],
    ) -> Result<Self, IpcError> {
        let bindings = TankBindings::resolve(variables)?;
        bindings.preflight(runtime)?;
        let model = TankFillingProcess::new(bindings.actuators(runtime)?);
        Ok(Self { bindings, model })
    }
    pub(super) fn owns(&self, id: SymbolId) -> bool {
        [self.bindings.low, self.bindings.high, self.bindings.percent].contains(&id)
    }
    pub(super) fn restage(&self, runtime: &mut Runtime) -> Result<(), IpcError> {
        // All targets/types validated before any mutation. The exclusive runtime
        // borrow and session mutex keep the three bounded writes indivisible.
        self.bindings.preflight(runtime)?;
        let sensors = self.model.snapshot().sensors();
        for (id, value) in [
            (self.bindings.low, PlcValue::Bool(sensors.low_level)),
            (self.bindings.high, PlcValue::Bool(sensors.high_level)),
            (self.bindings.percent, PlcValue::Real(sensors.level_percent)),
        ] {
            runtime.set_input(id, value).map_err(|_| internal())?;
        }
        Ok(())
    }
    pub(super) fn advance(
        &mut self,
        runtime: &mut Runtime,
        cycle: PlcTime,
    ) -> Result<(), IpcError> {
        self.bindings.preflight(runtime)?;
        let actuators = self.bindings.actuators(runtime)?;
        self.model.step(cycle, actuators).map_err(|_| internal())?;
        self.restage(runtime)
    }
    pub(super) fn reset(&mut self, runtime: &mut Runtime) -> Result<(), IpcError> {
        self.bindings.preflight(runtime)?;
        self.model = TankFillingProcess::new(self.model.snapshot().actuators());
        self.restage(runtime)
    }
    pub(super) fn snapshot(&self) -> ProcessSnapshotDto {
        let s = self.model.snapshot();
        ProcessSnapshotDto::TankFilling(TankProcessSnapshotDto {
            level_permille: s.level_permille(),
            low_level: s.sensors().low_level,
            high_level: s.sensors().high_level,
            inlet_valve_open: s.actuators().inlet_valve,
            drain_pump_running: s.actuators().drain_pump,
            low_threshold_permille: process_simulation::LOW_THRESHOLD_PERMILLE,
            high_threshold_permille: process_simulation::HIGH_THRESHOLD_PERMILLE,
            fill_rate_permille_per_second: process_simulation::FILL_RATE_PERMILLE_PER_SECOND,
            drain_rate_permille_per_second: process_simulation::DRAIN_RATE_PERMILLE_PER_SECOND,
        })
    }
}

impl RuntimeSessionService {
    pub fn attach_process(
        &self,
        request: AttachProcessSceneRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(
            RuntimeSessionRequest {
                contract_version: request.contract_version,
                session_id: request.session_id,
            },
            |s| {
                if s.runtime.snapshot().state() != RuntimeState::Ready {
                    return Err(application_error(
                        "PLC-PROCESS-006",
                        "Reset the faulted PLC runtime before attaching a process scene.",
                    ));
                }
                match request.scene_kind {
                    ProcessSceneKind::TankFilling => {
                        let candidate = AttachedTank::prepare(&s.runtime, &s.variables)?;
                        candidate.restage(&mut s.runtime)?;
                        s.process = Some(candidate);
                    }
                }
                s.snapshot()
            },
        )
    }
    pub fn detach_process(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| {
            s.process.take().ok_or_else(|| {
                application_error("PLC-PROCESS-005", "No process scene is attached.")
            })?;
            s.snapshot()
        })
    }
    pub fn reset_process(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| {
            s.process
                .as_mut()
                .ok_or_else(|| {
                    application_error("PLC-PROCESS-005", "No process scene is attached.")
                })?
                .reset(&mut s.runtime)?;
            s.snapshot()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = include_str!("../../../scenes/tank-filling/example.st");
    fn request(source: &str) -> LoadStProgramRequest {
        LoadStProgramRequest {
            contract_version: CONTRACT_VERSION,
            source: source.into(),
        }
    }
    fn loaded(response: LoadProgramResponse) -> RuntimeSnapshotDto {
        let LoadProgramResponse::Loaded { snapshot } = response else {
            panic!("Valid ST required")
        };
        *snapshot
    }
    fn attach(service: &RuntimeSessionService, id: RuntimeSessionId) -> RuntimeSnapshotDto {
        service
            .attach_process(AttachProcessSceneRequest {
                contract_version: CONTRACT_VERSION,
                session_id: id,
                scene_kind: ProcessSceneKind::TankFilling,
            })
            .unwrap()
    }
    #[test]
    fn failed_newest_load_keeps_attached_plant_and_rejects_an_older_prepared_candidate() {
        let service = RuntimeSessionService::default();
        let x = loaded(service.load(request(EXAMPLE)).unwrap());
        let before = attach(&service, x.session_id);
        let intent = service.reserve_load_intent().unwrap();
        let PreparedLoad::Ready(a) = RuntimeSessionService::prepare_load(EXAMPLE).unwrap() else {
            panic!()
        };
        assert!(matches!(
            service.load(request("invalid ST")).unwrap(),
            LoadProgramResponse::CompileFailed { .. }
        ));
        assert_eq!(
            service.commit_prepared_load(intent, a).unwrap_err().code,
            "APP-RUNTIME-STALE-LOAD"
        );
        assert_eq!(
            service
                .snapshot(RuntimeSessionRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: x.session_id
                })
                .unwrap(),
            before
        );
        let c = loaded(service.load(request(EXAMPLE)).unwrap());
        assert_eq!(c.session_id.get(), 2);
        assert!(c.process.is_none());
    }
    #[test]
    fn impossible_binding_invariant_preserves_runtime_and_plant_then_fails_session_closed() {
        let service = RuntimeSessionService::default();
        let x = loaded(service.load(request(EXAMPLE)).unwrap());
        attach(&service, x.session_id);
        let (core, plant, old_id) = {
            let mut slot = service.inner.lock().unwrap();
            let session = slot.loaded.as_mut().unwrap();
            let old_id = session.variables[0].symbol;
            let previous = (
                session.runtime.snapshot(),
                session.process.as_ref().unwrap().snapshot(),
                old_id,
            );
            session.variables[0].symbol = SymbolId::new(999).unwrap();
            previous
        };
        let error = service
            .attach_process(AttachProcessSceneRequest {
                contract_version: CONTRACT_VERSION,
                session_id: x.session_id,
                scene_kind: ProcessSceneKind::TankFilling,
            })
            .unwrap_err();
        assert_eq!(error.code, "APP-PROCESS-INTERNAL");
        {
            let mut slot = service.inner.lock().unwrap();
            let session = slot.loaded.as_mut().unwrap();
            assert_eq!(session.runtime.snapshot(), core);
            assert_eq!(session.process.as_ref().unwrap().snapshot(), plant);
            session.variables[0].symbol = old_id;
        }
        let target = RuntimeSessionRequest {
            contract_version: CONTRACT_VERSION,
            session_id: x.session_id,
        };
        assert_eq!(
            service.single_scan(target.clone()).unwrap_err().code,
            "APP-PROCESS-INTERNAL"
        );
        assert_eq!(
            service.reset(target.clone()).unwrap_err().code,
            "APP-PROCESS-INTERNAL"
        );
        assert_eq!(
            service.snapshot(target.clone()).unwrap_err().code,
            "APP-PROCESS-INTERNAL"
        );
        service.unload(target).unwrap();
        assert!(
            loaded(service.load(request(EXAMPLE)).unwrap())
                .process
                .is_none()
        );
    }
}

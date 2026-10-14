use crate::{
    errors::{ConfigurationError, FaultKind, InputError, RuntimeFault, ScanError},
    executor::Executor,
    function_blocks::{FunctionBlockStateSnapshot, InstanceStates, initial_states},
    memory::{Memory, Values},
    plan::ExecutionPlan,
};
use plc_types::{
    PlcTime, PlcValue,
    diagnostics::Diagnostic,
    identity::{FunctionBlockInstanceId, SymbolId},
};
use simulation_core::SimulationClock;
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeConfig {
    cycle_period: PlcTime,
    execution_budget: NonZeroU64,
}
impl RuntimeConfig {
    pub fn new(cycle_period: PlcTime, execution_budget: u64) -> Result<Self, ConfigurationError> {
        if cycle_period == PlcTime::ZERO {
            return Err(ConfigurationError::ZeroCyclePeriod);
        }
        Ok(Self {
            cycle_period,
            execution_budget: NonZeroU64::new(execution_budget)
                .ok_or(ConfigurationError::ZeroExecutionBudget)?,
        })
    }
    pub fn cycle_period(self) -> PlcTime {
        self.cycle_period
    }
    pub fn execution_budget(self) -> u64 {
        self.execution_budget.get()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Ready,
    Faulted,
}

/// Owned immutable observation, not trace recording or a frontend transport contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    staged_inputs: Values,
    input_image: Values,
    committed: Values,
    outputs: Values,
    time: PlcTime,
    completed_scans: u64,
    state: RuntimeState,
    fault: Option<Diagnostic>,
    function_blocks:
        std::collections::BTreeMap<FunctionBlockInstanceId, FunctionBlockStateSnapshot>,
}
impl RuntimeSnapshot {
    pub fn function_blocks(
        &self,
    ) -> &std::collections::BTreeMap<FunctionBlockInstanceId, FunctionBlockStateSnapshot> {
        &self.function_blocks
    }
    pub fn staged_inputs(&self) -> &std::collections::BTreeMap<SymbolId, PlcValue> {
        &self.staged_inputs
    }
    pub fn input_image(&self) -> &std::collections::BTreeMap<SymbolId, PlcValue> {
        &self.input_image
    }
    pub fn committed(&self) -> &std::collections::BTreeMap<SymbolId, PlcValue> {
        &self.committed
    }
    pub fn outputs(&self) -> &std::collections::BTreeMap<SymbolId, PlcValue> {
        &self.outputs
    }
    pub fn time(&self) -> PlcTime {
        self.time
    }
    pub fn completed_scans(&self) -> u64 {
        self.completed_scans
    }
    pub fn state(&self) -> RuntimeState {
        self.state
    }
    pub fn fault(&self) -> Option<&Diagnostic> {
        self.fault.as_ref()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    snapshot: RuntimeSnapshot,
    operations: u64,
}
impl ScanResult {
    pub fn snapshot(&self) -> &RuntimeSnapshot {
        &self.snapshot
    }
    pub fn operations(&self) -> u64 {
        self.operations
    }
}

/// Exactly one synchronous cyclic task. No borrowed source or shared input handles.
pub struct Runtime {
    plan: ExecutionPlan,
    config: RuntimeConfig,
    memory: Memory,
    clock: SimulationClock,
    completed_scans: u64,
    state: RuntimeState,
    last_failure: Option<ScanError>,
    instances: InstanceStates,
}
impl Runtime {
    pub fn new(plan: ExecutionPlan, config: RuntimeConfig) -> Self {
        let memory = Memory::new(&plan.layout);
        let instances = initial_states(&plan.instances);
        Self {
            plan,
            config,
            memory,
            clock: SimulationClock::new(),
            completed_scans: 0,
            state: RuntimeState::Ready,
            last_failure: None,
            instances,
        }
    }
    pub fn state(&self) -> RuntimeState {
        self.state
    }
    pub fn set_input(&mut self, id: SymbolId, value: PlcValue) -> Result<(), InputError> {
        self.memory.set_input(&self.plan.layout, id, value)
    }
    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            function_blocks: self
                .instances
                .iter()
                .map(|(id, state)| (*id, state.snapshot()))
                .collect(),
            staged_inputs: self.memory.staged_inputs.clone(),
            input_image: self.memory.input_image.clone(),
            committed: self.memory.committed.clone(),
            outputs: self.memory.outputs.clone(),
            time: self.clock.now(),
            completed_scans: self.completed_scans,
            state: self.state,
            fault: self
                .last_failure
                .as_ref()
                .and_then(ScanError::diagnostic)
                .cloned(),
        }
    }
    pub fn single_scan(&mut self) -> Result<ScanResult, ScanError> {
        if self.state != RuntimeState::Ready {
            return Err(ScanError::Faulted);
        }
        match self.scan_transaction() {
            Ok(operations) => Ok(ScanResult {
                snapshot: self.snapshot(),
                operations,
            }),
            Err(error) => {
                self.state = RuntimeState::Faulted;
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }
    fn scan_transaction(&mut self) -> Result<u64, ScanError> {
        let mut working = self.memory.begin_scan();
        let mut working_instances = self.instances.clone();
        let operations = Executor::new(
            &self.plan,
            &mut working,
            &mut working_instances,
            self.clock.now(),
            self.config.execution_budget(),
        )
        .execute()?;
        let mut next_clock = self.clock;
        next_clock
            .advance(self.config.cycle_period())
            .map_err(|_| {
                ScanError::Fault(RuntimeFault::new(FaultKind::ClockOverflow.diagnostic()))
            })?;
        let next_count = self.completed_scans.checked_add(1).ok_or_else(|| {
            ScanError::Fault(RuntimeFault::new(
                FaultKind::ScanCounterOverflow.diagnostic(),
            ))
        })?;
        self.memory
            .commit(&self.plan.layout, working)
            .map_err(ScanError::Internal)?;
        self.instances = working_instances;
        self.completed_scans = next_count;
        self.clock = next_clock;
        Ok(operations)
    }
    /// Reinitialize all images, fault, count, and time; preserve plan/configuration.
    pub fn reset(&mut self) {
        self.memory = Memory::new(&self.plan.layout);
        self.instances = initial_states(&self.plan.instances);
        self.clock = SimulationClock::new();
        self.completed_scans = 0;
        self.state = RuntimeState::Ready;
        self.last_failure = None;
    }
}

#[cfg(test)]
#[path = "ton_transaction_tests.rs"]
mod ton_transaction_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lower,
        plan::{ExecExpressionKind, ExecStatementKind},
    };
    use plc_ir::{Expression, Program, Statement, validate};
    use plc_types::{
        PlcType,
        diagnostics::DiagnosticCategory,
        identity::{DocumentId, PouId, SourceElementId},
        source::{SourceLanguage, SourceLocation, SourceMap, SourceRef},
        symbols::{Pou, PouKind, Scope, Symbol, SymbolTable, VariableCategory},
    };

    fn plan() -> (ExecutionPlan, SymbolId) {
        let owner = PouId::new(1).unwrap();
        let target = SymbolId::new(1).unwrap();
        let source = SourceRef::new(
            DocumentId::new(1).unwrap(),
            SourceElementId::new(1).unwrap(),
        );
        let mut symbols = SymbolTable::new();
        symbols
            .declare_pou(Pou::new(owner, "Main", PouKind::Program).unwrap())
            .unwrap();
        symbols
            .declare(
                Symbol::new(
                    target,
                    "Output",
                    PlcType::Bool,
                    Scope::Pou(owner),
                    VariableCategory::Output,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        let mut sources = SourceMap::new();
        sources
            .register_document(source.document_id(), SourceLanguage::St)
            .unwrap();
        sources
            .register_element(SourceLocation::new(source, SourceLanguage::St, None))
            .unwrap();
        let unused = SourceRef::new(source.document_id(), SourceElementId::new(2).unwrap());
        sources
            .register_element(SourceLocation::new(unused, SourceLanguage::St, None))
            .unwrap();
        let program = Program::new(
            symbols,
            sources,
            owner,
            vec![Statement::assign(
                source,
                target,
                Expression::constant(source, PlcValue::Bool(true)),
            )],
        );
        (lower(&validate(&program).unwrap()).unwrap(), target)
    }
    #[test]
    fn owned_plan_retains_only_execution_source_anchors() {
        let (plan, _) = plan();
        assert_eq!(plan.sources.elements().count(), 1);
        assert_eq!(
            plan.sources
                .elements()
                .next()
                .unwrap()
                .reference()
                .element_id()
                .get(),
            1
        );
    }
    #[test]
    fn completed_counter_overflow_rolls_back_writes_and_clock() {
        let (plan, target) = plan();
        let mut r = Runtime::new(
            plan,
            RuntimeConfig::new(PlcTime::from_micros(1), 2).unwrap(),
        );
        // Test an otherwise unreachable boundary without exposing a production count setter.
        r.completed_scans = u64::MAX;
        let error = r.single_scan().unwrap_err();
        assert_eq!(error.diagnostic().unwrap().code(), "PLC-RUNTIME-006");
        assert_eq!(r.snapshot().completed_scans(), u64::MAX);
        assert_eq!(r.snapshot().time(), PlcTime::ZERO);
        assert_eq!(r.snapshot().outputs()[&target], PlcValue::Bool(false));
        assert_eq!(r.state(), RuntimeState::Faulted);
    }
    #[test]
    fn impossible_result_type_is_internal_error_not_student_fault() {
        let (mut plan, _) = plan();
        let ExecStatementKind::Assign(_, expression) = &mut plan.body[0].kind else {
            panic!("assignment expected")
        };
        expression.kind = ExecExpressionKind::Constant(PlcValue::Int(1));
        let mut r = Runtime::new(
            plan,
            RuntimeConfig::new(PlcTime::from_micros(1), 2).unwrap(),
        );
        let error = r.single_scan().unwrap_err();
        assert!(matches!(error, ScanError::Internal(_)));
        assert_eq!(
            error.diagnostic().unwrap().category(),
            DiagnosticCategory::InternalApplicationError
        );
        assert_eq!(error.diagnostic().unwrap().code(), "APP-CORE-001");
        assert_eq!(r.snapshot().completed_scans(), 0);
    }
}

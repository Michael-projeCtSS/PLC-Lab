use crate::{
    errors::{FaultKind, InternalError, RuntimeFault, ScanError},
    function_blocks::InstanceStates,
    memory::WorkingState,
    operators::{self, OperationError},
    plan::{ExecExpression, ExecExpressionKind, ExecStatement, ExecStatementKind, ExecutionPlan},
};
use plc_types::{
    PlcTime, PlcValue, diagnostics::RelatedIdentity, identity::SymbolId, source::SourceRef,
};

pub(crate) struct Executor<'a> {
    plan: &'a ExecutionPlan,
    working: &'a mut WorkingState,
    limit: u64,
    used: u64,
    instances: &'a mut InstanceStates,
    now: PlcTime,
}
impl<'a> Executor<'a> {
    pub(crate) fn new(
        plan: &'a ExecutionPlan,
        working: &'a mut WorkingState,
        instances: &'a mut InstanceStates,
        now: PlcTime,
        limit: u64,
    ) -> Self {
        Self {
            plan,
            working,
            limit,
            used: 0,
            instances,
            now,
        }
    }
    pub(crate) fn execute(mut self) -> Result<u64, ScanError> {
        self.body(&self.plan.body)?;
        Ok(self.used)
    }
    fn fault(&self, kind: FaultKind, source: SourceRef, related: Option<SymbolId>) -> ScanError {
        let mut diagnostic = kind
            .diagnostic()
            .with_source(self.plan.sources.get(source).cloned());
        if let Some(id) = related {
            diagnostic = diagnostic.with_related(RelatedIdentity::Symbol(id));
        }
        ScanError::Fault(RuntimeFault::new(diagnostic))
    }
    fn internal(&self, error: InternalError, source: SourceRef) -> ScanError {
        ScanError::Internal(error.at(self.plan.sources.get(source).cloned()))
    }
    fn charge(&mut self, source: SourceRef, related: Option<SymbolId>) -> Result<(), ScanError> {
        if self.used >= self.limit {
            return Err(self.fault(FaultKind::Budget, source, related));
        }
        self.used += 1;
        Ok(())
    }
    fn operation(
        &self,
        result: Result<PlcValue, OperationError>,
        source: SourceRef,
        related: Option<SymbolId>,
    ) -> Result<PlcValue, ScanError> {
        result.map_err(|e| match e {
            OperationError::Fault(kind) => self.fault(kind, source, related),
            OperationError::Internal(e) => self.internal(e, source),
        })
    }
    fn expression(
        &mut self,
        expression: &ExecExpression,
        related: Option<SymbolId>,
    ) -> Result<PlcValue, ScanError> {
        let value = match &expression.kind {
            ExecExpressionKind::Constant(value) => {
                self.charge(expression.source, related)?;
                *value
            }
            ExecExpressionKind::Read(id) => {
                self.charge(expression.source, related)?;
                self.working
                    .read(*id)
                    .map_err(|e| self.internal(e, expression.source))?
            }
            ExecExpressionKind::Unary(op, operand) => {
                let operand = self.expression(operand, related)?;
                self.charge(expression.source, related)?;
                self.operation(operators::unary(*op, operand), expression.source, related)?
            }
            ExecExpressionKind::Binary(op, left, right) => {
                let left = self.expression(left, related)?;
                let right = self.expression(right, related)?;
                self.charge(expression.source, related)?;
                self.operation(
                    operators::binary(*op, left, right),
                    expression.source,
                    related,
                )?
            }
        };
        if value.plc_type() != expression.result_type {
            return Err(self.internal(
                InternalError::new("An execution result differs from its validated type."),
                expression.source,
            ));
        }
        Ok(value)
    }
    fn body(&mut self, body: &[ExecStatement]) -> Result<(), ScanError> {
        for statement in body {
            match &statement.kind {
                ExecStatementKind::Assign(target, value) => {
                    let value = self.expression(value, Some(*target))?;
                    self.charge(statement.source, Some(*target))?;
                    self.working
                        .write(*target, value)
                        .map_err(|e| self.internal(e, statement.source))?;
                }
                ExecStatementKind::If(condition, then_body, else_body) => {
                    let condition = self.expression(condition, None)?;
                    self.charge(statement.source, None)?;
                    let PlcValue::Bool(condition) = condition else {
                        return Err(self.internal(
                            InternalError::new(
                                "A validated conditional does not have a BOOL result.",
                            ),
                            statement.source,
                        ));
                    };
                    self.body(if condition { then_body } else { else_body })?;
                }
                ExecStatementKind::TonCall {
                    instance,
                    input,
                    preset,
                    q_target,
                    et_target,
                } => {
                    let input = self.expression(input, None)?;
                    let preset = self.expression(preset, None)?;
                    let (PlcValue::Bool(input), PlcValue::Time(preset)) = (input, preset) else {
                        return Err(self.internal(
                            InternalError::new(
                                "A validated TON call has incompatible input types.",
                            ),
                            statement.source,
                        ));
                    };
                    self.charge(statement.source, None)?;
                    let state = self.instances.get_mut(instance).ok_or_else(|| {
                        ScanError::Internal(
                            InternalError::new("A compiled TON instance has no working state.")
                                .at(self.plan.sources.get(statement.source).cloned()),
                        )
                    })?;
                    let (q, et) = state
                        .ton(input, preset, self.now)
                        .map_err(|e| self.internal(e, statement.source))?;
                    if let Some(target) = q_target {
                        self.charge(statement.source, Some(*target))?;
                        self.working
                            .write(*target, PlcValue::Bool(q))
                            .map_err(|e| self.internal(e, statement.source))?;
                    }
                    if let Some(target) = et_target {
                        self.charge(statement.source, Some(*target))?;
                        self.working
                            .write(*target, PlcValue::Time(et))
                            .map_err(|e| self.internal(e, statement.source))?;
                    }
                }
            }
        }
        Ok(())
    }
}

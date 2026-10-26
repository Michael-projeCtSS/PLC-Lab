use crate::{
    errors::{FaultKind, InternalError, RuntimeFault, ScanError},
    function_blocks::{CallInputs, InstanceStates},
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
            ExecExpressionKind::Logical(op, operands) => {
                let mut values = Vec::with_capacity(operands.len());
                for operand in operands {
                    values.push(self.expression(operand, related)?);
                }
                self.charge(expression.source, related)?;
                self.operation(operators::logical(*op, &values), expression.source, related)?
            }
            ExecExpressionKind::FunctionBlockOutput(instance, parameter) => {
                self.charge(expression.source, related)?;
                self.instances
                    .get(instance)
                    .and_then(|state| state.output(*parameter))
                    .ok_or_else(|| {
                        self.internal(
                            InternalError::new("A compiled block output has no working state."),
                            expression.source,
                        )
                    })?
            }
            ExecExpressionKind::Conversion(function, operand) => {
                let operand = self.expression(operand, related)?;
                self.charge(expression.source, related)?;
                self.operation(operators::convert(*function, operand), expression.source, related)?
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
                ExecStatementKind::StandardCall { instance, inputs, outputs } => {
                    let mut values = Vec::with_capacity(inputs.len());
                    for (parameter, expression) in inputs {
                        values.push((*parameter, self.expression(expression, None)?));
                    }
                    self.charge(statement.source, None)?;
                    let source = statement.source;
                    let state = self.instances.get_mut(instance).ok_or_else(|| {
                        ScanError::Internal(
                            InternalError::new("A compiled block instance has no working state.")
                                .at(self.plan.sources.get(source).cloned()),
                        )
                    })?;
                    state
                        .call(CallInputs(&values), self.now)
                        .map_err(|e| ScanError::Internal(e.at(self.plan.sources.get(source).cloned())))?;
                    for (parameter, target) in outputs {
                        let value = self.instances[instance]
                            .output(plc_types::identity::ParameterId::new(*parameter).map_err(|_| {
                                self.internal(InternalError::new("A compiled block output identity is zero."), source)
                            })?)
                            .ok_or_else(|| self.internal(InternalError::new("A compiled block output is unknown."), source))?;
                        self.charge(source, Some(*target))?;
                        self.working
                            .write(*target, value)
                            .map_err(|e| self.internal(e, source))?;
                    }
                }
            }
        }
        Ok(())
    }
}

use plc_types::{
    PlcType,
    diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticCode, Severity, codes},
    identity::SymbolId,
};

pub(crate) fn diagnostic(
    code: &'static str,
    category: DiagnosticCategory,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("fixed core diagnostic code"),
        category,
        Severity::Error,
        message,
    )
    .expect("fixed nonblank core explanation")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFault {
    diagnostic: Diagnostic,
}
impl RuntimeFault {
    pub(crate) fn new(diagnostic: Diagnostic) -> Self {
        Self { diagnostic }
    }
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalError {
    diagnostic: Diagnostic,
}
impl InternalError {
    pub(crate) fn new(message: &'static str) -> Self {
        Self {
            diagnostic: diagnostic(
                codes::INTERNAL_INVARIANT,
                DiagnosticCategory::InternalApplicationError,
                message,
            ),
        }
    }
    pub(crate) fn at(mut self, source: Option<plc_types::source::SourceLocation>) -> Self {
        self.diagnostic = self.diagnostic.with_source(source);
        self
    }
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweringError {
    Unsupported(Diagnostic),
    Internal(InternalError),
}
impl LoweringError {
    pub fn diagnostic(&self) -> &Diagnostic {
        match self {
            Self::Unsupported(d) => d,
            Self::Internal(e) => e.diagnostic(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    Fault(RuntimeFault),
    Internal(InternalError),
    Faulted,
}
impl ScanError {
    pub fn diagnostic(&self) -> Option<&Diagnostic> {
        match self {
            Self::Fault(f) => Some(f.diagnostic()),
            Self::Internal(e) => Some(e.diagnostic()),
            Self::Faulted => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationError {
    ZeroCyclePeriod,
    ZeroExecutionBudget,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    UnknownSymbol(SymbolId),
    NotExternalInput(SymbolId),
    TypeMismatch {
        symbol: SymbolId,
        expected: PlcType,
        actual: PlcType,
    },
}

macro_rules! diagnostic_error {
    ($name:ident) => {
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.diagnostic().message())
            }
        }
        impl std::error::Error for $name {}
    };
}
diagnostic_error!(RuntimeFault);
diagnostic_error!(InternalError);
diagnostic_error!(LoweringError);
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.diagnostic() {
            Some(d) => f.write_str(d.message()),
            None => f.write_str("The runtime is faulted; explicitly reset it before another scan."),
        }
    }
}
impl std::error::Error for ScanError {}
impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ZeroCyclePeriod => "The cycle period must be positive.",
            Self::ZeroExecutionBudget => "The execution budget must be positive.",
        })
    }
}
impl std::error::Error for ConfigurationError {}
impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSymbol(id) => {
                write!(f, "Symbol {} is not in the entry memory layout.", id.get())
            }
            Self::NotExternalInput(id) => {
                write!(f, "Symbol {} is not an external entry input.", id.get())
            }
            Self::TypeMismatch {
                symbol,
                expected,
                actual,
            } => write!(
                f,
                "Input {} requires {expected:?}, received {actual:?}.",
                symbol.get()
            ),
        }
    }
}
impl std::error::Error for InputError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FaultKind {
    Budget,
    IntOverflow,
    DivisionByZero,
    NonFiniteReal,
    ClockOverflow,
    ScanCounterOverflow,
}
impl FaultKind {
    pub(crate) fn diagnostic(self) -> Diagnostic {
        let (code, message) = match self {
            Self::Budget => (
                codes::EXECUTION_BUDGET,
                "The scan exceeded its execution operation budget.",
            ),
            Self::IntOverflow => (
                codes::INT_OVERFLOW,
                "INT arithmetic exceeded the supported signed 16-bit range.",
            ),
            Self::DivisionByZero => (
                codes::DIVISION_BY_ZERO,
                "INT division by zero is not allowed.",
            ),
            Self::NonFiniteReal => (
                codes::NON_FINITE_REAL,
                "REAL arithmetic produced a non-finite result.",
            ),
            Self::ClockOverflow => (
                codes::CLOCK_OVERFLOW,
                "Completing this scan would exceed the virtual time range.",
            ),
            Self::ScanCounterOverflow => (
                codes::SCAN_COUNTER_OVERFLOW,
                "Completing this scan would exceed the scan counter range.",
            ),
        };
        diagnostic(code, DiagnosticCategory::RuntimeFault, message)
    }
}

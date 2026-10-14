//! Shared data; category tags do not implement domain fault behavior.

use crate::identity::{ComponentId, FunctionBlockId, FunctionBlockInstanceId, PouId, SymbolId};
pub use crate::source::{SourceLocation, SourcePosition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticCategory {
    CompilerDiagnostic,
    RuntimeFault,
    SimulationFault,
    CommunicationFault,
    InternalApplicationError,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct DiagnosticCode(String);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticError {
    InvalidCode,
    EmptyMessage,
}
impl std::fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCode => {
                "Diagnostic codes require uppercase ASCII letters, digits, and internal hyphens."
            }
            Self::EmptyMessage => "Diagnostics require a nonblank explanation.",
        })
    }
}
impl std::error::Error for DiagnosticError {}
impl DiagnosticCode {
    pub fn new(code: impl Into<String>) -> Result<Self, DiagnosticError> {
        let code = code.into();
        if code.is_empty()
            || code.starts_with('-')
            || code.ends_with('-')
            || code.contains("--")
            || !code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(DiagnosticError::InvalidCode);
        }
        Ok(Self(code))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub mod codes {
    pub const UNKNOWN_SYMBOL: &str = "PLC-COMP-001";
    pub const DUPLICATE_SYMBOL: &str = "PLC-COMP-002";
    pub const INVALID_SCOPE: &str = "PLC-COMP-003";
    pub const INCOMPATIBLE_REDECLARATION: &str = "PLC-COMP-004";
    pub const DUPLICATE_IDENTITY: &str = "PLC-COMP-005";
    pub const INVALID_NAME: &str = "PLC-COMP-006";
    pub const ASSIGNMENT_TYPE: &str = "PLC-COMP-007";
    pub const OPERAND_TYPE: &str = "PLC-COMP-008";
    pub const CONDITION_TYPE: &str = "PLC-COMP-009";
    pub const UNKNOWN_SOURCE: &str = "PLC-COMP-010";
    pub const UNSUPPORTED_CALL: &str = "PLC-COMP-011";
    pub const VALIDATION_LIMIT: &str = "PLC-COMP-012";
    pub const INVALID_ENTRY: &str = "PLC-COMP-013";
    pub const INVALID_WRITE_TARGET: &str = "PLC-COMP-014";
    pub const UNSUPPORTED_DECLARATION: &str = "PLC-COMP-015";
    pub const UNKNOWN_FB_INSTANCE: &str = "PLC-COMP-016";
    pub const UNKNOWN_FB_DEFINITION: &str = "PLC-COMP-017";
    pub const MISSING_FB_INPUT: &str = "PLC-COMP-018";
    pub const UNKNOWN_FB_PARAMETER: &str = "PLC-COMP-019";
    pub const FB_PARAMETER_DIRECTION: &str = "PLC-COMP-020";
    pub const FB_PARAMETER_TYPE: &str = "PLC-COMP-021";
    pub const DUPLICATE_FB_BINDING: &str = "PLC-COMP-022";
    pub const UNSUPPORTED_FB_KIND: &str = "PLC-COMP-023";
    pub const INVALID_FB_OWNER: &str = "PLC-COMP-024";
    pub const EXECUTION_BUDGET: &str = "PLC-RUNTIME-001";
    pub const INT_OVERFLOW: &str = "PLC-RUNTIME-002";
    pub const DIVISION_BY_ZERO: &str = "PLC-RUNTIME-003";
    pub const NON_FINITE_REAL: &str = "PLC-RUNTIME-004";
    pub const CLOCK_OVERFLOW: &str = "PLC-RUNTIME-005";
    pub const SCAN_COUNTER_OVERFLOW: &str = "PLC-RUNTIME-006";
    pub const INTERNAL_INVARIANT: &str = "APP-CORE-001";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum RelatedIdentity {
    Symbol(SymbolId),
    Pou(PouId),
    Component(ComponentId),
    FunctionBlockDefinition(FunctionBlockId),
    FunctionBlockInstance(FunctionBlockInstanceId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    code: DiagnosticCode,
    category: DiagnosticCategory,
    severity: Severity,
    message: String,
    source: Option<SourceLocation>,
    related: Option<RelatedIdentity>,
}
impl Diagnostic {
    pub fn new(
        code: DiagnosticCode,
        category: DiagnosticCategory,
        severity: Severity,
        message: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        let message = message.into();
        if message.trim().is_empty() {
            return Err(DiagnosticError::EmptyMessage);
        }
        Ok(Self {
            code,
            category,
            severity,
            message,
            source: None,
            related: None,
        })
    }
    pub(crate) fn compiler_error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(
            DiagnosticCode::new(code).expect("core diagnostic code invariant"),
            DiagnosticCategory::CompilerDiagnostic,
            Severity::Error,
            message,
        )
        .expect("core diagnostic message invariant")
    }
    pub fn with_source(mut self, source: Option<SourceLocation>) -> Self {
        self.source = source;
        self
    }
    pub fn with_related(mut self, related: RelatedIdentity) -> Self {
        self.related = Some(related);
        self
    }
    pub fn code(&self) -> &str {
        self.code.as_str()
    }
    pub fn category(&self) -> DiagnosticCategory {
        self.category
    }
    pub fn severity(&self) -> Severity {
        self.severity
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn source(&self) -> Option<&SourceLocation> {
        self.source.as_ref()
    }
    pub fn related(&self) -> Option<RelatedIdentity> {
        self.related
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{DocumentId, SourceElementId};
    use crate::source::{SourceLanguage, SourceRef, TextSpan};

    #[test]
    fn stable_semantic_codes_are_valid_and_distinct_from_internal_failure() {
        let entries = [
            codes::UNKNOWN_SYMBOL,
            codes::DUPLICATE_SYMBOL,
            codes::INVALID_SCOPE,
            codes::INCOMPATIBLE_REDECLARATION,
            codes::DUPLICATE_IDENTITY,
            codes::INVALID_NAME,
            codes::ASSIGNMENT_TYPE,
            codes::OPERAND_TYPE,
            codes::CONDITION_TYPE,
            codes::UNKNOWN_SOURCE,
            codes::UNSUPPORTED_CALL,
            codes::VALIDATION_LIMIT,
            codes::INVALID_ENTRY,
            codes::INVALID_WRITE_TARGET,
            codes::UNSUPPORTED_DECLARATION,
            codes::UNKNOWN_FB_INSTANCE,
            codes::UNKNOWN_FB_DEFINITION,
            codes::MISSING_FB_INPUT,
            codes::UNKNOWN_FB_PARAMETER,
            codes::FB_PARAMETER_DIRECTION,
            codes::FB_PARAMETER_TYPE,
            codes::DUPLICATE_FB_BINDING,
            codes::UNSUPPORTED_FB_KIND,
            codes::INVALID_FB_OWNER,
        ];
        for (index, code) in entries.into_iter().enumerate() {
            assert_eq!(code, format!("PLC-COMP-{:03}", index + 1));
            assert!(DiagnosticCode::new(code).is_ok());
        }
        assert_eq!(codes::INTERNAL_INVARIANT, "APP-CORE-001");
        for (index, code) in [
            codes::EXECUTION_BUDGET,
            codes::INT_OVERFLOW,
            codes::DIVISION_BY_ZERO,
            codes::NON_FINITE_REAL,
            codes::CLOCK_OVERFLOW,
            codes::SCAN_COUNTER_OVERFLOW,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(code, format!("PLC-RUNTIME-{:03}", index + 1));
            assert!(DiagnosticCode::new(code).is_ok());
        }
        let student =
            Diagnostic::compiler_error(codes::UNKNOWN_SYMBOL, "The variable is not declared.");
        let internal = Diagnostic::new(
            DiagnosticCode::new(codes::INTERNAL_INVARIANT).unwrap(),
            DiagnosticCategory::InternalApplicationError,
            Severity::Error,
            "A core invariant failed.",
        )
        .unwrap();
        assert_ne!(student.category(), internal.category());
    }
    #[test]
    fn rejects_invalid_codes_and_blank_messages() {
        for code in ["", "a", "-APP", "APP-", "APP--001", "APP 001", "APP_001"] {
            assert_eq!(DiagnosticCode::new(code), Err(DiagnosticError::InvalidCode));
        }
        assert_eq!(
            Diagnostic::new(
                DiagnosticCode::new("APP-001").unwrap(),
                DiagnosticCategory::InternalApplicationError,
                Severity::Warning,
                " \n"
            ),
            Err(DiagnosticError::EmptyMessage)
        );
    }
    #[test]
    fn serializes_text_and_graphical_sources_and_related_identities() {
        let reference = SourceRef::new(
            DocumentId::new(7).unwrap(),
            SourceElementId::new(12).unwrap(),
        );
        let span = TextSpan::new(
            SourcePosition::new(1, 1).unwrap(),
            SourcePosition::new(1, 6).unwrap(),
        )
        .unwrap();
        let base =
            Diagnostic::compiler_error(codes::UNKNOWN_SYMBOL, "The variable is not declared.");
        assert!(serde_json::to_value(&base).unwrap()["source"].is_null());
        let text = base
            .clone()
            .with_source(Some(SourceLocation::new(
                reference,
                SourceLanguage::St,
                Some(span),
            )))
            .with_related(RelatedIdentity::Symbol(SymbolId::new(4).unwrap()));
        let json = serde_json::to_value(&text).unwrap();
        assert_eq!(json["code"], "PLC-COMP-001");
        assert_eq!(json["source"]["reference"]["documentId"], 7);
        assert_eq!(json["source"]["textSpan"]["end"]["column"], 6);
        assert_eq!(json["related"]["kind"], "symbol");
        assert_eq!(text.source().unwrap().reference(), reference);
        assert_eq!(
            text.related(),
            Some(RelatedIdentity::Symbol(SymbolId::new(4).unwrap()))
        );
        assert!(!text.message().is_empty());
        let graphical = base.with_source(Some(SourceLocation::new(
            reference,
            SourceLanguage::Fbd,
            None,
        )));
        assert!(serde_json::to_value(&graphical).unwrap()["source"]["textSpan"].is_null());
    }
    #[test]
    fn categories_and_severities_keep_m0_transport_names() {
        for (category, name) in [
            (DiagnosticCategory::CompilerDiagnostic, "compilerDiagnostic"),
            (DiagnosticCategory::RuntimeFault, "runtimeFault"),
            (DiagnosticCategory::SimulationFault, "simulationFault"),
            (DiagnosticCategory::CommunicationFault, "communicationFault"),
            (
                DiagnosticCategory::InternalApplicationError,
                "internalApplicationError",
            ),
        ] {
            assert_eq!(serde_json::to_value(category).unwrap(), name);
        }
        for (severity, name) in [
            (Severity::Info, "info"),
            (Severity::Warning, "warning"),
            (Severity::Error, "error"),
        ] {
            let diagnostic = Diagnostic::new(
                DiagnosticCode::new("APP-001").unwrap(),
                DiagnosticCategory::InternalApplicationError,
                severity,
                "An explanation.",
            )
            .unwrap();
            assert_eq!(serde_json::to_value(&diagnostic).unwrap()["severity"], name);
            assert_eq!(diagnostic.severity(), severity);
        }
    }
}

//! Presentation transport only. Session ownership lives in desktop-session.
use crate::{CompilerDiagnostic, CompilerSpan};
use plc_types::{PlcReal, PlcTime, PlcType, PlcValue};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

macro_rules! decimal_identity {
    ($name:ident, $integer:ty, $nonzero:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
        pub struct $name(#[ts(type = "string")] $integer);
        impl $name {
            pub fn new(value: $integer) -> Result<Self, &'static str> {
                if $nonzero && value == 0 {
                    Err("Identity must be nonzero.")
                } else {
                    Ok(Self(value))
                }
            }
            pub fn get(self) -> $integer {
                self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = &'static str;
            fn try_from(text: String) -> Result<Self, Self::Error> {
                if text.is_empty()
                    || !text.bytes().all(|b| b.is_ascii_digit())
                    || (text.len() > 1 && text.starts_with('0'))
                {
                    return Err("Expected canonical unsigned decimal text.");
                }
                Self::new(
                    text.parse::<$integer>()
                        .map_err(|_| "Decimal value is out of range.")?,
                )
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0.to_string()
            }
        }
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0.to_string())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::try_from(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
decimal_identity!(RuntimeSessionId, u64, true);
decimal_identity!(RuntimeVariableId, u32, true);
decimal_identity!(RuntimeInstanceId, u32, true);
decimal_identity!(ExactU64, u64, false);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "UPPERCASE", deny_unknown_fields)]
pub enum RuntimeValue {
    Bool {
        value: bool,
    },
    Int {
        value: i16,
    },
    Real {
        #[ts(type = "number")]
        value: PlcReal,
    },
    Time {
        #[ts(type = "string")]
        microseconds: PlcTime,
    },
}
impl From<PlcValue> for RuntimeValue {
    fn from(value: PlcValue) -> Self {
        match value {
            PlcValue::Bool(value) => Self::Bool { value },
            PlcValue::Int(value) => Self::Int { value },
            PlcValue::Real(value) => Self::Real { value },
            PlcValue::Time(microseconds) => Self::Time { microseconds },
        }
    }
}
impl From<RuntimeValue> for PlcValue {
    fn from(value: RuntimeValue) -> Self {
        match value {
            RuntimeValue::Bool { value } => Self::Bool(value),
            RuntimeValue::Int { value } => Self::Int(value),
            RuntimeValue::Real { value } => Self::Real(value),
            RuntimeValue::Time { microseconds } => Self::Time(microseconds),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum RuntimeRole {
    Input,
    Output,
    Local,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
pub enum SessionRuntimeState {
    Ready,
    Faulted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeVariable {
    pub variable_id: RuntimeVariableId,
    pub name: String,
    #[ts(type = "\"BOOL\" | \"INT\" | \"REAL\" | \"TIME\"")]
    pub type_name: PlcType,
    pub role: RuntimeRole,
    pub value: RuntimeValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "UPPERCASE",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RuntimeFunctionBlock {
    Ton {
        instance_id: RuntimeInstanceId,
        name: String,
        q: bool,
        #[ts(type = "string")]
        et_micros: PlcTime,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeRelatedVariable {
    pub variable_id: RuntimeVariableId,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeDiagnostic {
    pub code: String,
    #[ts(type = "\"error\" | \"warning\" | \"info\"")]
    pub severity: String,
    pub message: String,
    pub span: Option<CompilerSpan>,
    #[ts(type = "\"runtimeFault\" | \"internalApplicationError\"")]
    pub category: String,
    pub related_variable: Option<RuntimeRelatedVariable>,
}
impl RuntimeDiagnostic {
    pub fn from_core(
        d: &plc_types::diagnostics::Diagnostic,
        related_variable: Option<RuntimeRelatedVariable>,
    ) -> Self {
        let display = CompilerDiagnostic::from(d);
        Self {
            code: display.code,
            severity: display.severity,
            message: display.message,
            span: display.span,
            category: match d.category() {
                plc_types::diagnostics::DiagnosticCategory::RuntimeFault => "runtimeFault",
                _ => "internalApplicationError",
            }
            .into(),
            related_variable,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSnapshotDto {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
    pub program_name: String,
    pub state: SessionRuntimeState,
    pub completed_scans: ExactU64,
    #[ts(type = "string")]
    pub simulation_time_micros: PlcTime,
    #[ts(type = "string")]
    pub cycle_period_micros: PlcTime,
    pub operation_budget: ExactU64,
    pub variables: Vec<RuntimeVariable>,
    pub function_blocks: Vec<RuntimeFunctionBlock>,
    pub fault: Option<RuntimeDiagnostic>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadStProgramRequest {
    pub contract_version: u32,
    pub source: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSessionRequest {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetRuntimeInputRequest {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
    pub variable_id: RuntimeVariableId,
    pub value: RuntimeValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum LoadProgramResponse {
    Loaded {
        snapshot: Box<RuntimeSnapshotDto>,
    },
    CompileFailed {
        diagnostics: Vec<CompilerDiagnostic>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnloadProgramResponse {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
}

pub(crate) fn session_typescript(config: &ts_rs::Config) -> String {
    [
        RuntimeSessionId::decl(config),
        RuntimeVariableId::decl(config),
        RuntimeInstanceId::decl(config),
        ExactU64::decl(config),
        RuntimeValue::decl(config),
        RuntimeRole::decl(config),
        SessionRuntimeState::decl(config),
        RuntimeVariable::decl(config),
        RuntimeFunctionBlock::decl(config),
        RuntimeRelatedVariable::decl(config),
        RuntimeDiagnostic::decl(config),
        RuntimeSnapshotDto::decl(config),
        LoadStProgramRequest::decl(config),
        RuntimeSessionRequest::decl(config),
        SetRuntimeInputRequest::decl(config),
        LoadProgramResponse::decl(config),
        UnloadProgramResponse::decl(config),
    ]
    .into_iter()
    .map(|d| format!("\nexport {d}\n"))
    .collect()
}

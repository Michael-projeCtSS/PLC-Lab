//! Pure compiler adapter: presentation metadata only, never a runtime instance.
use crate::{CONTRACT_VERSION, IpcError};
use plc_types::{diagnostics::Diagnostic, source::TextSpan};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileStRequest {
    pub contract_version: u32,
    pub source: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerPosition {
    pub line: u32,
    pub column: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerSpan {
    pub start: CompilerPosition,
    pub end: CompilerPosition,
}
impl From<TextSpan> for CompilerSpan {
    fn from(span: TextSpan) -> Self {
        Self {
            start: CompilerPosition {
                line: span.start().line(),
                column: span.start().column(),
            },
            end: CompilerPosition {
                line: span.end().line(),
                column: span.end().column(),
            },
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerDiagnostic {
    pub code: String,
    #[ts(type = "\"error\" | \"warning\" | \"info\"")]
    pub severity: String,
    pub message: String,
    pub span: Option<CompilerSpan>,
}
impl From<&Diagnostic> for CompilerDiagnostic {
    fn from(d: &Diagnostic) -> Self {
        Self {
            code: d.code().into(),
            severity: match d.severity() {
                plc_types::diagnostics::Severity::Error => "error",
                plc_types::diagnostics::Severity::Warning => "warning",
                plc_types::diagnostics::Severity::Info => "info",
            }
            .into(),
            message: d.message().into(),
            span: d
                .source()
                .and_then(|s| s.text_span())
                .map(CompilerSpan::from),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerSymbol {
    pub name: String,
    #[ts(type = "\"BOOL\" | \"INT\" | \"REAL\" | \"TIME\" | \"TON\"")]
    pub type_name: String,
    #[ts(type = "\"Input\" | \"Output\" | \"Local\" | \"FB Instance\"")]
    pub role: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileSuccess {
    #[ts(type = "true")]
    pub success: bool,
    pub program_name: String,
    pub symbols: Vec<CompilerSymbol>,
    pub diagnostics: Vec<CompilerDiagnostic>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileFailure {
    #[ts(type = "false")]
    pub success: bool,
    pub diagnostics: Vec<CompilerDiagnostic>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
pub enum CompileResponse {
    Success(CompileSuccess),
    Failure(CompileFailure),
}

pub fn compile_st(request: CompileStRequest) -> Result<CompileResponse, IpcError> {
    if request.contract_version != CONTRACT_VERSION {
        return Err(IpcError {
            code: "APP-IPC-001".into(),
            message: format!(
                "IPC contract version {} is unsupported; expected {CONTRACT_VERSION}.",
                request.contract_version
            ),
        });
    }
    Ok(
        match st_frontend::compile_st(&request.source, st_frontend::CompilerConfig::default()) {
            Ok(compiled) => CompileResponse::Success(CompileSuccess {
                success: true,
                program_name: compiled.program_name().into(),
                symbols: compiled
                    .declarations()
                    .iter()
                    .map(|s| CompilerSymbol {
                        name: s.name.clone(),
                        type_name: s.type_name.clone(),
                        role: s.role.clone(),
                    })
                    .collect(),
                diagnostics: Vec::new(),
            }),
            Err(diagnostics) => CompileResponse::Failure(CompileFailure {
                success: false,
                diagnostics: diagnostics.iter().map(CompilerDiagnostic::from).collect(),
            }),
        },
    )
}
pub(crate) fn compilation_typescript(config: &ts_rs::Config) -> String {
    [
        CompileStRequest::decl(config),
        CompilerPosition::decl(config),
        CompilerSpan::decl(config),
        CompilerDiagnostic::decl(config),
        CompilerSymbol::decl(config),
        CompileSuccess::decl(config),
        CompileFailure::decl(config),
        CompileResponse::decl(config),
    ]
    .into_iter()
    .map(|d| format!("\nexport {d}\n"))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(source: &str) -> CompileStRequest {
        CompileStRequest {
            contract_version: CONTRACT_VERSION,
            source: source.into(),
        }
    }
    #[test]
    fn source_compiles_to_metadata_without_executable_data() {
        let response = compile_st(request("PROGRAM main VAR_INPUT Start : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; END_VAR VAR Delay : TON; END_VAR Motor := Start; END_PROGRAM")).unwrap();
        let CompileResponse::Success(ref success) = response else {
            panic!()
        };
        assert!(success.success);
        assert_eq!(success.program_name, "main");
        assert_eq!(success.symbols.len(), 3);
        assert_eq!(success.symbols[2].role, "FB Instance");
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(
            json.as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            ["diagnostics", "programName", "success", "symbols"]
        );
    }
    #[test]
    fn invalid_source_returns_mapped_compiler_data() {
        let CompileResponse::Failure(failure) = compile_st(request(
            "PROGRAM Main\nVAR A : BOOL; END_VAR\n(* ą 😀 *) A := Missing;\nEND_PROGRAM",
        ))
        .unwrap() else {
            panic!()
        };
        assert!(!failure.success);
        let d = &failure.diagnostics[0];
        assert_eq!(d.code, "PLC-ST-006");
        assert_eq!(d.severity, "error");
        assert_eq!(d.span.as_ref().unwrap().start.line, 3);
    }
    #[test]
    fn compile_rejects_previous_and_future_contracts() {
        for version in [0, 1, 2, CONTRACT_VERSION + 1, u32::MAX] {
            let mut r = request("PROGRAM Main END_PROGRAM");
            r.contract_version = version;
            assert_eq!(compile_st(r).unwrap_err().code, "APP-IPC-001");
        }
    }
    #[test]
    fn requests_reject_malformed_and_additional_fields() {
        for json in [
            r#"{}"#,
            r#"{"contractVersion":3,"source":1}"#,
            r#"{"contractVersion":3,"source":"","run":true}"#,
        ] {
            assert!(serde_json::from_str::<CompileStRequest>(json).is_err());
        }
        let r = request("PROGRAM Main END_PROGRAM");
        assert_eq!(
            serde_json::from_str::<CompileStRequest>(&serde_json::to_string(&r).unwrap()).unwrap(),
            r
        );
    }
    #[test]
    fn compilation_is_repeatable_and_has_no_hidden_state() {
        let r =
            request("PROGRAM Main VAR D : TON; END_VAR D(IN := TRUE, PT := T#300ms); END_PROGRAM");
        let first = compile_st(r.clone()).unwrap();
        assert_eq!(first, compile_st(r).unwrap());
    }
    #[test]
    fn source_limits_are_visible_as_data() {
        let CompileResponse::Failure(failure) = compile_st(request(&" ".repeat(65_537))).unwrap()
        else {
            panic!()
        };
        assert_eq!(failure.diagnostics[0].code, "PLC-ST-010");
    }
}

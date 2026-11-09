//! Declaration and name resolution only; no variable storage or process image.

use crate::{
    PlcType, PlcValue,
    diagnostics::{Diagnostic, RelatedIdentity, codes},
    identity::{PouId, SymbolId},
    source::SourceRef,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Global,
    Pou(PouId),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableCategory {
    Input,
    Output,
    InOut,
    Local,
}
/// Whether a variable keeps its value over a warm restart (ADR-0038).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Retention {
    #[default]
    NonRetained,
    Retained,
}
/// A variable the runtime supplies itself; programs may only read it (ADR-0038).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemVariable {
    /// TRUE during exactly the first executed scan after each start.
    FirstScan,
}
impl SystemVariable {
    pub fn name(self) -> &'static str {
        match self {
            Self::FirstScan => "FIRST_SCAN",
        }
    }
    pub fn plc_type(self) -> PlcType {
        match self {
            Self::FirstScan => PlcType::Bool,
        }
    }
    /// The system variable with this name (case-insensitive), if any.
    pub fn named(name: &str) -> Option<Self> {
        name.eq_ignore_ascii_case("FIRST_SCAN")
            .then_some(Self::FirstScan)
    }
}
/// Function/FB are ownership tags only, with no callable implementation in M1A.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PouKind {
    Program,
    Function,
    FunctionBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    InvalidName(String),
    UnknownSymbol(SymbolId),
    UnknownName(String),
    InvalidScope(Scope),
    DuplicateIdentity,
    DuplicateName(String),
    IncompatibleRedeclaration(String),
    /// Only program variables (not inputs, outputs or system variables) can be retained.
    InvalidRetention(String),
    InitialValueType {
        name: String,
        expected: PlcType,
        actual: PlcType,
    },
}
impl std::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "'{name}' is not a valid PLC Lab identifier."),
            Self::UnknownSymbol(id) => write!(f, "Symbol {} is not declared.", id.get()),
            Self::UnknownName(name) => write!(f, "'{name}' is not declared in this scope."),
            Self::InvalidScope(_) => {
                f.write_str("The declaration or lookup uses an invalid scope.")
            }
            Self::DuplicateIdentity => f.write_str("This semantic identity is already declared."),
            Self::DuplicateName(name) => {
                write!(f, "'{name}' is already declared in this namespace.")
            }
            Self::IncompatibleRedeclaration(name) => write!(
                f,
                "'{name}' is redeclared with a different type or category."
            ),
            Self::InvalidRetention(name) => write!(
                f,
                "'{name}' cannot be retained: only program variables (VAR) are retentive."
            ),
            Self::InitialValueType {
                name,
                expected,
                actual,
            } => write!(
                f,
                "The initial value of '{name}' must be {expected:?}, not {actual:?}."
            ),
        }
    }
}
impl std::error::Error for SymbolError {}
impl SymbolError {
    pub fn diagnostic(&self) -> Diagnostic {
        let code = match self {
            Self::InvalidName(_) => codes::INVALID_NAME,
            Self::UnknownSymbol(_) | Self::UnknownName(_) => codes::UNKNOWN_SYMBOL,
            Self::InvalidScope(_) => codes::INVALID_SCOPE,
            Self::DuplicateIdentity => codes::DUPLICATE_IDENTITY,
            Self::DuplicateName(_) => codes::DUPLICATE_SYMBOL,
            Self::IncompatibleRedeclaration(_) => codes::INCOMPATIBLE_REDECLARATION,
            Self::InvalidRetention(_) => codes::INVALID_RETENTION,
            Self::InitialValueType { .. } => codes::INITIAL_VALUE_TYPE,
        };
        let diagnostic = Diagnostic::compiler_error(code, self.to_string());
        match self {
            Self::UnknownSymbol(id) => diagnostic.with_related(RelatedIdentity::Symbol(*id)),
            Self::InvalidScope(Scope::Pou(id)) => {
                diagnostic.with_related(RelatedIdentity::Pou(*id))
            }
            _ => diagnostic,
        }
    }
}

/// A variable or instance path: identifiers separated by dots, as the compiler generates
/// for instance memory (`Pump1.Running`) and call temporaries (`Scale#3.Raw`, where a
/// `#<number>` suffix marks a call site). Source code cannot write these names: the lexer
/// splits at dots and never produces `#` (ADR-0044).
pub(crate) fn checked_path(name: String) -> Result<String, SymbolError> {
    let valid = name.split('.').all(|segment| {
        let (base, call) = segment.split_once('#').unwrap_or((segment, ""));
        checked_name(base.to_owned()).is_ok()
            && (!segment.contains('#')
                || (!call.is_empty() && call.bytes().all(|b| b.is_ascii_digit())))
    });
    if valid {
        Ok(name)
    } else {
        Err(SymbolError::InvalidName(name))
    }
}

pub(crate) fn checked_name(name: String) -> Result<String, SymbolError> {
    let mut bytes = name.bytes();
    if !bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(SymbolError::InvalidName(name));
    }
    Ok(name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pou {
    id: PouId,
    name: String,
    kind: PouKind,
}
impl Pou {
    pub fn new(id: PouId, name: impl Into<String>, kind: PouKind) -> Result<Self, SymbolError> {
        Ok(Self {
            id,
            name: checked_name(name.into())?,
            kind,
        })
    }
    pub fn id(&self) -> PouId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> PouKind {
        self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    id: SymbolId,
    name: String,
    plc_type: PlcType,
    scope: Scope,
    category: VariableCategory,
    source: Option<SourceRef>,
    retention: Retention,
    initial: Option<PlcValue>,
    system: Option<SystemVariable>,
}
impl Symbol {
    pub fn new(
        id: SymbolId,
        name: impl Into<String>,
        plc_type: PlcType,
        scope: Scope,
        category: VariableCategory,
        source: Option<SourceRef>,
    ) -> Result<Self, SymbolError> {
        if scope == Scope::Global && category != VariableCategory::Local {
            return Err(SymbolError::InvalidScope(scope));
        }
        Ok(Self {
            id,
            name: checked_path(name.into())?,
            plc_type,
            scope,
            category,
            source,
            retention: Retention::NonRetained,
            initial: None,
            system: None,
        })
    }
    /// The read-only variable the runtime supplies, owned by `scope`.
    pub fn system_variable(
        id: SymbolId,
        variable: SystemVariable,
        scope: Scope,
    ) -> Result<Self, SymbolError> {
        let mut symbol = Self::new(
            id,
            variable.name(),
            variable.plc_type(),
            scope,
            VariableCategory::Local,
            None,
        )?;
        symbol.system = Some(variable);
        Ok(symbol)
    }
    pub fn with_retention(mut self, retention: Retention) -> Result<Self, SymbolError> {
        if retention == Retention::Retained
            && (self.category != VariableCategory::Local || self.system.is_some())
        {
            return Err(SymbolError::InvalidRetention(self.name));
        }
        self.retention = retention;
        Ok(self)
    }
    pub fn with_initial(mut self, value: PlcValue) -> Result<Self, SymbolError> {
        if value.plc_type() != self.plc_type {
            return Err(SymbolError::InitialValueType {
                name: self.name,
                expected: self.plc_type,
                actual: value.plc_type(),
            });
        }
        self.initial = Some(value);
        Ok(self)
    }
    pub fn retention(&self) -> Retention {
        self.retention
    }
    /// The declared initial value, else the type's default.
    pub fn initial_value(&self) -> PlcValue {
        self.initial
            .unwrap_or_else(|| self.plc_type.default_value())
    }
    pub fn declared_initial(&self) -> Option<PlcValue> {
        self.initial
    }
    pub fn system(&self) -> Option<SystemVariable> {
        self.system
    }
    pub fn id(&self) -> SymbolId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn plc_type(&self) -> PlcType {
        self.plc_type
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn category(&self) -> VariableCategory {
        self.category
    }
    pub fn source(&self) -> Option<SourceRef> {
        self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolTable {
    pous: BTreeMap<PouId, Pou>,
    pou_names: BTreeMap<String, PouId>,
    symbols: BTreeMap<SymbolId, Symbol>,
    names: BTreeMap<(Scope, String), SymbolId>,
}
impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn declare_pou(&mut self, pou: Pou) -> Result<(), SymbolError> {
        if self.pous.contains_key(&pou.id) {
            return Err(SymbolError::DuplicateIdentity);
        }
        let key = pou.name.to_ascii_uppercase();
        if self.pou_names.contains_key(&key) {
            return Err(SymbolError::DuplicateName(pou.name));
        }
        self.pou_names.insert(key, pou.id);
        self.pous.insert(pou.id, pou);
        Ok(())
    }
    fn check_scope(&self, scope: Scope) -> Result<(), SymbolError> {
        if let Scope::Pou(id) = scope
            && !self.pous.contains_key(&id)
        {
            return Err(SymbolError::InvalidScope(scope));
        }
        Ok(())
    }
    pub fn declare(&mut self, symbol: Symbol) -> Result<(), SymbolError> {
        self.check_scope(symbol.scope)?;
        if self.symbols.contains_key(&symbol.id) {
            return Err(SymbolError::DuplicateIdentity);
        }
        let key = (symbol.scope, symbol.name.to_ascii_uppercase());
        if let Some(id) = self.names.get(&key) {
            let original = &self.symbols[id]; // The two indexes mutate together below.
            return Err(
                if original.plc_type == symbol.plc_type && original.category == symbol.category {
                    SymbolError::DuplicateName(symbol.name)
                } else {
                    SymbolError::IncompatibleRedeclaration(symbol.name)
                },
            );
        }
        self.names.insert(key, symbol.id);
        self.symbols.insert(symbol.id, symbol);
        Ok(())
    }
    pub fn get(&self, id: SymbolId) -> Result<&Symbol, SymbolError> {
        self.symbols.get(&id).ok_or(SymbolError::UnknownSymbol(id))
    }
    pub fn pou(&self, id: PouId) -> Option<&Pou> {
        self.pous.get(&id)
    }
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
    pub fn resolve(&self, scope: Scope, name: &str) -> Result<&Symbol, SymbolError> {
        self.check_scope(scope)?;
        let key = checked_path(name.into())?.to_ascii_uppercase();
        let id = self
            .names
            .get(&(scope, key.clone()))
            .or_else(|| self.names.get(&(Scope::Global, key)))
            .ok_or_else(|| SymbolError::UnknownName(name.into()))?;
        Ok(&self.symbols[id])
    }
    pub fn visible(&self, scope: Scope, id: SymbolId) -> Result<&Symbol, SymbolError> {
        self.check_scope(scope)?;
        let symbol = self.get(id)?;
        if symbol.scope != Scope::Global && symbol.scope != scope {
            return Err(SymbolError::InvalidScope(scope));
        }
        Ok(symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pid(n: u32) -> PouId {
        PouId::new(n).unwrap()
    }
    fn sid(n: u32) -> SymbolId {
        SymbolId::new(n).unwrap()
    }
    fn table() -> SymbolTable {
        let mut table = SymbolTable::new();
        for (n, name, kind) in [
            (1, "Main", PouKind::Program),
            (2, "Other", PouKind::Program),
            (3, "FutureFunction", PouKind::Function),
            (4, "FutureBlock", PouKind::FunctionBlock),
        ] {
            table
                .declare_pou(Pou::new(pid(n), name, kind).unwrap())
                .unwrap();
        }
        table
    }
    fn symbol(n: u32, name: &str, t: PlcType, scope: Scope, category: VariableCategory) -> Symbol {
        Symbol::new(sid(n), name, t, scope, category, None).unwrap()
    }
    #[test]
    fn resolution_is_ascii_case_insensitive_local_then_global() {
        let mut table = table();
        table
            .declare(symbol(
                1,
                "Motor",
                PlcType::Bool,
                Scope::Global,
                VariableCategory::Local,
            ))
            .unwrap();
        table
            .declare(symbol(
                2,
                "motor",
                PlcType::Int,
                Scope::Pou(pid(1)),
                VariableCategory::Local,
            ))
            .unwrap();
        assert_eq!(
            table.resolve(Scope::Pou(pid(1)), "MOTOR").unwrap().id(),
            sid(2)
        );
        assert_eq!(
            table.resolve(Scope::Pou(pid(2)), "mOtOr").unwrap().id(),
            sid(1)
        );
        assert_eq!(table.resolve(Scope::Global, "motor").unwrap().id(), sid(1));
        assert_eq!(table.get(sid(2)).unwrap().name(), "motor");
        assert!(table.visible(Scope::Pou(pid(2)), sid(2)).is_err());
        assert!(table.visible(Scope::Pou(pid(1)), sid(1)).is_ok());
    }
    #[test]
    fn all_variable_categories_and_owner_tags_are_representable() {
        let mut table = table();
        for (n, category) in [
            VariableCategory::Input,
            VariableCategory::Output,
            VariableCategory::InOut,
            VariableCategory::Local,
        ]
        .into_iter()
        .enumerate()
        {
            let n = n as u32 + 1;
            table
                .declare(symbol(
                    n,
                    &format!("V{n}"),
                    PlcType::Time,
                    Scope::Pou(pid(n)),
                    category,
                ))
                .unwrap();
            let s = table.get(sid(n)).unwrap();
            assert_eq!(s.category(), category);
            assert_eq!(s.scope(), Scope::Pou(pid(n)));
        }
        assert_eq!(table.pou(pid(3)).unwrap().kind(), PouKind::Function);
        assert_eq!(table.pou(pid(4)).unwrap().name(), "FutureBlock");
    }
    #[test]
    fn duplicates_redeclarations_unknown_names_and_invalid_scopes_are_explicit() {
        let mut table = table();
        let scope = Scope::Pou(pid(1));
        table
            .declare(symbol(
                1,
                "Start",
                PlcType::Bool,
                scope,
                VariableCategory::Input,
            ))
            .unwrap();
        assert_eq!(
            table.declare(symbol(
                1,
                "Other",
                PlcType::Bool,
                scope,
                VariableCategory::Local
            )),
            Err(SymbolError::DuplicateIdentity)
        );
        assert_eq!(
            table.declare(symbol(
                2,
                "START",
                PlcType::Bool,
                scope,
                VariableCategory::Input
            )),
            Err(SymbolError::DuplicateName("START".into()))
        );
        assert_eq!(
            table.declare(symbol(
                2,
                "Start",
                PlcType::Real,
                scope,
                VariableCategory::Input
            )),
            Err(SymbolError::IncompatibleRedeclaration("Start".into()))
        );
        assert_eq!(
            table.declare(symbol(
                2,
                "Start",
                PlcType::Bool,
                scope,
                VariableCategory::Output
            )),
            Err(SymbolError::IncompatibleRedeclaration("Start".into()))
        );
        assert_eq!(table.get(sid(2)), Err(SymbolError::UnknownSymbol(sid(2))));
        assert_eq!(
            table.resolve(scope, "Missing"),
            Err(SymbolError::UnknownName("Missing".into()))
        );
        assert_eq!(
            table.resolve(Scope::Pou(pid(99)), "Start"),
            Err(SymbolError::InvalidScope(Scope::Pou(pid(99))))
        );
        assert!(
            table
                .declare(symbol(
                    2,
                    "X",
                    PlcType::Bool,
                    Scope::Pou(pid(99)),
                    VariableCategory::Local
                ))
                .is_err()
        );
        assert!(
            Symbol::new(
                sid(2),
                "X",
                PlcType::Bool,
                Scope::Global,
                VariableCategory::Input,
                None
            )
            .is_err()
        );
        assert_eq!(table.symbols().count(), 1);
        assert_eq!(
            table.declare_pou(Pou::new(pid(9), "MAIN", PouKind::Program).unwrap()),
            Err(SymbolError::DuplicateName("MAIN".into()))
        );
        assert_eq!(
            table.declare_pou(Pou::new(pid(1), "New", PouKind::Program).unwrap()),
            Err(SymbolError::DuplicateIdentity)
        );
    }
    #[test]
    fn identifier_policy_and_diagnostic_codes_are_deliberate() {
        // Dotted paths name the members of user function block instances and function
        // call temporaries (ADR-0044); every segment must still be an identifier.
        for name in [
            "",
            " ",
            "1motor",
            "tank-level",
            "żółć",
            "A..B",
            ".A",
            "A.",
            "A.1b",
            "A#",
            "A#x",
            "#1",
            "A#1#2",
            "A.b-c",
        ] {
            let error = Symbol::new(
                sid(1),
                name,
                PlcType::Bool,
                Scope::Global,
                VariableCategory::Local,
                None,
            )
            .unwrap_err();
            assert_eq!(error.diagnostic().code(), "PLC-COMP-006", "{name}");
        }
        for name in ["_Motor2", "Pump1.Running", "Line.Pump1.Edge", "Both#3.A"] {
            assert!(
                Symbol::new(
                    sid(1),
                    name,
                    PlcType::Bool,
                    Scope::Global,
                    VariableCategory::Local,
                    None
                )
                .is_ok(),
                "{name}"
            );
        }
        assert_eq!(
            SymbolError::UnknownSymbol(sid(7)).diagnostic().related(),
            Some(RelatedIdentity::Symbol(sid(7)))
        );
        assert_eq!(
            SymbolError::UnknownName("X".into()).diagnostic().code(),
            "PLC-COMP-001"
        );
        assert_eq!(
            SymbolError::DuplicateName("X".into()).diagnostic().code(),
            "PLC-COMP-002"
        );
        assert_eq!(
            SymbolError::InvalidScope(Scope::Global).diagnostic().code(),
            "PLC-COMP-003"
        );
        assert_eq!(
            SymbolError::IncompatibleRedeclaration("X".into())
                .diagnostic()
                .code(),
            "PLC-COMP-004"
        );
        assert_eq!(
            SymbolError::DuplicateIdentity.diagnostic().code(),
            "PLC-COMP-005"
        );
    }
    #[test]
    fn identity_does_not_depend_on_visible_name() {
        let old = symbol(
            5,
            "Motor",
            PlcType::Bool,
            Scope::Global,
            VariableCategory::Local,
        );
        let renamed = symbol(
            5,
            "Drive",
            PlcType::Bool,
            Scope::Global,
            VariableCategory::Local,
        );
        assert_eq!(old.id(), renamed.id());
        assert_ne!(old.name(), renamed.name());
    }
}

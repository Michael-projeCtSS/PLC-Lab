//! Declaration and name resolution only; no variable storage or process image.

use crate::{
    PlcType,
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
            name: checked_name(name.into())?,
            plc_type,
            scope,
            category,
            source,
        })
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
        let key = checked_name(name.into())?.to_ascii_uppercase();
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
        for name in ["", " ", "1motor", "tank-level", "żółć", "A.B"] {
            let error = Symbol::new(
                sid(1),
                name,
                PlcType::Bool,
                Scope::Global,
                VariableCategory::Local,
                None,
            )
            .unwrap_err();
            assert_eq!(error.diagnostic().code(), "PLC-COMP-006");
        }
        assert!(
            Symbol::new(
                sid(1),
                "_Motor2",
                PlcType::Bool,
                Scope::Global,
                VariableCategory::Local,
                None
            )
            .is_ok()
        );
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

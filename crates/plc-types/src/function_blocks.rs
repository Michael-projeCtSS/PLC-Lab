//! Semantic declarations only. Standard behavior and persistent state belong to plc-runtime.
use crate::{
    PlcType,
    diagnostics::{Diagnostic, codes},
    identity::{FunctionBlockId, FunctionBlockInstanceId, ParameterId, PouId},
    source::SourceRef,
    symbols::{SymbolError, checked_name, checked_path},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterDirection {
    Input,
    Output,
    InOut,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StandardFunctionBlock {
    Ton,
    Tof,
    Tp,
    RTrig,
    FTrig,
    Ctu,
    Ctd,
    /// Educational PID controller (ADR-0046).
    Pid,
}

/// One parameter of a standard block signature. IDs are local to the signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardParameter {
    pub id: u32,
    pub name: &'static str,
    pub direction: ParameterDirection,
    pub plc_type: PlcType,
    pub required: bool,
}

const fn param(
    id: u32,
    name: &'static str,
    direction: ParameterDirection,
    plc_type: PlcType,
    required: bool,
) -> StandardParameter {
    StandardParameter {
        id,
        name,
        direction,
        plc_type,
        required,
    }
}
use ParameterDirection::{Input as IN, Output as OUT};
const TIMER: [StandardParameter; 4] = [
    param(1, "IN", IN, PlcType::Bool, true),
    param(2, "PT", IN, PlcType::Time, true),
    param(3, "Q", OUT, PlcType::Bool, false),
    param(4, "ET", OUT, PlcType::Time, false),
];
const TRIGGER: [StandardParameter; 2] = [
    param(1, "CLK", IN, PlcType::Bool, true),
    param(2, "Q", OUT, PlcType::Bool, false),
];
const UP_COUNTER: [StandardParameter; 5] = [
    param(1, "CU", IN, PlcType::Bool, true),
    param(2, "R", IN, PlcType::Bool, false),
    param(3, "PV", IN, PlcType::Int, true),
    param(4, "Q", OUT, PlcType::Bool, false),
    param(5, "CV", OUT, PlcType::Int, false),
];
/// ADR-0046. Optional inputs keep their last bound value; their initial values are
/// set by the runtime (KP 1.0, MV_MAX 100.0, ENABLE TRUE, the rest zero or FALSE).
const PID: [StandardParameter; 18] = [
    param(1, "SP", IN, PlcType::Real, true),
    param(2, "PV", IN, PlcType::Real, true),
    param(3, "KP", IN, PlcType::Real, false),
    param(4, "TI", IN, PlcType::Time, false),
    param(5, "TD", IN, PlcType::Time, false),
    param(6, "MV_MIN", IN, PlcType::Real, false),
    param(7, "MV_MAX", IN, PlcType::Real, false),
    param(8, "MANUAL", IN, PlcType::Bool, false),
    param(9, "MV_MAN", IN, PlcType::Real, false),
    param(10, "ENABLE", IN, PlcType::Bool, false),
    param(11, "RESET", IN, PlcType::Bool, false),
    param(12, "TS", IN, PlcType::Time, false),
    param(13, "MV", OUT, PlcType::Real, false),
    param(14, "E", OUT, PlcType::Real, false),
    param(15, "P", OUT, PlcType::Real, false),
    param(16, "I", OUT, PlcType::Real, false),
    param(17, "D", OUT, PlcType::Real, false),
    param(18, "SAT", OUT, PlcType::Bool, false),
];
const DOWN_COUNTER: [StandardParameter; 5] = [
    param(1, "CD", IN, PlcType::Bool, true),
    param(2, "LD", IN, PlcType::Bool, false),
    param(3, "PV", IN, PlcType::Int, true),
    param(4, "Q", OUT, PlcType::Bool, false),
    param(5, "CV", OUT, PlcType::Int, false),
];

impl StandardFunctionBlock {
    pub const ALL: [Self; 8] = [
        Self::Ton,
        Self::Tof,
        Self::Tp,
        Self::RTrig,
        Self::FTrig,
        Self::Ctu,
        Self::Ctd,
        Self::Pid,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Ton => "TON",
            Self::Tof => "TOF",
            Self::Tp => "TP",
            Self::RTrig => "R_TRIG",
            Self::FTrig => "F_TRIG",
            Self::Ctu => "CTU",
            Self::Ctd => "CTD",
            Self::Pid => "PID",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        let upper = name.to_ascii_uppercase();
        Self::ALL.into_iter().find(|k| k.name() == upper)
    }
    pub fn signature(self) -> &'static [StandardParameter] {
        match self {
            Self::Ton | Self::Tof | Self::Tp => &TIMER,
            Self::RTrig | Self::FTrig => &TRIGGER,
            Self::Ctu => &UP_COUNTER,
            Self::Ctd => &DOWN_COUNTER,
            Self::Pid => &PID,
        }
    }
    /// Inputs the runtime reports with the outputs, so a controller's setpoint,
    /// measurement, limits and tuning can be observed online (ADR-0049).
    pub fn observed_inputs(self) -> &'static [&'static str] {
        match self {
            Self::Pid => &[
                "SP", "PV", "KP", "TI", "TD", "MV_MIN", "MV_MAX", "MANUAL", "MV_MAN",
            ],
            _ => &[],
        }
    }
    pub fn parameter_named(self, name: &str) -> Option<StandardParameter> {
        self.signature()
            .iter()
            .copied()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionBlockKind {
    Standard(StandardFunctionBlock),
    UserDefined,
}

/// Standard signature identity; IDs are local to a definition, never its semantic kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TonParameter {
    In,
    Pt,
    Q,
    Et,
}
impl TonParameter {
    pub fn id(self) -> ParameterId {
        ParameterId::new(match self {
            Self::In => 1,
            Self::Pt => 2,
            Self::Q => 3,
            Self::Et => 4,
        })
        .expect("fixed standard parameter identity")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterDefinition {
    id: ParameterId,
    name: String,
    direction: ParameterDirection,
    plc_type: PlcType,
    required: bool,
}
impl ParameterDefinition {
    pub fn new(
        id: ParameterId,
        name: impl Into<String>,
        direction: ParameterDirection,
        plc_type: PlcType,
        required: bool,
    ) -> Result<Self, FunctionBlockDeclarationError> {
        Ok(Self {
            id,
            name: checked_name(name.into()).map_err(FunctionBlockDeclarationError::InvalidName)?,
            direction,
            plc_type,
            required,
        })
    }
    pub fn id(&self) -> ParameterId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn direction(&self) -> ParameterDirection {
        self.direction
    }
    pub fn plc_type(&self) -> PlcType {
        self.plc_type
    }
    pub fn required(&self) -> bool {
        self.required
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBlockDefinition {
    id: FunctionBlockId,
    kind: FunctionBlockKind,
    parameters: Vec<ParameterDefinition>,
    source: Option<SourceRef>,
}
impl FunctionBlockDefinition {
    pub fn ton(id: FunctionBlockId) -> Self {
        Self::standard(id, StandardFunctionBlock::Ton)
    }
    pub fn standard(id: FunctionBlockId, kind: StandardFunctionBlock) -> Self {
        Self {
            id,
            kind: FunctionBlockKind::Standard(kind),
            parameters: kind
                .signature()
                .iter()
                .map(|p| ParameterDefinition {
                    id: ParameterId::new(p.id).expect("fixed standard parameter identity"),
                    name: p.name.into(),
                    direction: p.direction,
                    plc_type: p.plc_type,
                    required: p.required,
                })
                .collect(),
            source: None,
        }
    }
    /// Future signature metadata only; there is no custom body or user-FB execution.
    pub fn user_defined(
        id: FunctionBlockId,
        parameters: Vec<ParameterDefinition>,
    ) -> Result<Self, FunctionBlockDeclarationError> {
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for parameter in &parameters {
            if !ids.insert(parameter.id) {
                return Err(FunctionBlockDeclarationError::DuplicateIdentity);
            }
            if !names.insert(parameter.name.to_ascii_uppercase()) {
                return Err(FunctionBlockDeclarationError::DuplicateName);
            }
        }
        Ok(Self {
            id,
            kind: FunctionBlockKind::UserDefined,
            parameters,
            source: None,
        })
    }
    pub fn with_source(mut self, source: SourceRef) -> Self {
        self.source = Some(source);
        self
    }
    pub fn id(&self) -> FunctionBlockId {
        self.id
    }
    pub fn kind(&self) -> FunctionBlockKind {
        self.kind
    }
    pub fn parameters(&self) -> &[ParameterDefinition] {
        &self.parameters
    }
    pub fn parameter(&self, id: ParameterId) -> Option<&ParameterDefinition> {
        self.parameters.iter().find(|p| p.id == id)
    }
    pub fn source(&self) -> Option<SourceRef> {
        self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBlockInstance {
    id: FunctionBlockInstanceId,
    definition: FunctionBlockId,
    owner: PouId,
    name: String,
    source: Option<SourceRef>,
}
impl FunctionBlockInstance {
    pub fn new(
        id: FunctionBlockInstanceId,
        definition: FunctionBlockId,
        owner: PouId,
        name: impl Into<String>,
        source: Option<SourceRef>,
    ) -> Result<Self, FunctionBlockDeclarationError> {
        Ok(Self {
            id,
            definition,
            owner,
            name: checked_path(name.into()).map_err(FunctionBlockDeclarationError::InvalidName)?,
            source,
        })
    }
    pub fn id(&self) -> FunctionBlockInstanceId {
        self.id
    }
    pub fn definition(&self) -> FunctionBlockId {
        self.definition
    }
    pub fn owner(&self) -> PouId {
        self.owner
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn source(&self) -> Option<SourceRef> {
        self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionBlockCatalog {
    definitions: BTreeMap<FunctionBlockId, FunctionBlockDefinition>,
    instances: BTreeMap<FunctionBlockInstanceId, FunctionBlockInstance>,
    names: BTreeSet<(PouId, String)>,
}
impl FunctionBlockCatalog {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn declare_definition(
        &mut self,
        definition: FunctionBlockDefinition,
    ) -> Result<(), FunctionBlockDeclarationError> {
        if self.definitions.contains_key(&definition.id) {
            return Err(FunctionBlockDeclarationError::DuplicateIdentity);
        }
        self.definitions.insert(definition.id, definition);
        Ok(())
    }
    /// Forward references are allowed during assembly; validation resolves them before use.
    pub fn declare_instance(
        &mut self,
        instance: FunctionBlockInstance,
    ) -> Result<(), FunctionBlockDeclarationError> {
        if self.instances.contains_key(&instance.id) {
            return Err(FunctionBlockDeclarationError::DuplicateIdentity);
        }
        let key = (instance.owner, instance.name.to_ascii_uppercase());
        if self.names.contains(&key) {
            return Err(FunctionBlockDeclarationError::DuplicateName);
        }
        self.names.insert(key);
        self.instances.insert(instance.id, instance);
        Ok(())
    }
    pub fn definition(&self, id: FunctionBlockId) -> Option<&FunctionBlockDefinition> {
        self.definitions.get(&id)
    }
    pub fn instance(&self, id: FunctionBlockInstanceId) -> Option<&FunctionBlockInstance> {
        self.instances.get(&id)
    }
    pub fn definitions(&self) -> impl Iterator<Item = &FunctionBlockDefinition> {
        self.definitions.values()
    }
    pub fn instances(&self) -> impl Iterator<Item = &FunctionBlockInstance> {
        self.instances.values()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionBlockDeclarationError {
    InvalidName(SymbolError),
    DuplicateIdentity,
    DuplicateName,
}
impl FunctionBlockDeclarationError {
    pub fn diagnostic(&self) -> Diagnostic {
        match self {
            Self::InvalidName(error) => error.diagnostic(),
            Self::DuplicateIdentity => Diagnostic::compiler_error(
                codes::DUPLICATE_IDENTITY,
                "A function-block declaration or parameter identity is duplicated.",
            ),
            Self::DuplicateName => Diagnostic::compiler_error(
                codes::DUPLICATE_SYMBOL,
                "A function-block instance or parameter name is duplicated in its namespace.",
            ),
        }
    }
}
impl std::fmt::Display for FunctionBlockDeclarationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.diagnostic().message())
    }
}
impl std::error::Error for FunctionBlockDeclarationError {}

#[cfg(test)]
mod tests {
    use super::*;
    fn did(n: u32) -> FunctionBlockId {
        FunctionBlockId::new(n).unwrap()
    }
    fn instance(n: u32, definition: u32, name: &str) -> FunctionBlockInstance {
        FunctionBlockInstance::new(
            FunctionBlockInstanceId::new(n).unwrap(),
            did(definition),
            PouId::new(1).unwrap(),
            name,
            None,
        )
        .unwrap()
    }
    #[test]
    fn ton_signature_is_authoritative_typed_and_independent_of_definition_id() {
        let definition = FunctionBlockDefinition::ton(did(77));
        assert_eq!(
            definition.kind(),
            FunctionBlockKind::Standard(StandardFunctionBlock::Ton)
        );
        let expected = [
            (1, "IN", ParameterDirection::Input, PlcType::Bool, true),
            (2, "PT", ParameterDirection::Input, PlcType::Time, true),
            (3, "Q", ParameterDirection::Output, PlcType::Bool, false),
            (4, "ET", ParameterDirection::Output, PlcType::Time, false),
        ];
        for (p, (id, name, direction, t, required)) in definition.parameters().iter().zip(expected)
        {
            assert_eq!(
                (
                    p.id().get(),
                    p.name(),
                    p.direction(),
                    p.plc_type(),
                    p.required()
                ),
                (id, name, direction, t, required)
            );
            assert_eq!(definition.parameter(p.id()), Some(p));
        }
        assert!(
            definition
                .parameter(ParameterId::new(99).unwrap())
                .is_none()
        );
    }
    #[test]
    fn instance_definition_mapping_is_unique_and_duplicate_rejection_atomic() {
        let mut catalog = FunctionBlockCatalog::new();
        catalog.declare_instance(instance(1, 77, "Timer")).unwrap();
        catalog
            .declare_definition(FunctionBlockDefinition::ton(did(77)))
            .unwrap();
        assert_eq!(
            catalog.declare_instance(instance(1, 88, "Other")),
            Err(FunctionBlockDeclarationError::DuplicateIdentity)
        );
        assert_eq!(
            catalog.declare_instance(instance(2, 88, "TIMER")),
            Err(FunctionBlockDeclarationError::DuplicateName)
        );
        assert_eq!(
            catalog.declare_definition(FunctionBlockDefinition::ton(did(77))),
            Err(FunctionBlockDeclarationError::DuplicateIdentity)
        );
        assert_eq!(
            catalog
                .instance(FunctionBlockInstanceId::new(1).unwrap())
                .unwrap()
                .definition(),
            did(77)
        );
        assert_eq!(catalog.instances().count(), 1);
        assert_eq!(catalog.definitions().count(), 1);
        assert_eq!(
            catalog
                .instance(FunctionBlockInstanceId::new(1).unwrap())
                .unwrap()
                .name(),
            "Timer"
        );
    }
    #[test]
    fn future_signature_metadata_supports_inout_without_executing_it() {
        let p = ParameterDefinition::new(
            ParameterId::new(9).unwrap(),
            "Value",
            ParameterDirection::InOut,
            PlcType::Int,
            true,
        )
        .unwrap();
        let definition = FunctionBlockDefinition::user_defined(did(1), vec![p.clone()]).unwrap();
        assert_eq!(definition.kind(), FunctionBlockKind::UserDefined);
        assert_eq!(
            FunctionBlockDefinition::user_defined(did(1), vec![p.clone(), p]),
            Err(FunctionBlockDeclarationError::DuplicateIdentity)
        );
        assert!(
            ParameterDefinition::new(
                ParameterId::new(1).unwrap(),
                " ",
                ParameterDirection::Input,
                PlcType::Bool,
                true
            )
            .is_err()
        );
        assert_eq!(
            FunctionBlockDeclarationError::DuplicateIdentity
                .diagnostic()
                .code(),
            "PLC-COMP-005"
        );
    }
}

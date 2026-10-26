//! Stable signal identities and descriptors. Metadata is stored once per session;
//! observations and traces reference signals only by [`SignalId`].

use plc_types::PlcValue;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalValue {
    Bool(bool),
    Int(i16),
    Real(f32),
    /// PLC TIME in microseconds.
    Time(u64),
    /// Physical integer quantity (millilitres, permille, microamperes, ...).
    Count(i64),
}
impl trace_core::TraceValue for SignalValue {}
impl SignalValue {
    pub fn from_plc(value: PlcValue) -> Self {
        match value {
            PlcValue::Bool(b) => Self::Bool(b),
            PlcValue::Int(i) => Self::Int(i),
            PlcValue::Real(r) => Self::Real(r.get()),
            PlcValue::Time(t) => Self::Time(t.as_micros()),
        }
    }
    pub fn to_plc(self) -> Option<PlcValue> {
        Some(match self {
            Self::Bool(b) => PlcValue::Bool(b),
            Self::Int(i) => PlcValue::Int(i),
            Self::Real(r) => PlcValue::real(r).ok()?,
            Self::Time(t) => PlcValue::Time(plc_types::PlcTime::from_micros(t)),
            Self::Count(_) => return None,
        })
    }
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Bool(b) => f64::from(u8::from(b)),
            Self::Int(i) => f64::from(i),
            Self::Real(r) => f64::from(r),
            Self::Time(t) => t as f64,
            Self::Count(c) => c as f64,
        }
    }
    pub fn value_type(self) -> ValueType {
        match self {
            Self::Bool(_) => ValueType::Bool,
            Self::Int(_) => ValueType::Int,
            Self::Real(_) => ValueType::Real,
            Self::Time(_) => ValueType::Time,
            Self::Count(_) => ValueType::Count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueType {
    Bool,
    Int,
    Real,
    Time,
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalKind {
    PlcInput,
    PlcOutput,
    PlcLocal,
    BlockOutput,
    Physical,
    Channel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalDescriptor {
    pub id: SignalId,
    /// Unique path, e.g. `plc.StartPB`, `plc.Delay.Q`, `process.tank.levelMl`, `io.R0/S2/CH0`.
    pub path: String,
    pub label: String,
    pub kind: SignalKind,
    pub value_type: ValueType,
    pub unit: Option<&'static str>,
    /// False when the signal belongs to a replaced program generation.
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SignalRegistry {
    descriptors: Vec<SignalDescriptor>,
    by_path: BTreeMap<String, SignalId>,
}

impl SignalRegistry {
    /// Return the existing ID for an identical path/type or allocate a new one.
    pub fn ensure(&mut self, path: String, label: String, kind: SignalKind, value_type: ValueType, unit: Option<&'static str>) -> SignalId {
        if let Some(id) = self.by_path.get(&path).copied() {
            let d = &mut self.descriptors[id.0 as usize];
            if d.kind == kind && d.value_type == value_type {
                d.active = true;
                d.label = label;
                return id;
            }
        }
        let id = SignalId(u32::try_from(self.descriptors.len()).unwrap_or(u32::MAX));
        self.descriptors.push(SignalDescriptor { id, path: path.clone(), label, kind, value_type, unit, active: true });
        self.by_path.insert(path, id);
        id
    }
    pub fn retire(&mut self, predicate: impl Fn(&SignalDescriptor) -> bool) {
        for d in &mut self.descriptors {
            if predicate(d) {
                d.active = false;
            }
        }
    }
    pub fn get(&self, id: SignalId) -> Option<&SignalDescriptor> {
        self.descriptors.get(id.0 as usize)
    }
    pub fn by_path(&self, path: &str) -> Option<SignalId> {
        self.by_path.get(path).copied()
    }
    pub fn iter(&self) -> impl Iterator<Item = &SignalDescriptor> {
        self.descriptors.iter()
    }
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

pub fn unit_of(quantity: &str) -> Option<&'static str> {
    if quantity.ends_with("Ml") {
        Some("ml")
    } else if quantity.ends_with("Permille") {
        Some("‰")
    } else if quantity.ends_with("MlPerS") {
        Some("ml/s")
    } else if quantity.ends_with("Ma") {
        Some("mA")
    } else if quantity == "signal" {
        Some("µA/mV")
    } else {
        None
    }
}

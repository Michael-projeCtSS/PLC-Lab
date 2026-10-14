//! Values, not evaluation. Arithmetic and clock behavior are outside M1A.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PlcType {
    Bool,
    Int,
    Real,
    Time,
}

impl PlcType {
    /// Authoritative deterministic defaults for the generic Alpha profile.
    pub fn default_value(self) -> PlcValue {
        match self {
            Self::Bool => PlcValue::Bool(false),
            Self::Int => PlcValue::Int(0),
            Self::Real => PlcValue::Real(PlcReal(0.0)),
            Self::Time => PlcValue::Time(PlcTime::ZERO),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueError {
    IntOutOfRange,
    NonFiniteReal,
    TimeOutOfRange,
}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::IntOutOfRange => "INT must be between -32768 and 32767.",
            Self::NonFiniteReal => "REAL must be a finite binary32 value.",
            Self::TimeOutOfRange => "TIME must fit a nonnegative u64 microsecond duration.",
        })
    }
}
impl std::error::Error for ValueError {}

/// Finite IEEE 754 binary32. Negative zero is normalized to positive zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f32", into = "f32")]
pub struct PlcReal(f32);

// NaN cannot be constructed, so numeric equality is reflexive.
impl Eq for PlcReal {}

impl PlcReal {
    pub fn new(value: f32) -> Result<Self, ValueError> {
        if !value.is_finite() {
            return Err(ValueError::NonFiniteReal);
        }
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }
    pub fn get(self) -> f32 {
        self.0
    }
}
impl TryFrom<f32> for PlcReal {
    type Error = ValueError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<PlcReal> for f32 {
    fn from(value: PlcReal) -> Self {
        value.get()
    }
}

/// Duration only. There is no timestamp, clock, arithmetic, or OS dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlcTime(u64);

impl PlcTime {
    pub const ZERO: Self = Self(0);
    pub fn from_micros(value: u64) -> Self {
        Self(value)
    }
    pub fn try_from_micros(value: i128) -> Result<Self, ValueError> {
        u64::try_from(value)
            .map(Self)
            .map_err(|_| ValueError::TimeOutOfRange)
    }
    pub fn as_micros(self) -> u64 {
        self.0
    }
}

// Exact machine serialization, not parsing PLC TIME literals. Matches ADR-0003/0008.
impl Serialize for PlcTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for PlcTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.is_empty()
            || !text.bytes().all(|b| b.is_ascii_digit())
            || (text.len() > 1 && text.starts_with('0'))
        {
            return Err(serde::de::Error::custom(
                "TIME requires canonical unsigned decimal microseconds.",
            ));
        }
        text.parse::<u64>()
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "UPPERCASE",
    deny_unknown_fields
)]
pub enum PlcValue {
    Bool(bool),
    Int(i16),
    Real(PlcReal),
    Time(PlcTime),
}

impl PlcValue {
    pub fn plc_type(self) -> PlcType {
        match self {
            Self::Bool(_) => PlcType::Bool,
            Self::Int(_) => PlcType::Int,
            Self::Real(_) => PlcType::Real,
            Self::Time(_) => PlcType::Time,
        }
    }
    pub fn int(value: i64) -> Result<Self, ValueError> {
        i16::try_from(value)
            .map(Self::Int)
            .map_err(|_| ValueError::IntOutOfRange)
    }
    pub fn real(value: f32) -> Result<Self, ValueError> {
        PlcReal::new(value).map(Self::Real)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_defaults_have_exact_types_and_canonical_positive_real_zero() {
        assert_eq!(PlcType::Bool.default_value(), PlcValue::Bool(false));
        assert_eq!(PlcType::Int.default_value(), PlcValue::Int(0));
        assert_eq!(PlcType::Time.default_value(), PlcValue::Time(PlcTime::ZERO));
        let PlcValue::Real(real) = PlcType::Real.default_value() else {
            panic!("REAL expected")
        };
        assert_eq!(real.get().to_bits(), 0);
        for t in [PlcType::Bool, PlcType::Int, PlcType::Real, PlcType::Time] {
            assert_eq!(t.default_value().plc_type(), t);
        }
    }

    #[test]
    fn bool_values_and_type_identity_do_not_coerce() {
        assert_ne!(PlcValue::Bool(false), PlcValue::Bool(true));
        assert_ne!(PlcValue::Bool(false), PlcValue::Int(0));
        for (value, kind) in [
            (PlcValue::Bool(true), PlcType::Bool),
            (PlcValue::Int(1), PlcType::Int),
            (PlcValue::real(1.0).unwrap(), PlcType::Real),
            (PlcValue::Time(PlcTime::ZERO), PlcType::Time),
        ] {
            assert_eq!(value.plc_type(), kind);
        }
    }

    #[test]
    fn int_boundaries_are_checked_without_wrapping() {
        for n in [-32768, -1, 0, 32767] {
            assert_eq!(PlcValue::int(n).unwrap(), PlcValue::Int(n as i16));
        }
        for n in [i64::MIN, -32769, 32768, i64::MAX] {
            assert_eq!(PlcValue::int(n), Err(ValueError::IntOutOfRange));
        }
    }

    #[test]
    fn real_is_finite_binary32_with_canonical_zero() {
        for n in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(PlcReal::new(n), Err(ValueError::NonFiniteReal));
        }
        for n in [
            f32::MIN,
            f32::MAX,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            -2.5,
        ] {
            assert_eq!(PlcReal::new(n).unwrap().get().to_bits(), n.to_bits());
        }
        assert_eq!(PlcReal::new(-0.0).unwrap().get().to_bits(), 0);
        assert_eq!(PlcReal::new(-0.0), PlcReal::new(0.0));
    }

    #[test]
    fn time_is_a_checked_nonnegative_microsecond_duration() {
        assert_eq!(PlcTime::ZERO.as_micros(), 0);
        for n in [0, 1, 5_000_000, u64::MAX] {
            assert_eq!(
                PlcTime::try_from_micros(i128::from(n)).unwrap(),
                PlcTime::from_micros(n)
            );
        }
        for n in [-1, i128::MIN, i128::from(u64::MAX) + 1] {
            assert_eq!(PlcTime::try_from_micros(n), Err(ValueError::TimeOutOfRange));
        }
    }

    #[test]
    fn typed_values_round_trip_including_large_time_and_real_extremes() {
        for value in [
            PlcValue::Bool(false),
            PlcValue::Bool(true),
            PlcValue::Int(i16::MIN),
            PlcValue::Int(i16::MAX),
            PlcValue::real(f32::MAX).unwrap(),
            PlcValue::real(f32::from_bits(1)).unwrap(),
            PlcValue::real(-0.0).unwrap(),
            PlcValue::Time(PlcTime::ZERO),
            PlcValue::Time(PlcTime::from_micros(u64::MAX)),
        ] {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(serde_json::from_str::<PlcValue>(&json).unwrap(), value);
        }
        assert_eq!(
            serde_json::to_value(PlcValue::Time(PlcTime::from_micros(u64::MAX))).unwrap()["value"],
            "18446744073709551615"
        );
    }

    #[test]
    fn deserialization_cannot_bypass_value_invariants() {
        for json in [
            r#"{"type":"INT","value":32768}"#,
            r#"{"type":"REAL","value":1e100}"#,
            r#"{"type":"TIME","value":"-1"}"#,
            r#"{"type":"TIME","value":"18446744073709551616"}"#,
            r#"{"type":"TIME","value":"5s"}"#,
            r#"{"type":"TIME","value":"01"}"#,
            r#"{"type":"TIME","value":1}"#,
            r#"{"type":"TIME","value":"+1"}"#,
            r#"{"type":"BOOL","value":1}"#,
            r#"{"type":"DINT","value":1}"#,
        ] {
            assert!(serde_json::from_str::<PlcValue>(json).is_err(), "{json}");
        }
    }
}

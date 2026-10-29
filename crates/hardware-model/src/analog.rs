//! Analog input conversion: field signal -> channel raw value, per profile representation.
//!
//! The banded integer model (nominal, overrange, underrange, overflow, underflow) uses the
//! proportions of the widely documented S7 representation: 100 % = nominal full scale N,
//! overrange up to N * 32511 / 27648 (about 117.6 %), live-zero underrange down to
//! -N * 4864 / 27648 (about -17.6 %), overflow 32767 and underflow -32768. Other
//! profiles reuse the proportions with their own N. Fidelity is declared by the profile.

use lab_model::{hardware::ChannelMode, profile::AnalogRepresentation};
use process_simulation::{AnalogField, AnalogUnit};

pub const OVERFLOW: i16 = 32767;
pub const UNDERFLOW: i16 = -32768;
/// Below this loop current a live-zero input with diagnostics reports a wire break.
pub const WIRE_BREAK_BELOW_UA: i64 = 1_185;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalogRaw {
    Int(i16),
    Real(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnalogCondition {
    Normal,
    Overrange,
    Underrange,
    Overflow,
    Underflow,
    WireBreak,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalogReading {
    pub raw: AnalogRaw,
    pub condition: AnalogCondition,
    /// What a multimeter in the channel's measuring mode would read (µA or mV).
    pub measured: i64,
}

/// Input resistance assumed when a voltage source drives a current input (approximation).
const CURRENT_INPUT_OHMS: i64 = 250;

/// Convert a field signal for a channel mode. `diagnostics` enables wire-break detection.
pub fn convert(
    field: Option<AnalogField>,
    mode: ChannelMode,
    diagnostics: bool,
    representation: AnalogRepresentation,
) -> Option<AnalogReading> {
    let (lo, hi, live_zero, unit) = match mode {
        ChannelMode::Current4To20mA => (4_000i64, 20_000i64, true, AnalogUnit::Microamp),
        ChannelMode::Current0To20mA => (0, 20_000, false, AnalogUnit::Microamp),
        ChannelMode::Voltage0To10V => (0, 10_000, false, AnalogUnit::Millivolt),
        ChannelMode::Disabled | ChannelMode::Digital => return None,
    };
    // Physical interpretation of the connected source in this measuring mode.
    let measured = match field {
        None => 0,
        Some(f) if f.unit == unit => f.value,
        // Voltage source into a low-impedance current input: I = U / R.
        Some(f) if unit == AnalogUnit::Microamp => f.value * 1_000 / CURRENT_INPUT_OHMS,
        // Current loop into a high-impedance voltage input: the loop saturates near supply.
        Some(f) if f.value > 0 => 24_000,
        Some(_) => 0,
    };
    if live_zero && diagnostics && measured < WIRE_BREAK_BELOW_UA {
        return Some(AnalogReading {
            raw: match representation {
                AnalogRepresentation::EngineeringUnits => AnalogRaw::Real(0.0),
                _ => AnalogRaw::Int(OVERFLOW),
            },
            condition: AnalogCondition::WireBreak,
            measured,
        });
    }
    let reading = match representation {
        AnalogRepresentation::EngineeringUnits => {
            let value = measured as f32 / 1_000.0;
            let condition = if measured > hi {
                AnalogCondition::Overrange
            } else if measured < lo {
                AnalogCondition::Underrange
            } else {
                AnalogCondition::Normal
            };
            AnalogReading {
                raw: AnalogRaw::Real(value),
                condition,
                measured,
            }
        }
        AnalogRepresentation::FullScale32767 => {
            let n = 32_767i64;
            let raw = scale(measured, lo, hi, n);
            let (raw, condition) = if raw > n {
                (n, AnalogCondition::Overflow)
            } else if raw < 0 {
                (0, AnalogCondition::Underflow)
            } else {
                (raw, AnalogCondition::Normal)
            };
            AnalogReading {
                raw: AnalogRaw::Int(i16::try_from(raw).unwrap_or(OVERFLOW)),
                condition,
                measured,
            }
        }
        AnalogRepresentation::PerTenThousand | AnalogRepresentation::S7Nominal27648 => {
            let n = if representation == AnalogRepresentation::S7Nominal27648 {
                27_648
            } else {
                10_000
            };
            let over = n * 32_511 / 27_648;
            let under = if live_zero { -(n * 4_864 / 27_648) } else { 0 };
            let raw = scale(measured, lo, hi, n);
            let (raw, condition) = if raw > over {
                (i64::from(OVERFLOW), AnalogCondition::Overflow)
            } else if raw > n {
                (raw, AnalogCondition::Overrange)
            } else if raw < under {
                (i64::from(UNDERFLOW), AnalogCondition::Underflow)
            } else if raw < 0 {
                (raw, AnalogCondition::Underrange)
            } else {
                (raw, AnalogCondition::Normal)
            };
            AnalogReading {
                raw: AnalogRaw::Int(i16::try_from(raw).unwrap_or(OVERFLOW)),
                condition,
                measured,
            }
        }
    };
    Some(reading)
}

/// What an analog output channel drives into the field for a commanded value: the inverse
/// of the input bands. Values beyond the overrange limit are clamped there (overflow), as
/// are values below the lower limit (underflow); engineering-unit profiles command the
/// field value directly (mA or V).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalogOutputValue {
    /// Microamperes or millivolts at the terminal.
    pub field: i64,
    pub unit: AnalogUnit,
    pub condition: AnalogCondition,
}

pub fn output(
    command: AnalogRaw,
    mode: ChannelMode,
    representation: AnalogRepresentation,
) -> Option<AnalogOutputValue> {
    let (lo, hi, live_zero, unit) = match mode {
        ChannelMode::Current4To20mA => (4_000i64, 20_000i64, true, AnalogUnit::Microamp),
        ChannelMode::Current0To20mA => (0, 20_000, false, AnalogUnit::Microamp),
        ChannelMode::Voltage0To10V => (0, 10_000, false, AnalogUnit::Millivolt),
        ChannelMode::Disabled | ChannelMode::Digital => return None,
    };
    let span = hi - lo;
    // Overrange headroom of the input bands (about 117.6 % of span), live zero down to 0.
    let max = lo + span * 32_511 / 27_648;
    let min = if live_zero { 0 } else { lo };
    let wanted = match (command, representation) {
        (AnalogRaw::Real(v), _) => (f64::from(v) * 1_000.0).round() as i64,
        (AnalogRaw::Int(raw), representation) => {
            let n = match representation {
                AnalogRepresentation::S7Nominal27648 => 27_648,
                AnalogRepresentation::FullScale32767 => 32_767,
                AnalogRepresentation::PerTenThousand | AnalogRepresentation::EngineeringUnits => {
                    10_000
                }
            };
            lo + (i64::from(raw) * span * 2 + n).div_euclid(2 * n)
        }
    };
    let (field, condition) = if wanted > max {
        (max, AnalogCondition::Overflow)
    } else if wanted > hi {
        (wanted, AnalogCondition::Overrange)
    } else if wanted < min {
        (min, AnalogCondition::Underflow)
    } else if wanted < lo {
        (wanted, AnalogCondition::Underrange)
    } else {
        (wanted, AnalogCondition::Normal)
    };
    Some(AnalogOutputValue {
        field,
        unit,
        condition,
    })
}

/// Round-half-up linear scaling of `x` in `[lo, hi]` to `[0, n]`.
fn scale(x: i64, lo: i64, hi: i64, n: i64) -> i64 {
    let numerator = (i128::from(x) - i128::from(lo)) * i128::from(n) * 2 + i128::from(hi - lo);
    let denominator = i128::from(hi - lo) * 2;
    i64::try_from(numerator.div_euclid(denominator)).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lab_model::scene::AnalogSignal;

    fn current(ua: i64) -> Option<AnalogField> {
        Some(AnalogField {
            unit: AnalogUnit::Microamp,
            value: ua,
            signal: AnalogSignal::Current4To20mA,
            loop_powered: true,
        })
    }
    #[test]
    fn outputs_drive_the_inverse_of_the_input_bands() {
        let out = |raw: i16, mode| {
            let o = output(
                AnalogRaw::Int(raw),
                mode,
                AnalogRepresentation::PerTenThousand,
            )
            .unwrap();
            (o.field, o.condition)
        };
        assert_eq!(
            out(0, ChannelMode::Current4To20mA),
            (4_000, AnalogCondition::Normal)
        );
        assert_eq!(
            out(5_000, ChannelMode::Current4To20mA),
            (12_000, AnalogCondition::Normal)
        );
        assert_eq!(
            out(10_000, ChannelMode::Voltage0To10V),
            (10_000, AnalogCondition::Normal)
        );
        assert_eq!(
            out(11_000, ChannelMode::Voltage0To10V).1,
            AnalogCondition::Overrange
        );
        assert_eq!(
            out(i16::MAX, ChannelMode::Current0To20mA).1,
            AnalogCondition::Overflow
        );
        assert_eq!(
            out(-1_000, ChannelMode::Current4To20mA),
            (2_400, AnalogCondition::Underrange)
        );
        assert_eq!(
            out(-20_000, ChannelMode::Voltage0To10V),
            (0, AnalogCondition::Underflow)
        );
        assert!(
            output(
                AnalogRaw::Int(0),
                ChannelMode::Disabled,
                AnalogRepresentation::PerTenThousand
            )
            .is_none()
        );
        // An input reads back what an output drives.
        let field = output(
            AnalogRaw::Int(2_500),
            ChannelMode::Current4To20mA,
            AnalogRepresentation::S7Nominal27648,
        )
        .unwrap();
        let back = convert(
            Some(AnalogField {
                unit: field.unit,
                value: field.field,
                signal: AnalogSignal::Current4To20mA,
                loop_powered: false,
            }),
            ChannelMode::Current4To20mA,
            false,
            AnalogRepresentation::S7Nominal27648,
        )
        .unwrap();
        assert_eq!(back.raw, AnalogRaw::Int(2_500));
    }

    fn int(r: AnalogReading) -> (i16, AnalogCondition) {
        let AnalogRaw::Int(v) = r.raw else {
            panic!("integer expected")
        };
        (v, r.condition)
    }

    #[test]
    fn s7_style_bands_for_live_zero_current() {
        let s7 = AnalogRepresentation::S7Nominal27648;
        let c = |ua| int(convert(current(ua), ChannelMode::Current4To20mA, false, s7).unwrap());
        assert_eq!(c(4_000), (0, AnalogCondition::Normal));
        assert_eq!(c(12_000), (13_824, AnalogCondition::Normal));
        assert_eq!(c(20_000), (27_648, AnalogCondition::Normal));
        assert_eq!(c(21_000).1, AnalogCondition::Overrange);
        assert_eq!(c(23_000), (OVERFLOW, AnalogCondition::Overflow));
        assert_eq!(c(3_000).1, AnalogCondition::Underrange);
        assert_eq!(c(1_000), (UNDERFLOW, AnalogCondition::Underflow));
        let diag = int(convert(current(0), ChannelMode::Current4To20mA, true, s7).unwrap());
        assert_eq!(diag, (OVERFLOW, AnalogCondition::WireBreak));
    }

    #[test]
    fn range_mismatch_shifts_the_reading_like_real_commissioning() {
        let s7 = AnalogRepresentation::S7Nominal27648;
        // A 4-20 mA transmitter at 0 % read by a channel configured 0-20 mA.
        let r = int(convert(current(4_000), ChannelMode::Current0To20mA, false, s7).unwrap());
        assert_eq!(r, (5_530, AnalogCondition::Normal));
        // A current loop wired to a voltage channel saturates.
        let v = int(convert(current(12_000), ChannelMode::Voltage0To10V, false, s7).unwrap());
        assert_eq!(v.1, AnalogCondition::Overflow);
    }

    #[test]
    fn other_representations() {
        let g = int(convert(
            current(12_000),
            ChannelMode::Current4To20mA,
            false,
            AnalogRepresentation::PerTenThousand,
        )
        .unwrap());
        assert_eq!(g, (5_000, AnalogCondition::Normal));
        let b = int(convert(
            current(20_000),
            ChannelMode::Current4To20mA,
            false,
            AnalogRepresentation::FullScale32767,
        )
        .unwrap());
        assert_eq!(b, (32_767, AnalogCondition::Normal));
        let e = convert(
            current(8_000),
            ChannelMode::Current4To20mA,
            false,
            AnalogRepresentation::EngineeringUnits,
        )
        .unwrap();
        assert_eq!(e.raw, AnalogRaw::Real(8.0));
        assert!(
            convert(
                current(8_000),
                ChannelMode::Disabled,
                false,
                AnalogRepresentation::S7Nominal27648
            )
            .is_none()
        );
    }
}

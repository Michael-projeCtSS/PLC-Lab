use crate::ast::Literal;
use plc_types::{PlcTime, PlcValue};

pub(crate) fn value(literal: &Literal, negative: bool) -> Result<PlcValue, &'static str> {
    match literal {
        Literal::Bool(v) if !negative => Ok(PlcValue::Bool(*v)),
        Literal::Integer(raw) => {
            let magnitude = raw
                .parse::<i64>()
                .map_err(|_| "INT literal is outside -32768..32767.")?;
            let number = if negative {
                magnitude
                    .checked_neg()
                    .ok_or("INT literal is outside -32768..32767.")?
            } else {
                magnitude
            };
            PlcValue::int(number).map_err(|_| "INT literal is outside -32768..32767.")
        }
        Literal::Real(raw) => {
            let number = raw
                .parse::<f32>()
                .map_err(|_| "Expected a decimal REAL literal.")?;
            PlcValue::real(if negative { -number } else { number })
                .map_err(|_| "REAL literal must be finite binary32.")
        }
        Literal::Time(raw) if !negative => time(raw).map(PlcValue::Time),
        _ => Err("This literal cannot have a numeric minus sign."),
    }
}

/// Exact integer microseconds; components strictly descend and cannot repeat.
pub(crate) fn time(raw: &str) -> Result<PlcTime, &'static str> {
    let raw = raw.to_ascii_lowercase();
    let bytes = raw.as_bytes();
    let mut cursor = 0;
    let mut previous = 6;
    let mut micros = 0u64;
    if bytes.is_empty() {
        return Err("TIME requires nonnegative integer components, such as T#300ms.");
    }
    while cursor < bytes.len() {
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start {
            return Err(
                "TIME components must be nonnegative integers; fractions and signs are unsupported.",
            );
        }
        let amount = raw[start..cursor]
            .parse::<u64>()
            .map_err(|_| "TIME literal overflows microseconds.")?;
        let tail = &raw[cursor..];
        let (unit, rank, factor) = if tail.starts_with("ms") {
            ("ms", 1, 1_000)
        } else if tail.starts_with("us") {
            ("us", 0, 1)
        } else if tail.starts_with('d') {
            ("d", 5, 86_400_000_000)
        } else if tail.starts_with('h') {
            ("h", 4, 3_600_000_000)
        } else if tail.starts_with('m') {
            ("m", 3, 60_000_000)
        } else if tail.starts_with('s') {
            ("s", 2, 1_000_000)
        } else {
            return Err("TIME units are d, h, m, s, ms and us; nanoseconds are unsupported.");
        };
        if rank >= previous {
            return Err("TIME components must descend d, h, m, s, ms, us without repetition.");
        }
        previous = rank;
        micros = amount
            .checked_mul(factor)
            .and_then(|n| micros.checked_add(n))
            .ok_or("TIME literal overflows microseconds.")?;
        cursor += unit.len();
    }
    Ok(PlcTime::from_micros(micros))
}

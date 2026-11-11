//! Literal operands of ladder elements, in IEC syntax, typed by where they are used. The
//! rules match Structured Text literals: INT is -32768..32767, REAL a finite binary32,
//! TIME `T#` or `TIME#` with integer components that descend d, h, m, s, ms, us.
use plc_types::{PlcTime, PlcType, PlcValue};

/// The value of literal `text` of type `kind`, or why it is not one.
pub fn parse(text: &str, kind: PlcType) -> Result<PlcValue, String> {
    let text = text.trim();
    match kind {
        PlcType::Bool => match text.to_ascii_uppercase().as_str() {
            "TRUE" => Ok(PlcValue::Bool(true)),
            "FALSE" => Ok(PlcValue::Bool(false)),
            _ => Err(format!("'{text}' is not a BOOL literal (TRUE or FALSE).")),
        },
        PlcType::Int => text
            .parse::<i64>()
            .ok()
            .and_then(|n| PlcValue::int(n).ok())
            .ok_or_else(|| format!("'{text}' is not an INT literal in -32768..32767.")),
        PlcType::Real => {
            let valid = !text.is_empty()
                && text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E'));
            text.parse::<f32>()
                .ok()
                .filter(|_| valid)
                .and_then(|n| PlcValue::real(n).ok())
                .ok_or_else(|| format!("'{text}' is not a finite REAL literal."))
        }
        PlcType::Time => time(text).map(PlcValue::Time),
    }
}

fn time(text: &str) -> Result<PlcTime, String> {
    let upper = text.to_ascii_uppercase();
    let raw = upper
        .strip_prefix("TIME#")
        .or_else(|| upper.strip_prefix("T#"))
        .ok_or_else(|| format!("'{text}' is not a TIME literal; write it like T#5s or T#1m30s."))?
        .to_ascii_lowercase()
        .replace('_', "");
    let bytes = raw.as_bytes();
    if bytes.is_empty() {
        return Err(format!("'{text}' has no TIME components, such as T#300ms."));
    }
    let mut cursor = 0;
    let mut previous = 6;
    let mut micros = 0u64;
    while cursor < bytes.len() {
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start {
            return Err(format!(
                "'{text}': TIME components are nonnegative integers with a unit."
            ));
        }
        let amount = raw[start..cursor]
            .parse::<u64>()
            .map_err(|_| format!("'{text}' overflows TIME."))?;
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
            return Err(format!("'{text}': TIME units are d, h, m, s, ms and us."));
        };
        if rank >= previous {
            return Err(format!(
                "'{text}': TIME components must descend d, h, m, s, ms, us without repetition."
            ));
        }
        previous = rank;
        micros = amount
            .checked_mul(factor)
            .and_then(|n| micros.checked_add(n))
            .ok_or_else(|| format!("'{text}' overflows TIME."))?;
        cursor += unit.len();
    }
    Ok(PlcTime::from_micros(micros))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_are_typed_by_their_use() {
        assert_eq!(parse("true", PlcType::Bool), Ok(PlcValue::Bool(true)));
        assert_eq!(parse(" 10 ", PlcType::Int), Ok(PlcValue::Int(10)));
        assert_eq!(parse("-32768", PlcType::Int), Ok(PlcValue::Int(-32768)));
        assert!(parse("40000", PlcType::Int).is_err());
        assert!(parse("1.5", PlcType::Int).is_err());
        assert_eq!(
            parse("2.5", PlcType::Real),
            PlcValue::real(2.5).map_err(|_| String::new())
        );
        assert!(parse("inf", PlcType::Real).is_err());
        assert!(parse("NaN", PlcType::Real).is_err());
        assert_eq!(
            parse("T#1m30s", PlcType::Time),
            Ok(PlcValue::Time(PlcTime::from_micros(90_000_000)))
        );
        assert_eq!(
            parse("time#500ms", PlcType::Time),
            Ok(PlcValue::Time(PlcTime::from_micros(500_000)))
        );
        assert!(parse("5s", PlcType::Time).is_err(), "prefix required");
        assert!(parse("T#5s5s", PlcType::Time).is_err());
        assert!(parse("T#1.5s", PlcType::Time).is_err());
        assert!(parse("T#", PlcType::Time).is_err());
    }
}

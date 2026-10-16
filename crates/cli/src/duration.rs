use std::time::Duration;

/// Parses durations like `45m`, `2h`, `1d12h` or `2h30m`.
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let error = || format!("invalid duration {input:?}; use e.g. 45m, 2h, 1d12h");
    let mut total: u64 = 0;
    let mut number = String::new();
    for c in input.trim().chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let unit = match c.to_ascii_lowercase() {
            'd' => 86_400,
            'h' => 3_600,
            'm' => 60,
            's' => 1,
            _ => return Err(error()),
        };
        let value: u64 = number.parse().map_err(|_| error())?;
        total = value
            .checked_mul(unit)
            .and_then(|secs| total.checked_add(secs))
            .ok_or_else(error)?;
        number.clear();
    }
    if !number.is_empty() || total == 0 {
        return Err(error());
    }
    Ok(Duration::from_secs(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!(parse_duration("45m"), Ok(Duration::from_mins(45)));
        assert_eq!(parse_duration("2h30m"), Ok(Duration::from_mins(150)));
        assert_eq!(parse_duration("1d12h"), Ok(Duration::from_hours(36)));
        assert_eq!(parse_duration("90s"), Ok(Duration::from_secs(90)));
        assert_eq!(parse_duration("2H"), Ok(Duration::from_hours(2)));
    }

    #[test]
    fn rejects_garbage() {
        for input in ["", "10", "h", "0m", "1x", "1h30", "99999999999999999999d"] {
            assert!(parse_duration(input).is_err(), "{input}");
        }
    }
}

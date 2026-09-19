/// A single cron field, expanded into the concrete set of values it
/// matches. Cron fields are small enough (max 60 values, for minutes)
/// that a `Vec<u32>` beats a bitset in code clarity for no real cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub values: Vec<u32>,
    /// `true` for day-of-month/day-of-week when the field text is `*`
    /// (or `?`) — needed for cron's OR-not-AND rule between those two
    /// fields specifically (see `schedule::day_matches`).
    pub is_wildcard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub minute: Field,
    pub hour: Field,
    pub day_of_month: Field,
    pub month: Field,
    pub day_of_week: Field,
}

const MONTH_NAMES: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DOW_NAMES: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

fn name_to_num(token: &str, names: &[&str]) -> Option<u32> {
    let lower = token.to_ascii_lowercase();
    names.iter().position(|n| *n == lower).map(|i| i as u32)
}

fn parse_atom(token: &str, min: u32, max: u32, names: &[&str]) -> Result<u32, String> {
    if let Ok(n) = token.parse::<u32>() {
        return Ok(n);
    }
    name_to_num(token, names)
        .ok_or_else(|| format!("'{token}' is not a valid value in range {min}-{max}"))
}

/// Parses one `,`-separated cron field (e.g. `*/15`, `1-5`, `MON-FRI`,
/// `0,15,30,45`) into its expanded value set, clamped to `[min, max]`.
/// `is_dow` selects cron's day-of-week quirk where both `0` and `7` mean
/// Sunday; it's a parameter rather than inferred from `names` so the
/// mapping is explicit at every call site.
pub fn parse_field(text: &str, min: u32, max: u32, names: &[&str]) -> Result<Field, String> {
    parse_field_inner(text, min, max, names, false)
}

fn parse_dow_field(text: &str) -> Result<Field, String> {
    parse_field_inner(text, 0, 6, &DOW_NAMES, true)
}

fn parse_field_inner(
    text: &str,
    min: u32,
    max: u32,
    names: &[&str],
    is_dow: bool,
) -> Result<Field, String> {
    let is_wildcard = text == "*" || text == "?";
    let mut values = Vec::new();

    for part in text.split(',') {
        let (range_part, step) = match part.split_once('/') {
            Some((r, s)) => (
                r,
                s.parse::<u32>()
                    .map_err(|_| format!("'{s}' is not a valid step"))?,
            ),
            None => (part, 1),
        };
        if step == 0 {
            return Err("step of 0 would loop forever".to_string());
        }

        let (lo, hi) = if range_part == "*" || range_part == "?" {
            (min, max)
        } else if let Some((a, b)) = range_part.split_once('-') {
            (
                parse_atom(a, min, max, names)?,
                parse_atom(b, min, max, names)?,
            )
        } else {
            let v = parse_atom(range_part, min, max, names)?;
            (v, v)
        };
        if lo > hi {
            return Err(format!("range {lo}-{hi} is backwards"));
        }

        let mut v = lo;
        while v <= hi {
            // Cron treats both 0 and 7 as Sunday for day-of-week.
            let normalized = if is_dow && v == 7 { 0 } else { v };
            if normalized < min || normalized > max {
                return Err(format!("{normalized} is out of range {min}-{max}"));
            }
            if !values.contains(&normalized) {
                values.push(normalized);
            }
            v += step;
        }
    }

    values.sort_unstable();
    Ok(Field {
        values,
        is_wildcard,
    })
}

/// Expands `@hourly`/`@daily`/etc. shortcuts to their 5-field equivalent,
/// same mapping cron(8) itself uses.
fn expand_shorthand(expr: &str) -> Option<&'static str> {
    match expr.trim() {
        "@yearly" | "@annually" => Some("0 0 1 1 *"),
        "@monthly" => Some("0 0 1 * *"),
        "@weekly" => Some("0 0 * * 0"),
        "@daily" | "@midnight" => Some("0 0 * * *"),
        "@hourly" => Some("0 * * * *"),
        _ => None,
    }
}

/// Parses a standard 5-field cron expression (`minute hour dom month dow`)
/// or one of the `@`-shorthand forms.
pub fn parse_schedule(expr: &str) -> Result<Schedule, String> {
    let expanded;
    let expr = match expand_shorthand(expr) {
        Some(e) => e,
        None => {
            expanded = expr.trim().to_string();
            expanded.as_str()
        }
    };

    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(format!(
            "expected 5 fields (minute hour day-of-month month day-of-week), got {}: '{expr}'",
            fields.len()
        ));
    }

    Ok(Schedule {
        minute: parse_field(fields[0], 0, 59, &[])?,
        hour: parse_field(fields[1], 0, 23, &[])?,
        day_of_month: parse_field(fields[2], 1, 31, &[])?,
        month: parse_field(fields[3], 1, 12, &MONTH_NAMES)?,
        day_of_week: parse_dow_field(fields[4])?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_expands_to_full_range() {
        let f = parse_field("*", 0, 59, &[]).unwrap();
        assert_eq!(f.values.len(), 60);
        assert!(f.is_wildcard);
    }

    #[test]
    fn step_expands_correctly() {
        let f = parse_field("*/15", 0, 59, &[]).unwrap();
        assert_eq!(f.values, vec![0, 15, 30, 45]);
    }

    #[test]
    fn range_expands_correctly() {
        let f = parse_field("9-17", 0, 23, &[]).unwrap();
        assert_eq!(f.values, (9..=17).collect::<Vec<_>>());
    }

    #[test]
    fn comma_list_and_names_both_work() {
        let f = parse_field("MON,WED,FRI", 0, 6, &DOW_NAMES).unwrap();
        assert_eq!(f.values, vec![1, 3, 5]);
    }

    #[test]
    fn dow_seven_normalizes_to_sunday_zero() {
        let f = parse_dow_field("7").unwrap();
        assert_eq!(f.values, vec![0]);
    }

    #[test]
    fn shorthand_expands() {
        let s = parse_schedule("@daily").unwrap();
        assert_eq!(s.hour.values, vec![0]);
        assert_eq!(s.minute.values, vec![0]);
    }

    #[test]
    fn wrong_field_count_is_a_clean_error() {
        assert!(parse_schedule("* * *").is_err());
    }

    #[test]
    fn out_of_range_value_is_a_clean_error() {
        assert!(parse_field("99", 0, 59, &[]).is_err());
    }

    #[test]
    fn zero_step_is_a_clean_error() {
        assert!(parse_field("*/0", 0, 59, &[]).is_err());
    }
}

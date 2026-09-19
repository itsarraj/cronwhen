use crate::parse::{Field, Schedule};

const DOW_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn list_of(values: &[u32], names: Option<&[&str]>) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|v| match names {
            Some(n) => n[*v as usize].to_string(),
            None => v.to_string(),
        })
        .collect();
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => format!(
            "{}, and {}",
            items[..items.len() - 1].join(", "),
            items[items.len() - 1]
        ),
    }
}

/// A step field (`*/N`) is detected structurally, not by re-parsing the
/// original text: any field whose values start at `min`, are evenly
/// spaced by a constant `N > 1`, and span through to `max` reads the
/// same as an explicitly-typed `*/N`, whether or not that's literally
/// how it was written — `0,5,10,...,55` and `*/5` are the same set.
fn step_of(field: &Field, min: u32, max: u32) -> Option<u32> {
    if field.values.len() < 2 || field.values[0] != min {
        return None;
    }
    let step = field.values[1] - field.values[0];
    let evenly_spaced = field.values.windows(2).all(|w| w[1] - w[0] == step);
    let spans_to_max = *field.values.last().unwrap() + step > max;
    if step > 1 && evenly_spaced && spans_to_max {
        Some(step)
    } else {
        None
    }
}

/// Best-effort plain-English description of a cron schedule. This
/// covers the common shapes (every N minutes/hours, a fixed daily time,
/// specific weekdays, a specific day-of-month) with a real sentence;
/// anything less common falls back to a plain per-field readout rather
/// than guessing at prose — see README for exactly which shapes get the
/// nice sentence.
pub fn describe(schedule: &Schedule) -> String {
    let min_step = step_of(&schedule.minute, 0, 59);
    let hour_step = step_of(&schedule.hour, 0, 23);

    if schedule.minute.is_wildcard
        && min_step.is_none()
        && schedule.hour.is_wildcard
        && hour_step.is_none()
        && schedule.day_of_month.is_wildcard
        && schedule.month.is_wildcard
        && schedule.day_of_week.is_wildcard
    {
        return "every minute".to_string();
    }

    if let Some(n) = min_step {
        if schedule.hour.is_wildcard
            && schedule.day_of_month.is_wildcard
            && schedule.month.is_wildcard
            && schedule.day_of_week.is_wildcard
        {
            return format!("every {n} minutes");
        }
    }

    if let Some(n) = hour_step {
        if schedule.minute.values == [0]
            && schedule.day_of_month.is_wildcard
            && schedule.month.is_wildcard
            && schedule.day_of_week.is_wildcard
        {
            return format!("every {n} hours, on the hour");
        }
    }

    if schedule.minute.values.len() == 1 && schedule.hour.values.len() == 1 {
        let time = format!(
            "{:02}:{:02}",
            schedule.hour.values[0], schedule.minute.values[0]
        );

        let day_part = match (
            schedule.day_of_month.is_wildcard,
            schedule.day_of_week.is_wildcard,
            schedule.month.is_wildcard,
        ) {
            (true, true, true) => "every day".to_string(),
            (true, false, true) => format!(
                "on {}",
                list_of(&schedule.day_of_week.values, Some(&DOW_NAMES))
            ),
            (false, true, true) => format!(
                "on day {} of the month",
                list_of(&schedule.day_of_month.values, None)
            ),
            (true, true, false) => format!(
                "every day in {}",
                list_of(
                    &schedule
                        .month
                        .values
                        .iter()
                        .map(|m| m - 1)
                        .collect::<Vec<_>>(),
                    Some(&MONTH_NAMES)
                )
            ),
            _ => return describe_fields_plainly(schedule),
        };

        return format!("at {time} {day_part}");
    }

    describe_fields_plainly(schedule)
}

fn describe_fields_plainly(schedule: &Schedule) -> String {
    format!(
        "minute={} hour={} day-of-month={} month={} day-of-week={}",
        field_summary(&schedule.minute),
        field_summary(&schedule.hour),
        field_summary(&schedule.day_of_month),
        field_summary(&schedule.month),
        field_summary(&schedule.day_of_week),
    )
}

fn field_summary(field: &Field) -> String {
    if field.is_wildcard && field.values.len() > 1 {
        "*".to_string()
    } else {
        list_of(&field.values, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_schedule;

    #[test]
    fn every_minute() {
        assert_eq!(
            describe(&parse_schedule("* * * * *").unwrap()),
            "every minute"
        );
    }

    #[test]
    fn every_n_minutes() {
        assert_eq!(
            describe(&parse_schedule("*/5 * * * *").unwrap()),
            "every 5 minutes"
        );
    }

    #[test]
    fn daily_at_a_fixed_time() {
        assert_eq!(
            describe(&parse_schedule("30 9 * * *").unwrap()),
            "at 09:30 every day"
        );
    }

    #[test]
    fn specific_weekdays() {
        assert_eq!(
            describe(&parse_schedule("0 8 * * MON,WED,FRI").unwrap()),
            "at 08:00 on Monday, Wednesday, and Friday"
        );
    }

    #[test]
    fn specific_day_of_month() {
        assert_eq!(
            describe(&parse_schedule("0 0 1 * *").unwrap()),
            "at 00:00 on day 1 of the month"
        );
    }

    #[test]
    fn unusual_shape_falls_back_to_plain_field_readout_without_panicking() {
        let text = describe(&parse_schedule("15,45 */3 1-15 * MON").unwrap());
        assert!(text.contains("minute="));
    }
}

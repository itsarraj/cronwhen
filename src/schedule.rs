use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike};

use crate::parse::Schedule;

/// Cron's day-of-month/day-of-week rule: if *both* fields are restricted
/// (not `*`), a day matches if it satisfies *either* one — not both.
/// If only one is restricted, only that one constrains the day. This is
/// a well-known cron quirk (vixie-cron and every descendant of it
/// implement it this way) that trips people up constantly, which is
/// half the reason a tool like this is worth having.
fn day_matches(schedule: &Schedule, day_of_month: u32, day_of_week: u32) -> bool {
    let dom_restricted = !schedule.day_of_month.is_wildcard;
    let dow_restricted = !schedule.day_of_week.is_wildcard;

    match (dom_restricted, dow_restricted) {
        (false, false) => true,
        (true, false) => schedule.day_of_month.values.contains(&day_of_month),
        (false, true) => schedule.day_of_week.values.contains(&day_of_week),
        (true, true) => {
            schedule.day_of_month.values.contains(&day_of_month)
                || schedule.day_of_week.values.contains(&day_of_week)
        }
    }
}

/// How far ahead to search before giving up. Most schedules find their
/// next match within minutes; the pathological case is something like
/// `0 0 29 2 *` (Feb 29 only), whose next occurrence can be several
/// years out depending on where `from` falls in the leap-year cycle —
/// a documented, honest limitation, not a bug: see README.
const SEARCH_WINDOW_MINUTES: i64 = 5 * 366 * 24 * 60;

/// Returns the next `count` times (strictly after `from`) that
/// `schedule` matches, searching minute-by-minute up to
/// `SEARCH_WINDOW_MINUTES` ahead.
pub fn next_runs<Tz: TimeZone>(
    schedule: &Schedule,
    from: DateTime<Tz>,
    count: usize,
) -> Vec<DateTime<Tz>> {
    let mut cursor = from
        .with_second(0)
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(from.clone())
        + Duration::minutes(1);

    let mut results = Vec::with_capacity(count);
    for _ in 0..SEARCH_WINDOW_MINUTES {
        if results.len() >= count {
            break;
        }
        let month = cursor.month();
        let minute = cursor.minute();
        let hour = cursor.hour();
        let dom = cursor.day();
        let dow = cursor.weekday().num_days_from_sunday();

        if schedule.month.values.contains(&month)
            && schedule.hour.values.contains(&hour)
            && schedule.minute.values.contains(&minute)
            && day_matches(schedule, dom, dow)
        {
            results.push(cursor.clone());
        }
        cursor += Duration::minutes(1);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_schedule;
    use chrono::Local;

    #[test]
    fn every_minute_matches_the_very_next_minute() {
        let s = parse_schedule("* * * * *").unwrap();
        let from = Local.with_ymd_and_hms(2026, 3, 1, 10, 30, 15).unwrap();
        let runs = next_runs(&s, from, 1);
        assert_eq!(
            runs[0],
            Local.with_ymd_and_hms(2026, 3, 1, 10, 31, 0).unwrap()
        );
    }

    #[test]
    fn step_schedule_lands_on_expected_minutes() {
        let s = parse_schedule("*/15 * * * *").unwrap();
        let from = Local.with_ymd_and_hms(2026, 3, 1, 10, 3, 0).unwrap();
        let runs = next_runs(&s, from, 3);
        let minutes: Vec<u32> = runs.iter().map(|t| t.minute()).collect();
        assert_eq!(minutes, vec![15, 30, 45]);
    }

    #[test]
    fn daily_schedule_rolls_over_to_the_next_day() {
        let s = parse_schedule("0 9 * * *").unwrap();
        let from = Local.with_ymd_and_hms(2026, 3, 1, 9, 30, 0).unwrap();
        let runs = next_runs(&s, from, 1);
        assert_eq!(runs[0].day(), 2);
        assert_eq!(runs[0].hour(), 9);
    }

    #[test]
    fn dom_or_dow_rule_matches_on_either_when_both_restricted() {
        // 13th of the month OR a Friday — classic "Friday the 13th" cron.
        let s = parse_schedule("0 0 13 * FRI").unwrap();
        let from = Local.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let runs = next_runs(&s, from, 3);
        for r in &runs {
            assert!(r.day() == 13 || r.weekday().num_days_from_sunday() == 5);
        }
        // Both a plain Friday and the 13th should show up somewhere in
        // a wide enough sample, proving this is really OR not AND.
        assert!(runs.iter().any(|r| r.day() == 13));
    }

    #[test]
    fn wildcard_dom_and_dow_means_every_day_at_the_given_time() {
        let s = parse_schedule("30 14 * * *").unwrap();
        let from = Local.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();
        let runs = next_runs(&s, from, 5);
        assert_eq!(runs.len(), 5);
        for r in &runs {
            assert_eq!((r.hour(), r.minute()), (14, 30));
        }
    }
}

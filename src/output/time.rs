//! Shared human-readable time formatting used by both CLI display and JSON `_readable` fields.

use chrono::{DateTime, Datelike, Local, Timelike, Utc};

/// Format an absolute time as a rich human-readable string.
///
/// Same year:  `25th Feb at 5:00pm`
/// Other year: `25th Feb 2027 at 5:00pm`
pub fn format_datetime(dt: DateTime<Utc>) -> String {
    let local = dt.with_timezone(&Local);
    let now_local = Local::now();

    let day = local.day();
    let suffix = ordinal_suffix(day);
    let month = local.format("%b");
    let time = format_time_12h(local.hour(), local.minute());

    if local.year() == now_local.year() {
        format!("{day}{suffix} {month} at {time}")
    } else {
        format!("{day}{suffix} {month} {} at {time}", local.year())
    }
}

/// Format a duration between two times as a human-readable relative string.
///
/// Returns e.g. `"in 2h and 15m"` or `"3 days ago"`.
pub fn format_relative(from: DateTime<Utc>, to: DateTime<Utc>) -> String {
    let secs = (to - from).num_seconds();
    match secs.cmp(&0) {
        std::cmp::Ordering::Less => {
            format!("{} ago", format_duration_human(secs.unsigned_abs()))
        }
        std::cmp::Ordering::Equal => "now".to_string(),
        std::cmp::Ordering::Greater => {
            format!("in {}", format_duration_human(secs.unsigned_abs()))
        }
    }
}

/// Format a datetime with its relative offset: `"25th Feb at 5:00pm (in 2h and 15m)"`
pub fn format_datetime_with_relative(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
    format!("{} ({})", format_datetime(dt), format_relative(now, dt))
}

/// Format a duration as a human-readable string with "and" separator.
///
/// Examples: `"45s"`, `"12m"`, `"2h and 15m"`, `"3 days and 4h"`.
pub fn format_duration_human(total_secs: u64) -> String {
    if total_secs < 60 {
        return format!("{total_secs}s");
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;

    if days > 0 {
        let day_word = if days == 1 { "day" } else { "days" };
        if hours > 0 {
            format!("{days} {day_word} and {hours}h")
        } else if mins > 0 {
            format!("{days} {day_word} and {mins}m")
        } else {
            format!("{days} {day_word}")
        }
    } else if hours > 0 {
        if mins > 0 {
            format!("{hours}h and {mins}m")
        } else {
            format!("{hours}h")
        }
    } else {
        format!("{mins}m")
    }
}

/// Format a compact duration string (no "and"), for schedule descriptions.
pub fn format_duration_short(total_secs: u64) -> String {
    if total_secs < 60 {
        return format!("{total_secs}s");
    }
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;

    if days > 0 {
        if hours > 0 {
            format!("{days}d {hours}h")
        } else {
            format!("{days}d")
        }
    } else if hours > 0 {
        if mins > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{hours}h")
        }
    } else {
        format!("{mins}m")
    }
}

/// English ordinal suffix for a day number.
fn ordinal_suffix(day: u32) -> &'static str {
    match day % 100 {
        11..=13 => "th",
        _ => match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}

/// Format hour:minute in 12-hour format: `"5:00pm"`, `"12:30am"`.
fn format_time_12h(hour: u32, minute: u32) -> String {
    let (period, h12) = if hour == 0 {
        ("am", 12)
    } else if hour < 12 {
        ("am", hour)
    } else if hour == 12 {
        ("pm", 12)
    } else {
        ("pm", hour - 12)
    };
    format!("{h12}:{minute:02}{period}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_suffixes() {
        assert_eq!(ordinal_suffix(1), "st");
        assert_eq!(ordinal_suffix(2), "nd");
        assert_eq!(ordinal_suffix(3), "rd");
        assert_eq!(ordinal_suffix(4), "th");
        assert_eq!(ordinal_suffix(11), "th");
        assert_eq!(ordinal_suffix(12), "th");
        assert_eq!(ordinal_suffix(13), "th");
        assert_eq!(ordinal_suffix(21), "st");
        assert_eq!(ordinal_suffix(22), "nd");
        assert_eq!(ordinal_suffix(23), "rd");
        assert_eq!(ordinal_suffix(31), "st");
    }

    #[test]
    fn time_12h_format() {
        assert_eq!(format_time_12h(0, 0), "12:00am");
        assert_eq!(format_time_12h(1, 30), "1:30am");
        assert_eq!(format_time_12h(12, 0), "12:00pm");
        assert_eq!(format_time_12h(13, 45), "1:45pm");
        assert_eq!(format_time_12h(23, 59), "11:59pm");
    }

    #[test]
    fn duration_human_seconds() {
        assert_eq!(format_duration_human(0), "0s");
        assert_eq!(format_duration_human(45), "45s");
    }

    #[test]
    fn duration_human_minutes() {
        assert_eq!(format_duration_human(60), "1m");
        assert_eq!(format_duration_human(300), "5m");
    }

    #[test]
    fn duration_human_hours_and_minutes() {
        assert_eq!(format_duration_human(3600), "1h");
        assert_eq!(format_duration_human(3600 + 900), "1h and 15m");
    }

    #[test]
    fn duration_human_days() {
        assert_eq!(format_duration_human(86400), "1 day");
        assert_eq!(format_duration_human(2 * 86400), "2 days");
        assert_eq!(format_duration_human(86400 + 7200), "1 day and 2h");
    }

    #[test]
    fn relative_future() {
        let now = Utc::now();
        let future = now + chrono::Duration::hours(2) + chrono::Duration::minutes(15);
        let result = format_relative(now, future);
        assert!(result.starts_with("in "));
        assert!(result.contains("2h and 15m"));
    }

    #[test]
    fn relative_past() {
        let now = Utc::now();
        let past = now - chrono::Duration::days(3);
        let result = format_relative(now, past);
        assert!(result.ends_with(" ago"));
        assert!(result.contains("3 days"));
    }
}

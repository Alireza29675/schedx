use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Local, Utc};

use crate::model::schedule::ParsedSchedule;

/// Parse a schedule string into a `ParsedSchedule`.
///
/// Accepted formats (in order of attempt):
/// 1. Relative one-shot: `in <N><s|m|h|d>` or bare `<N><s|m|h|d>`
/// 2. Relative recurring: `every <N><s|m|h|d>` -> cron or interval
/// 3. Cron: exactly 5 space-separated fields
/// 4. ISO-8601 / RFC-3339 absolute datetime
pub fn parse_schedule(input: &str, now: DateTime<Utc>) -> Result<ParsedSchedule> {
    let input = input.trim();

    if input.is_empty() {
        bail!(
            "Error: Empty schedule string.\n\
             Accepted examples: 'in 4h', 'every 30m', '0 9 * * 1-5', '2026-03-01T14:00:00Z'"
        );
    }

    // Try relative one-shot: `in <N><unit>`
    if let Some(rest) = input.strip_prefix("in ") {
        return parse_relative_oneshot(rest.trim(), now);
    }

    // Try relative recurring: `every <N><unit>`
    if let Some(rest) = input.strip_prefix("every ") {
        return parse_relative_recurring(rest.trim());
    }

    // Try bare duration: `10s`, `5m`, `2h`, `1d` (treat as one-shot)
    if looks_like_bare_duration(input) {
        return parse_relative_oneshot(input, now);
    }

    // Try cron (5 fields)
    if looks_like_cron(input) {
        return parse_cron(input);
    }

    // Try ISO-8601 / RFC-3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        let dt_utc = dt.with_timezone(&Utc);
        if dt_utc <= now {
            bail!(
                "Error: Schedule time '{input}' is in the past.\n\
                 Provide a future timestamp."
            );
        }
        return Ok(ParsedSchedule::OneShot {
            fire_at: dt_utc,
            human: format!("once at {dt_utc}"),
        });
    }

    bail!(
        "Error: Could not parse schedule '{input}'.\n\
         Accepted examples: 'in 4h', '30s', 'every 10s', '0 9 * * 1-5', '2026-03-01T14:00:00Z'"
    );
}

/// Check if the string looks like a bare duration: digits followed by s/m/h/d
fn looks_like_bare_duration(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let unit = s.chars().last().unwrap_or(' ');
    if !matches!(unit, 's' | 'm' | 'h' | 'd') {
        return false;
    }
    let num_part = &s[..s.len() - 1];
    !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit())
}

fn parse_relative_oneshot(s: &str, now: DateTime<Utc>) -> Result<ParsedSchedule> {
    let (n, unit) = parse_duration_spec(s)?;
    if n == 0 {
        bail!(
            "Error: Could not parse schedule 'in {s}'.\n\
             Duration must be greater than zero."
        );
    }

    let duration = match unit {
        's' => Duration::seconds(i64::from(n)),
        'm' => Duration::minutes(i64::from(n)),
        'h' => Duration::hours(i64::from(n)),
        'd' => Duration::days(i64::from(n)),
        _ => unreachable!(),
    };

    let fire_at = now + duration;
    let human = format!("once in {n}{unit}");

    Ok(ParsedSchedule::OneShot { fire_at, human })
}

fn parse_relative_recurring(s: &str) -> Result<ParsedSchedule> {
    let (n, unit) = parse_duration_spec(s)?;
    if n == 0 {
        bail!(
            "Error: Could not parse schedule 'every {s}'.\n\
             Interval must be greater than zero.\n\
             Accepted examples: 'every 10s', 'every 30m', 'every 6h', 'every 2d'"
        );
    }

    // Seconds use the interval-based scheduler (cron can't do sub-minute)
    if unit == 's' {
        let every_seconds = u64::from(n);
        return Ok(ParsedSchedule::RecurringInterval {
            every_seconds,
            human: format!("every {n}s"),
        });
    }

    let (expr, human) = match unit {
        'm' => {
            if n > 59 {
                bail!(
                    "Error: Could not parse schedule 'every {s}'.\n\
                     Minute interval must be 1-59."
                );
            }
            // Sub-minute equivalent: use interval scheduler for < 1m
            (format!("*/{n} * * * *"), format!("every {n} minute(s)"))
        }
        'h' => {
            if n > 23 {
                bail!(
                    "Error: Could not parse schedule 'every {s}'.\n\
                     Hour interval must be 1-23."
                );
            }
            (format!("0 */{n} * * *"), format!("every {n} hour(s)"))
        }
        'd' => {
            if n > 30 {
                bail!(
                    "Error: Could not parse schedule 'every {s}'.\n\
                     Day interval must be 1-30."
                );
            }
            (format!("0 0 */{n} * *"), format!("every {n} day(s)"))
        }
        _ => unreachable!(),
    };

    // Validate the generated cron expression
    validate_cron_expr(&expr)?;

    Ok(ParsedSchedule::RecurringCron { expr, human })
}

fn parse_duration_spec(s: &str) -> Result<(u32, char)> {
    if s.is_empty() {
        bail!(
            "Error: Missing duration value.\n\
             Accepted examples: 'in 4h', 'every 30m', 'in 2d', '10s'"
        );
    }

    let unit = s.chars().last().unwrap_or(' ');
    if !matches!(unit, 's' | 'm' | 'h' | 'd') {
        bail!(
            "Error: Could not parse schedule duration '{s}'.\n\
             Use 's' (seconds), 'm' (minutes), 'h' (hours), or 'd' (days).\n\
             Accepted examples: 'in 10s', 'in 4h', 'every 30m', 'in 2d'"
        );
    }

    let num_str = &s[..s.len() - 1];
    let n: u32 = num_str.parse().map_err(|_| {
        anyhow::anyhow!(
            "Error: Could not parse schedule duration '{s}'.\n\
             Expected a number followed by s/m/h/d.\n\
             Accepted examples: 'in 10s', 'in 4h', 'every 30m', 'in 2d'"
        )
    })?;

    Ok((n, unit))
}

fn looks_like_cron(input: &str) -> bool {
    let fields: Vec<&str> = input.split_whitespace().collect();
    fields.len() == 5
}

fn parse_cron(input: &str) -> Result<ParsedSchedule> {
    validate_cron_expr(input)?;
    let human = describe_cron(input);
    Ok(ParsedSchedule::RecurringCron {
        expr: input.to_string(),
        human,
    })
}

fn validate_cron_expr(expr: &str) -> Result<()> {
    // The `cron` crate expects 7-field expressions (sec min hour dom month dow year).
    // We store 5-field and prepend "0 " (seconds) and append " *" (year) for validation.
    let seven_field = format!("0 {expr} *");
    seven_field.parse::<cron::Schedule>().map_err(|e| {
        anyhow::anyhow!(
            "Error: Invalid cron expression '{expr}': {e}\n\
             Accepted examples: '0 9 * * 1-5', '*/15 * * * *', '0 0 1 * *'"
        )
    })?;
    Ok(())
}

fn describe_cron(expr: &str) -> String {
    // Simple human-readable description for common patterns
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return format!("cron: {expr}");
    }

    format!("cron: {expr}")
}

/// Compute the next fire time for a cron expression after `after`.
/// Uses local timezone for cron evaluation.
pub fn next_cron_time(expr: &str, after: DateTime<Utc>) -> Result<Option<DateTime<Utc>>> {
    let seven_field = format!("0 {expr} *");
    let schedule: cron::Schedule = seven_field
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid cron expression '{expr}': {e}"))?;

    let after_local = after.with_timezone(&Local);
    let next = schedule.after(&after_local).next();
    Ok(next.map(|t| t.with_timezone(&Utc)))
}

/// Find the latest due cron time in the range `(after, now]`.
/// Returns `None` if no occurrence falls in that range.
pub fn latest_due_cron_time(
    expr: &str,
    after: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>> {
    let seven_field = format!("0 {expr} *");
    let schedule: cron::Schedule = seven_field
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid cron expression '{expr}': {e}"))?;

    let after_local = after.with_timezone(&Local);
    let now_local = now.with_timezone(&Local);

    let mut latest: Option<DateTime<Utc>> = None;
    for t in schedule.after(&after_local) {
        if t > now_local {
            break;
        }
        latest = Some(t.with_timezone(&Utc));
    }

    Ok(latest)
}

/// Compute the next fire time for a recurring interval schedule.
///
/// Returns the earliest future interval multiple after `now`.
/// If the anchor is behind, this skips over past multiples to avoid showing
/// stale "next run" times.
pub fn next_interval_time(
    every_seconds: u64,
    last_scheduled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    let anchor = last_scheduled_at.unwrap_or(created_at);
    let secs = i64::try_from(every_seconds).unwrap_or(i64::MAX);
    let elapsed = (now - anchor).num_seconds();
    if elapsed < secs {
        // Not yet overdue — next fire is just anchor + interval
        return anchor + Duration::seconds(secs);
    }
    // Skip past all overdue multiples: next future = anchor + (periods+1)*interval
    let periods = elapsed / secs;
    anchor + Duration::seconds((periods + 1) * secs)
}

/// Check if an interval-based job is due.
///
/// Returns `Some(scheduled_for)` with the **latest** due interval time in
/// `(anchor, now]`.  This ensures the job "catches up" to the current time
/// even when the dispatcher tick interval is larger than `every_seconds`.
pub fn interval_is_due(
    every_seconds: u64,
    last_scheduled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let anchor = last_scheduled_at.unwrap_or(created_at);
    let secs = i64::try_from(every_seconds).unwrap_or(i64::MAX);
    let elapsed = (now - anchor).num_seconds();
    if elapsed < secs {
        return None;
    }
    // Latest interval multiple that falls at or before `now`
    let periods = elapsed / secs;
    let latest = anchor + Duration::seconds(periods * secs);
    if latest <= now { Some(latest) } else { None }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn parse_cron_5_field() {
        let result = parse_schedule("*/15 * * * *", now()).unwrap();
        match result {
            ParsedSchedule::RecurringCron { expr, .. } => assert_eq!(expr, "*/15 * * * *"),
            _ => panic!("expected recurring cron"),
        }
    }

    #[test]
    fn parse_in_relative() {
        let result = parse_schedule("in 30m", now()).unwrap();
        match result {
            ParsedSchedule::OneShot { .. } => {}
            _ => panic!("expected one-shot"),
        }
    }

    #[test]
    fn parse_in_seconds() {
        let n = now();
        let result = parse_schedule("in 10s", n).unwrap();
        match result {
            ParsedSchedule::OneShot { fire_at, .. } => {
                let diff = (fire_at - n).num_seconds();
                assert!((9..=11).contains(&diff));
            }
            _ => panic!("expected one-shot"),
        }
    }

    #[test]
    fn parse_bare_duration() {
        let n = now();
        let result = parse_schedule("30s", n).unwrap();
        match result {
            ParsedSchedule::OneShot { fire_at, .. } => {
                let diff = (fire_at - n).num_seconds();
                assert!((29..=31).contains(&diff));
            }
            _ => panic!("expected one-shot"),
        }
    }

    #[test]
    fn parse_bare_duration_minutes() {
        let n = now();
        let result = parse_schedule("5m", n).unwrap();
        match result {
            ParsedSchedule::OneShot { fire_at, .. } => {
                let diff = (fire_at - n).num_seconds();
                assert!((299..=301).contains(&diff));
            }
            _ => panic!("expected one-shot"),
        }
    }

    #[test]
    fn parse_every_seconds() {
        let result = parse_schedule("every 10s", now()).unwrap();
        match result {
            ParsedSchedule::RecurringInterval { every_seconds, .. } => {
                assert_eq!(every_seconds, 10);
            }
            _ => panic!("expected recurring interval"),
        }
    }

    #[test]
    fn parse_every_minutes() {
        let result = parse_schedule("every 5m", now()).unwrap();
        match result {
            ParsedSchedule::RecurringCron { expr, .. } => assert_eq!(expr, "*/5 * * * *"),
            _ => panic!("expected recurring cron"),
        }
    }

    #[test]
    fn parse_every_hours() {
        let result = parse_schedule("every 2h", now()).unwrap();
        match result {
            ParsedSchedule::RecurringCron { expr, .. } => assert_eq!(expr, "0 */2 * * *"),
            _ => panic!("expected recurring cron"),
        }
    }

    #[test]
    fn parse_every_days() {
        let result = parse_schedule("every 1d", now()).unwrap();
        match result {
            ParsedSchedule::RecurringCron { expr, .. } => assert_eq!(expr, "0 0 */1 * *"),
            _ => panic!("expected recurring cron"),
        }
    }

    #[test]
    fn parse_iso_datetime() {
        let result = parse_schedule("2099-12-31T23:59:59Z", now()).unwrap();
        match result {
            ParsedSchedule::OneShot { fire_at, .. } => {
                assert_eq!(fire_at.year(), 2099);
            }
            _ => panic!("expected one-shot"),
        }
    }

    #[test]
    fn reject_empty() {
        assert!(parse_schedule("", now()).is_err());
    }

    #[test]
    fn reject_invalid() {
        assert!(parse_schedule("garbage", now()).is_err());
    }

    #[test]
    fn reject_every_zero() {
        assert!(parse_schedule("every 0h", now()).is_err());
    }

    #[test]
    fn reject_every_zero_seconds() {
        assert!(parse_schedule("every 0s", now()).is_err());
    }

    #[test]
    fn reject_every_too_large_minutes() {
        assert!(parse_schedule("every 60m", now()).is_err());
    }

    #[test]
    fn reject_past_datetime() {
        assert!(parse_schedule("2020-01-01T00:00:00Z", now()).is_err());
    }

    #[test]
    fn interval_due_when_past() {
        let created = Utc::now() - Duration::seconds(20);
        let result = interval_is_due(10, None, created, Utc::now());
        assert!(result.is_some());
    }

    #[test]
    fn interval_not_due_yet() {
        let created = Utc::now();
        let result = interval_is_due(60, None, created, Utc::now());
        assert!(result.is_none());
    }
}

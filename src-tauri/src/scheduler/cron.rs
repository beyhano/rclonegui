use chrono::{DateTime, Utc};
use cron::Schedule;

/// Parse a cron expression and return the next scheduled UTC time.
pub fn next_cron_time(expr: &str) -> Result<Option<DateTime<Utc>>, String> {
    let schedule: Schedule = expr
        .parse()
        .map_err(|e| format!("Invalid cron expression '{}': {}", expr, e))?;

    match schedule.upcoming(Utc).next() {
        Some(dt) => Ok(Some(dt)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cron_returns_next_time() {
        let result = next_cron_time("0 15 * * * *").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_invalid_cron_returns_error() {
        let result = next_cron_time("not-a-cron");
        assert!(result.is_err());
    }

    #[test]
    fn test_daily_at_midnight() {
        let result = next_cron_time("0 0 * * * *").unwrap();
        assert!(result.is_some());
    }

}

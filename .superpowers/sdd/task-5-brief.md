# Task 5: Cron parser module

**Files:**
- Create: `src-tauri/src/scheduler/cron.rs`
- Create: `src-tauri/src/scheduler/mod.rs`

**Interfaces:**
- Produces: `pub fn next_cron_time(expr: &str) -> Result<Option<DateTime<Utc>>, String>`
- Produces: `pub fn format_next_run(expr: &str) -> Result<String, String>`

### Create `scheduler/mod.rs`

```rust
pub mod cron;
pub mod engine;
pub mod scheduler;
```

Note: `engine` and `scheduler` modules are created in later tasks. This mod file just declares them.

### Create `scheduler/cron.rs`

```rust
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

/// Format the duration until the next run as a human-readable string.
pub fn format_next_run(expr: &str) -> Result<String, String> {
    match next_cron_time(expr)? {
        Some(dt) => {
            let now = Utc::now();
            let duration = dt.signed_duration_since(now);
            if duration.num_seconds() < 60 {
                Ok("in less than a minute".to_string())
            } else if duration.num_minutes() < 60 {
                Ok(format!("in {} minutes", duration.num_minutes()))
            } else if duration.num_hours() < 24 {
                Ok(format!("in {} hours", duration.num_hours()))
            } else {
                Ok(format!("in {} days", duration.num_days()))
            }
        }
        None => Ok("no upcoming run".to_string()),
    }
}
```

### Tests

```rust
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

    #[test]
    fn test_format_next_run() {
        let result = format_next_run("0 0 1 1 * *").unwrap();
        assert!(result.contains("in") || result.contains("no upcoming"));
    }

    #[test]
    fn test_invalid_format_returns_error() {
        let result = format_next_run("");
        assert!(result.is_err());
    }
}
```

### Registration

The `scheduler` module must be registered in `lib.rs`. Add `mod scheduler;` alongside the other mod declarations.

### Verification

```bash
cd src-tauri && cargo test scheduler::cron::tests -- --nocapture
```
Expected: All 5 tests PASS.

### Commit

```bash
git add -A && git commit -m "feat(scheduler): add cron parser module"
```

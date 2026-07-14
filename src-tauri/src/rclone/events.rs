/// Rclone process event stream: reads stdout/stderr, parses progress lines,
/// and emits structured Tauri events to the frontend.
///
/// # Event Namespace
///
/// | Event | Payload | Trigger |
/// |---|---|---|
/// | `rclone:progress` | `ProgressPayload` | Progress line matched |
/// | `rclone:log` | `{line}` | Non-progress stdout/stderr line |
///
/// # Regex
///
/// Captures rclone `--progress` output lines like:
/// `Transferred: 1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s`
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use uuid::Uuid;

/// Payload for the `rclone:progress` event.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub process_id: String,
    pub transferred: String,
    pub total: String,
    pub percent: u8,
    pub speed: String,
    pub eta: String,
}

/// Compile and cache the rclone progress regex.
///
/// Matches lines of the form:
/// `Transferred: 1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s`
fn progress_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"Transferred:\s+(?P<transferred>[\d.]+\s*\w*)\s+/\s+(?P<total>[\d.]+\s*\w*),\s+(?P<percent>\d+)%,\s+(?P<speed>[\d.]+\s*\w+/s)(?:,\s+ETA\s+(?P<eta>[\w\d-]+))?",
        )
        .expect("valid progress regex")
    })
}

/// Parse a single stdout line into a `ProgressPayload`.
///
/// Returns `None` if the line does not match the rclone progress format.
pub fn parse_progress_line(process_id: Uuid, line: &str) -> Option<ProgressPayload> {
    let caps = progress_regex().captures(line)?;

    Some(ProgressPayload {
        process_id: process_id.to_string(),
        transferred: caps["transferred"].to_string(),
        total: caps["total"].to_string(),
        percent: caps["percent"].parse().unwrap_or(0),
        speed: caps["speed"].to_string(),
        eta: caps
            .name("eta")
            .map(|m| m.as_str().to_string())
            .unwrap_or_default(),
    })
}

/// Start reading stdout and stderr streams from an rclone process.
///
/// Spawns two concurrent tokio tasks:
/// - **stdout**: reads lines, attempts to parse progress, emits
///   `rclone:progress` on match or `rclone:log` otherwise.
/// - **stderr**: reads lines and emits `rclone:log`.
///
/// Returns a `JoinHandle` that resolves when both streams are exhausted.
pub fn start_event_stream(
    app: AppHandle,
    process_id: Uuid,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let app_clone = app.clone();
        let pid = process_id;

        // stdout reader task
        let stdout_handle = tokio::spawn({
            let app = app_clone.clone();
            let pid = pid;
            async move {
                let mut lines = stdout.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(payload) = parse_progress_line(pid, &line) {
                        let _ = app.emit("rclone:progress", payload);
                    } else if !line.is_empty() {
                        let _ = app.emit(
                            "rclone:log",
                            serde_json::json!({ "process_id": pid.to_string(), "line": line }),
                        );
                    }
                }
            }
        });

        // stderr reader task
        let stderr_handle = tokio::spawn({
            let app = app_clone;
            let pid = pid;
            async move {
                let mut lines = stderr.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() {
                        let _ = app.emit(
                            "rclone:log",
                            serde_json::json!({ "process_id": pid.to_string(), "line": line }),
                        );
                    }
                }
            }
        });

        let _ = tokio::join!(stdout_handle, stderr_handle);
    })
}

// ----- Task 3.4 RED test: parse_progress_line with fixture lines -----

#[cfg(test)]
mod tests {
    use super::*;

    fn test_id() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    // -- Full progress line --

    #[test]
    fn test_parse_full_progress_line() {
        let line = "Transferred: 1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s";
        let result = parse_progress_line(test_id(), line).unwrap();

        assert_eq!(result.process_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(result.transferred, "1.190 GiB");
        assert_eq!(result.total, "1.190 GiB");
        assert_eq!(result.percent, 100);
        assert_eq!(result.speed, "12.034 MiB/s");
        assert_eq!(result.eta, "0s");
    }

    // -- Progress line with different values --

    #[test]
    fn test_parse_partial_progress_line() {
        let line = "Transferred: 523.789 MiB / 2.000 GiB, 25%, 45.221 MiB/s, ETA 34s";
        let result = parse_progress_line(test_id(), line).unwrap();

        assert_eq!(result.transferred, "523.789 MiB");
        assert_eq!(result.total, "2.000 GiB");
        assert_eq!(result.percent, 25);
        assert_eq!(result.speed, "45.221 MiB/s");
        assert_eq!(result.eta, "34s");
    }

    // -- Progress line without ETA --

    #[test]
    fn test_parse_progress_line_without_eta() {
        let line = "Transferred: 100.000 KiB / 500.000 KiB, 20%, 2.000 MiB/s";
        let result = parse_progress_line(test_id(), line).unwrap();

        assert_eq!(result.transferred, "100.000 KiB");
        assert_eq!(result.percent, 20);
        assert_eq!(result.speed, "2.000 MiB/s");
        assert_eq!(result.eta, ""); // ETA is optional → defaults to empty
    }

    // -- Progress line with integer values --

    #[test]
    fn test_parse_integer_progress() {
        let line = "Transferred: 1 / 1, 100%, 0 B/s, ETA -";
        let result = parse_progress_line(test_id(), line).unwrap();

        assert_eq!(result.transferred, "1");
        assert_eq!(result.total, "1");
        assert_eq!(result.percent, 100);
        assert_eq!(result.speed, "0 B/s");
        assert_eq!(result.eta, "-");
    }

    // -- Non-progress line (passthrough → None) --

    #[test]
    fn test_parse_non_progress_line_returns_none() {
        let lines = [
            "INFO  : file.txt: Copied (new)",
            "2024/01/01 12:00:00 NOTICE: Starting sync...",
            "There was an error: file not found",
            "Some random log output",
            "Transferred:   	", // truncated — missing fields
        ];

        for line in &lines {
            assert!(
                parse_progress_line(test_id(), line).is_none(),
                "expected None for non-progress line: {line:?}"
            );
        }
    }

    // -- Empty line --

    #[test]
    fn test_parse_empty_line_returns_none() {
        assert!(parse_progress_line(test_id(), "").is_none());
        assert!(parse_progress_line(test_id(), "   ").is_none());
    }

    // -- Progress line at 0% --

    #[test]
    fn test_parse_zero_percent_progress() {
        let line = "Transferred: 0.000 B / 1.000 GiB, 0%, 0.000 B/s, ETA -";
        let result = parse_progress_line(test_id(), line).unwrap();

        assert_eq!(result.transferred, "0.000 B");
        assert_eq!(result.total, "1.000 GiB");
        assert_eq!(result.percent, 0);
        assert_eq!(result.speed, "0.000 B/s");
    }

    // -- ETA with alphabetic suffix (e.g. 1h2m3s) --

    #[test]
    fn test_parse_eta_alphabetic() {
        let line = "Transferred: 500.000 MiB / 1.000 GiB, 48%, 10.000 MiB/s, ETA 1m2s";
        let result = parse_progress_line(test_id(), line).unwrap();
        assert_eq!(result.eta, "1m2s");
    }
}

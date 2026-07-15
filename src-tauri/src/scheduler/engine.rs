use chrono::Utc;
use serde::Serialize;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

pub use crate::db::task_repo::Task;
use crate::rclone::events::parse_progress_line;

/// Build a `tokio::process::Command` that never opens a console window on Windows.
fn no_window_cmd(program: impl AsRef<std::ffi::OsStr>) -> tokio::process::Command {
    let cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    let cmd = {
        let mut cmd = cmd;
        cmd.creation_flags(0x0800_0000);
        cmd
    };
    cmd
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub task_id: String,
    pub process_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Execute a scheduled task: spawn rclone, capture progress, wait for exit.
pub async fn execute_task(
    task: &Task,
    rclone_path: &str,
    app: Option<&tauri::AppHandle<tauri::Wry>>,
) -> Result<TaskResult, String> {
    let started_at = Utc::now().to_rfc3339();
    let process_id = Uuid::new_v4();

    // --- Karadelik (Black Hole) handler ---
    // If destination is "(karadelik)", replace with platform-specific null device.
    let dest = if task.dest_provider == "(karadelik)" {
        let null_dev = if cfg!(target_os = "windows") { "NUL" } else { "/dev/null" };
        let msg = format!(
            "WARN: Karadelik hedefi kullaniliyor. Hedef: {}. Veri kurtarilamaz!",
            null_dev
        );
        eprintln!("{}", msg);
        if let Some(a) = app {
            let _ = a.emit("rclone:log", serde_json::json!({
                "process_id": process_id.to_string(),
                "line": &msg,
            }));
        }
        null_dev.to_string()
    } else {
        task.dest_provider.clone()
    };

    // Build rclone args — source/dest are full paths (e.g. "gdrive:/backups" or "C:\Users\me")
    let mut args = vec![task.operation.clone()];
    args.push(task.source_provider.clone());
    args.push(dest);
    for pattern in &task.exclude_patterns {
        args.push("--exclude".to_string());
        args.push(pattern.clone());
    }
    args.push("--progress".to_string());

    let mut child = no_window_cmd(rclone_path)
        .args(&args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone for task '{}': {}", task.name, e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    // Register PID in AppState (task_id → PID) for targeted stop capability
    let pid = child.id().unwrap_or(0);
    if let Some(a) = app {
        let state = a.state::<crate::state::AppState>();
        let mut pids = state.task_pids.lock().await;
        pids.insert(task.id.clone(), pid);
    }

    let mut error_lines = Vec::new();

    // Read stdout, parse progress, and emit events
    let mut stdout_lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = stdout_lines.next_line().await {
        if let Some(payload) = parse_progress_line(process_id, &line) {
            if let Some(app) = app {
                let _ = app.emit("rclone:progress", &payload);
            }
        }
    }

    // Read stderr for errors and emit log events
    let mut stderr_lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = stderr_lines.next_line().await {
        if !line.is_empty() {
            if let Some(app) = app {
                let _ = app.emit(
                    "rclone:log",
                    serde_json::json!({
                        "process_id": process_id.to_string(),
                        "line": &line,
                    }),
                );
            }
            error_lines.push(line);
        }
    }

    // Wait for process exit
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Wait error: {}", e))?;
    let completed_at = Utc::now().to_rfc3339();
    let success = status.success();

    // Remove PID from tracking
    if let Some(a) = app {
        let state = a.state::<crate::state::AppState>();
        let mut pids = state.task_pids.lock().await;
        pids.remove(&task.id);
    }

    Ok(TaskResult {
        task_id: task.id.clone(),
        process_id: process_id.to_string(),
        started_at,
        completed_at: Some(completed_at),
        success,
        error_message: if success {
            None
        } else {
            Some(error_lines.join("\n"))
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::task_repo::Task;

    fn sample_task() -> Task {
        Task {
            id: "test-id".into(),
            name: "Test Task".into(),
            slug: "test-task".into(),
            source_provider: "local".into(),
            source_config: serde_json::Value::Null,
            dest_provider: "local".into(),
            dest_config: serde_json::Value::Null,
            operation: "copy".into(),
            exclude_patterns: vec![],
            cron_expr: "0 * * * * *".into(),
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_execute_task_invalid_path_returns_error() {
        let task = sample_task();
        let result = execute_task(&task, "/nonexistent/rclone", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_task_empty_path_returns_error() {
        let task = sample_task();
        let result = execute_task(&task, "", None).await;
        assert!(result.is_err());
    }
}

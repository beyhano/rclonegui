use chrono::Utc;
use serde::Serialize;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

pub use crate::db::task_repo::Task;
use crate::rclone::events::parse_progress_line;

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
    app: Option<&tauri::AppHandle>,
) -> Result<TaskResult, String> {
    let started_at = Utc::now().to_rfc3339();
    let process_id = Uuid::new_v4();

    // Build rclone args — source/dest are full paths (e.g. "gdrive:/backups" or "C:\Users\me")
    let mut args = vec![task.operation.clone()];
    args.push(task.source_provider.clone());
    args.push(task.dest_provider.clone());
    for pattern in &task.exclude_patterns {
        args.push("--exclude".to_string());
        args.push(pattern.clone());
    }
    args.push("--progress".to_string());

    let mut child = tokio::process::Command::new(rclone_path)
        .args(&args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone for task '{}': {}", task.name, e))?;

    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child.stderr.take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

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
                let _ = app.emit("rclone:log", serde_json::json!({
                    "process_id": process_id.to_string(),
                    "line": &line,
                }));
            }
            error_lines.push(line);
        }
    }

    // Wait for process exit
    let status = child.wait().await.map_err(|e| format!("Wait error: {}", e))?;
    let completed_at = Utc::now().to_rfc3339();
    let success = status.success();

    Ok(TaskResult {
        task_id: task.id.clone(),
        process_id: process_id.to_string(),
        started_at,
        completed_at: Some(completed_at),
        success,
        error_message: if success { None } else { Some(error_lines.join("\n")) },
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

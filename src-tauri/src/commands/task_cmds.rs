/// Tauri #[tauri::command] functions for task CRUD operations.
///
/// # Commands
///
/// | Command | Returns |
/// |---|---|
/// | `task_list` | `Vec<Task>` |
/// | `task_create` | `Task` |
/// | `task_update` | `Task` |
/// | `task_delete` | `()` |
/// | `task_toggle` | `Task` |
/// | `rclone_providers` | `serde_json::Value` |

use std::path::PathBuf;

use chrono::Utc;
use cron::Schedule;
use tauri::State;
use uuid::Uuid;

use crate::db::task_repo::Task;
use crate::state::AppState;

/// Helper: get the configured rclone binary path, or return an error.
fn get_rclone_path(state: &AppState) -> Result<PathBuf, String> {
    let guard = state.rclone_path.lock().map_err(|e| e.to_string())?;
    guard
        .clone()
        .ok_or_else(|| "No rclone binary configured".to_string())
}

/// Generate a URL-safe slug from a task name.
fn generate_slug(name: &str) -> String {
    let slug_base = name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
        .replace(' ', "-");
    format!("{}-{}", slug_base, &Uuid::new_v4().to_string()[..8])
}

/// Validate task input fields.
///
/// Returns `Ok(())` if valid, or `Err(message)` with a user-facing description.
fn validate_task_input(
    name: &str,
    source_provider: &str,
    dest_provider: &str,
    operation: &str,
    cron_expr: &str,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Task name must not be empty".to_string());
    }
    if source_provider.trim().is_empty() {
        return Err("Source provider must not be empty".to_string());
    }
    if dest_provider.trim().is_empty() {
        return Err("Destination provider must not be empty".to_string());
    }
    match operation {
        "copy" | "sync" | "move" | "bisync" => {}
        _ => {
            return Err(format!(
                "Invalid operation '{}'. Must be one of: copy, sync, move, bisync",
                operation
            ));
        }
    }
    cron_expr
        .parse::<Schedule>()
        .map_err(|e| format!("Invalid cron expression: {}", e))?;
    Ok(())
}

/// List all tasks ordered by `created_at` descending.
#[tauri::command]
pub async fn task_list(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let repo = state.task_repo.lock().await;
    repo.list().map_err(|e| e.to_string())
}

/// Create a new task.
///
/// Validates input, generates a unique slug, persists to the database,
/// and notifies the scheduler so it can watch the new task.
#[tauri::command]
pub async fn task_create(
    state: State<'_, AppState>,
    name: String,
    source_provider: String,
    source_config: serde_json::Value,
    dest_provider: String,
    dest_config: serde_json::Value,
    operation: String,
    exclude_patterns: Vec<String>,
    cron_expr: String,
) -> Result<Task, String> {
    validate_task_input(
        &name,
        &source_provider,
        &dest_provider,
        &operation,
        &cron_expr,
    )?;

    let now = Utc::now().to_rfc3339();
    let slug = generate_slug(&name);
    let id = Uuid::new_v4().to_string();

    let task = Task {
        id,
        name,
        slug,
        source_provider,
        source_config,
        dest_provider,
        dest_config,
        operation,
        exclude_patterns,
        cron_expr,
        enabled: false,
        created_at: now.clone(),
        updated_at: now,
    };

    // DB write — synchronous, drop guard before scheduler await.
    {
        let repo = state.task_repo.lock().await;
        repo.create(&task).map_err(|e| e.to_string())?;
    }

    // Notify the scheduler.
    let sched = state.scheduler.lock().await;
    if let Some(ref scheduler) = *sched {
        scheduler.add_task(&task).await;
    }

    Ok(task)
}

/// Update an existing task.
///
/// Preserves the original `created_at` timestamp. Does NOT update the
/// `enabled` flag — use `task_toggle` for that. Notifies the scheduler
/// so its loop picks up the new configuration.
#[tauri::command]
pub async fn task_update(
    state: State<'_, AppState>,
    id: String,
    name: String,
    slug: String,
    source_provider: String,
    source_config: serde_json::Value,
    dest_provider: String,
    dest_config: serde_json::Value,
    operation: String,
    exclude_patterns: Vec<String>,
    cron_expr: String,
) -> Result<Task, String> {
    validate_task_input(
        &name,
        &source_provider,
        &dest_provider,
        &operation,
        &cron_expr,
    )?;

    // DB read + update — synchronous, drop guard before await.
    let task = {
        let repo = state.task_repo.lock().await;

        let existing = repo
            .get_by_id(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Task not found: {}", id))?;

        let now = Utc::now().to_rfc3339();

        let task = Task {
            id,
            name,
            slug,
            source_provider,
            source_config,
            dest_provider,
            dest_config,
            operation,
            exclude_patterns,
            cron_expr,
            enabled: existing.enabled,
            created_at: existing.created_at,
            updated_at: now,
        };

        repo.update(&task).map_err(|e| e.to_string())?;
        task
    };

    // Notify the scheduler.
    let sched = state.scheduler.lock().await;
    if let Some(ref scheduler) = *sched {
        scheduler.update_task(&task).await;
    }

    Ok(task)
}

/// Delete a task by its ID. Removes it from the scheduler if running.
#[tauri::command]
pub async fn task_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    {
        let repo = state.task_repo.lock().await;
        repo.delete(&id).map_err(|e| e.to_string())?;
    }

    // Notify the scheduler.
    let sched = state.scheduler.lock().await;
    if let Some(ref scheduler) = *sched {
        scheduler.remove_task(&id).await;
    }

    Ok(())
}

/// Toggle a task's `enabled` flag.
///
/// If enabled, the scheduler starts watching the task on its cron schedule.
/// If disabled, the scheduler removes the task loop.
#[tauri::command]
pub async fn task_toggle(state: State<'_, AppState>, id: String) -> Result<Task, String> {
    // DB read + write — synchronous, drop guard before await.
    let (task, is_now_enabled) = {
        let repo = state.task_repo.lock().await;

        let mut task = repo
            .get_by_id(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Task not found: {}", id))?;

        task.enabled = !task.enabled;
        task.updated_at = Utc::now().to_rfc3339();
        let is_now_enabled = task.enabled;

        repo.update(&task).map_err(|e| e.to_string())?;
        (task, is_now_enabled)
    };

    // Notify the scheduler.
    let sched = state.scheduler.lock().await;
    if let Some(ref scheduler) = *sched {
        if is_now_enabled {
            scheduler.add_task(&task).await;
        } else {
            scheduler.remove_task(&id).await;
        }
    }

    Ok(task)
}

/// Run a task immediately, bypassing its schedule.
#[tauri::command]
pub async fn task_run_now(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Get the task
    let task_id = id.clone();
    let task = {
        let repo = state.task_repo.lock().await;
        repo.get_by_id(&task_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Task not found".to_string())?
    };

    // Execute via scheduler if available
    let sched = state.scheduler.lock().await;
    if let Some(ref scheduler) = *sched {
        scheduler.run_now(&task).await;
        Ok(())
    } else {
        Err("Scheduler not initialized".to_string())
    }
}

/// Fetch available rclone providers by running `rclone config providers`.
///
/// Returns the raw JSON output from rclone, parsed as a `serde_json::Value`.
#[tauri::command]
pub async fn rclone_providers(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let path = get_rclone_path(&state)?;

    let output = tokio::process::Command::new(&path)
        .arg("config")
        .arg("providers")
        .output()
        .await
        .map_err(|e| format!("Failed to execute rclone config providers: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone config providers failed: {}", stderr));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("Non-UTF-8 output: {}", e))?;

    serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse rclone providers JSON: {}", e))
}

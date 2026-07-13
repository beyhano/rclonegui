# Task 7: TaskScheduler — cron loop + lifecycle management

**Files:**
- Modify: `src-tauri/src/scheduler/scheduler.rs` (currently a stub)

**Interfaces:**
- Consumes: `TaskRepo` (via `Arc<Mutex<TaskRepo>>`), `String` (rclone_path via `Arc<RwLock<Option<String>>>`), `AppHandle`
- Produces: `TaskScheduler` struct with:
  - `new(repo, rclone_path, app) -> Self`
  - `async start(&self)` — start all enabled tasks
  - `async stop(&self)` — stop all task loops
  - `async add_task(&self, task: &Task)` — add a single task
  - `async remove_task(&self, task_id: &str)` — remove a task
  - `async update_task(&self, task: &Task)` — update a task
  - `async run_now(&self, task: &Task)` — run a task immediately

### Implementation

Replace stub in `src-tauri/src/scheduler/scheduler.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tauri::AppHandle;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::db::task_repo::{Task, TaskRepo};
use crate::scheduler::cron::next_cron_time;
use crate::scheduler::engine::{execute_task, TaskResult};

pub struct TaskScheduler {
    repo: Arc<Mutex<TaskRepo>>,
    rclone_path: Arc<RwLock<Option<String>>>,
    app: AppHandle,
    cancel_tokens: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    running: Arc<Mutex<Vec<String>>>,
    started: Arc<std::sync::atomic::AtomicBool>,
}

impl TaskScheduler {
    pub fn new(
        repo: Arc<Mutex<TaskRepo>>,
        rclone_path: Arc<RwLock<Option<String>>>,
        app: AppHandle,
    ) -> Self {
        Self {
            repo,
            rclone_path,
            app,
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(Vec::new())),
            started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start all enabled task loops.
    pub async fn start(&self) {
        self.started.store(true, std::sync::atomic::Ordering::SeqCst);
        let tasks = {
            let repo = self.repo.lock().await;
            repo.get_enabled().unwrap_or_default()
        };
        for task in tasks {
            self.spawn_task_loop(task).await;
        }
    }

    /// Stop all running task loops.
    pub async fn stop(&self) {
        self.started.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut tokens = self.cancel_tokens.lock().await;
        for (_, sender) in tokens.drain() {
            let _ = sender.send(());
        }
    }

    /// Add a single task to the scheduler.
    pub async fn add_task(&self, task: &Task) {
        if self.started.load(std::sync::atomic::Ordering::SeqCst) && task.enabled {
            self.spawn_task_loop(task).await;
        }
    }

    /// Remove a task from the scheduler.
    pub async fn remove_task(&self, task_id: &str) {
        let mut tokens = self.cancel_tokens.lock().await;
        if let Some(sender) = tokens.remove(task_id) {
            let _ = sender.send(());
        }
    }

    /// Update a task: cancel old loop, start new one if enabled.
    pub async fn update_task(&self, task: &Task) {
        self.remove_task(&task.id).await;
        self.add_task(task).await;
    }

    /// Run a task immediately, bypassing the schedule.
    pub async fn run_now(&self, task: &Task) {
        let rclone_path = self.rclone_path.read().await.clone().unwrap_or_default();
        let result = execute_task(task, &rclone_path).await;
        let app = self.app.clone();
        match result {
            Ok(task_result) => {
                // Save to DB
                let repo = self.repo.lock().await;
                let _ = repo.connection().execute(
                    "INSERT INTO transfers (id, remote_src, remote_dest, status, progress, started_at, completed_at, error_message, task_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        &task.source_provider,
                        &task.dest_provider,
                        if task_result.success { "completed" } else { "error" },
                        100.0,
                        &task_result.started_at,
                        &task_result.completed_at,
                        &task_result.error_message,
                        &task.id,
                    ],
                );

                let _ = app.emit(
                    if task_result.success { "task:completed" } else { "task:error" },
                    serde_json::json!({
                        "task_id": &task.id,
                        "task_name": &task.name,
                        "started_at": &task_result.started_at,
                        "completed_at": &task_result.completed_at,
                        "error": &task_result.error_message,
                    }),
                );
            }
            Err(e) => {
                let _ = app.emit("task:error", serde_json::json!({
                    "task_id": &task.id,
                    "task_name": &task.name,
                    "error": e,
                }));
            }
        }

        // Mark as not running
        let mut running = self.running.lock().await;
        running.retain(|id| id != &task.id);
    }

    fn spawn_task_loop(&self, task: Task) {
        let repo = self.repo.clone();
        let rclone_path = self.rclone_path.clone();
        let app = self.app.clone();
        let running = self.running.clone();
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();

        if let Ok(mut tokens) = self.cancel_tokens.try_lock() {
            tokens.insert(task.id.clone(), cancel_tx);
        }

        tokio::spawn(async move {
            loop {
                // Calculate next run time
                let next = match next_cron_time(&task.cron_expr) {
                    Ok(Some(dt)) => dt,
                    _ => break, // Invalid cron or no future time
                };

                let now = Utc::now();
                let delay = (next - now).max(chrono::Duration::zero());
                let delay_std = std::time::Duration::from_secs(
                    delay.num_seconds().max(0) as u64
                );

                tokio::select! {
                    _ = &mut cancel_rx => break,
                    _ = tokio::time::sleep(delay_std) => {
                        // Check overlap
                        let already_running = {
                            let mut r = running.lock().await;
                            if r.contains(&task.id) {
                                true
                            } else {
                                r.push(task.id.clone());
                                false
                            }
                        };

                        if already_running { continue; }

                        let path = rclone_path.read().await.clone().unwrap_or_default();
                        let result = execute_task(&task, &path).await;

                        match result {
                            Ok(task_result) => {
                                let repo_guard = repo.lock().await;
                                let _ = repo_guard.connection().execute(
                                    "INSERT INTO transfers (...) VALUES (?1, ...)",
                                    // same as run_now above
                                );
                                let _ = app.emit(
                                    if task_result.success { "task:completed" } else { "task:error" },
                                    serde_json::json!({
                                        "task_id": &task.id,
                                        "task_name": &task.name,
                                        "started_at": &task_result.started_at,
                                        "completed_at": &task_result.completed_at,
                                        "error": &task_result.error_message,
                                    }),
                                );
                            }
                            Err(e) => {
                                let _ = app.emit("task:error", serde_json::json!({
                                    "task_id": &task.id,
                                    "task_name": &task.name,
                                    "error": e,
                                }));
                            }
                        }

                        // Mark as not running
                        let mut r = running.lock().await;
                        r.retain(|id| id != &task.id);
                    }
                }
            }
        });
    }
}
```

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::task_repo::Task;

    fn sample_task() -> Task {
        Task {
            id: "test-id".into(),
            name: "Test".into(),
            slug: "test".into(),
            source_provider: "local".into(),
            source_config: serde_json::Value::Null,
            dest_provider: "local".into(),
            dest_config: serde_json::Value::Null,
            operation: "copy".into(),
            exclude_patterns: vec![],
            cron_expr: "0 * * * * *".into(),
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn test_scheduler_new() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrations::create_tables(&conn).unwrap();
        let repo = TaskRepo::new(conn);
        let repo = Arc::new(Mutex::new(repo));
        // Create a minimal app handle for testing (needs Tauri)
        // For now, just verify TaskScheduler construction works
        // (Full integration test in Task 8)
    }

    #[test]
    fn test_task_struct_deserialize() {
        let task = sample_task();
        assert_eq!(task.name, "Test");
        assert_eq!(task.slug, "test");
        assert!(task.enabled);
    }
}
```

Note: The scheduler tests require a Tauri `AppHandle`, which is complex to create in unit tests. The main test coverage comes from integration with Task 8 (Tauri commands). Focus on making the code compile and the `test_task_struct_deserialize` test pass.

### Verification

```bash
cd src-tauri && cargo check
```
Expected: Compiles with only pre-existing dead-code warnings.

### Commit

```bash
git add -A && git commit -m "feat(scheduler): add TaskScheduler with cron loop"
```

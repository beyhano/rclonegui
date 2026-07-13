use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;
use tokio::process::Child;
use uuid::Uuid;

use crate::scheduler::scheduler::TaskScheduler;

/// A handle to a running rclone process.
#[derive(Debug)]
pub struct ProcessHandle {
    pub child: Child,
    pub pid: u32,
    pub command: String,
    pub started_at: DateTime<Utc>,
}

impl ProcessHandle {
    pub fn new(child: Child, command: String) -> Self {
        let pid = child.id().unwrap_or(0);
        Self {
            child,
            pid,
            command,
            started_at: Utc::now(),
        }
    }
}

/// Mount-specific metadata for an active mount process.
#[derive(Debug, Clone, Serialize)]
pub struct MountInfo {
    pub id: String,
    pub remote: String,
    pub mount_point: String,
    pub status: String,
}

/// Shared application state managed by Tauri.
pub struct AppState {
    pub processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
    pub rclone_path: Arc<Mutex<Option<PathBuf>>>,
    pub db: Arc<Mutex<Connection>>,
    pub mounts: Arc<Mutex<HashMap<Uuid, MountInfo>>>,
    pub task_repo: Arc<tokio::sync::Mutex<crate::db::task_repo::TaskRepo>>,
    /// Optional TaskScheduler for running scheduled transfer tasks on cron.
    pub scheduler: Arc<tokio::sync::Mutex<Option<TaskScheduler>>>,
}

impl AppState {
    pub fn new(
        db: Connection,
        task_repo: Arc<tokio::sync::Mutex<crate::db::task_repo::TaskRepo>>,
        scheduler: Option<TaskScheduler>,
    ) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            rclone_path: Arc::new(Mutex::new(None)),
            db: Arc::new(Mutex::new(db)),
            mounts: Arc::new(Mutex::new(HashMap::new())),
            task_repo,
            scheduler: Arc::new(tokio::sync::Mutex::new(scheduler)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::task_repo::TaskRepo;

    #[test]
    fn test_app_state_creation() {
        let db = Connection::open_in_memory().unwrap();
        let repo = Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
            Connection::open_in_memory().unwrap(),
        )));
        let state = AppState::new(db, repo, None);

        assert!(state.processes.lock().unwrap().is_empty());
        assert!(state.rclone_path.lock().unwrap().is_none());
        assert!(state.mounts.lock().unwrap().is_empty());
    }

    #[test]
    fn test_process_handle_creation() {
        let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
        let child = rt.block_on(async {
            #[cfg(not(target_os = "windows"))]
            {
                tokio::process::Command::new("echo")
                    .arg("test")
                    .spawn()
            }
            #[cfg(target_os = "windows")]
            {
                tokio::process::Command::new("cmd.exe")
                    .args(["/c", "echo", "test"])
                    .spawn()
            }
        })
        .expect("failed to spawn echo");
        let pid = child.id().expect("child process has no pid");
        let handle = ProcessHandle::new(child, "echo test".to_string());

        assert!(handle.pid > 0);
        assert_eq!(handle.pid, pid);
        assert_eq!(handle.command, "echo test");
    }
}

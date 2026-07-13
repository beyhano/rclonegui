/// Rclone process lifecycle management.
///
/// Manages async child processes via `tokio::process::Command` with
/// `kill_on_drop(true)`. Process state lives in `Arc<Mutex<HashMap<Uuid, ProcessHandle>>>`
/// inside Tauri's `AppState`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::state::ProcessHandle;

/// Manages the lifecycle of rclone child processes.
///
/// All process state is held behind `Arc<Mutex<...>>` so it can be safely shared
/// with Tauri command handlers running on the async runtime.
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
    rclone_path: Arc<Mutex<Option<PathBuf>>>,
}

impl ProcessManager {
    /// Create a new `ProcessManager` bound to the given shared state.
    pub fn new(
        processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
        rclone_path: Arc<Mutex<Option<PathBuf>>>,
    ) -> Self {
        Self {
            processes,
            rclone_path,
        }
    }

    /// Spawn an rclone process with the given arguments.
    ///
    /// Returns a `Uuid` that identifies the process in the process map.
    ///
    /// # Errors
    ///
    /// - `"No rclone binary configured"` if `rclone_path` is `None`.
    /// - Propagates IO errors from `tokio::process::Command::spawn`.
    pub async fn spawn(&self, args: Vec<String>) -> Result<Uuid, String> {
        let path = {
            let guard = self.rclone_path.lock().map_err(|e| e.to_string())?;
            guard
                .clone()
                .ok_or_else(|| "No rclone binary configured".to_string())?
        };

        let child = tokio::process::Command::new(&path)
            .args(&args)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn rclone: {}", e))?;

        let id = Uuid::new_v4();
        let command_str = format!("{} {}", path.display(), args.join(" "));
        let handle = ProcessHandle::new(child, command_str);

        let mut guard = self.processes.lock().map_err(|e| e.to_string())?;
        guard.insert(id, handle);

        Ok(id)
    }

    /// Stop a tracked process by its UUID.
    ///
    /// Removes the process from the map (which drops the `Child` handle).
    /// `kill_on_drop(true)` ensures the OS process is terminated.
    ///
    /// # Errors
    ///
    /// - `"Process not found: {id}"` if no process with the given id exists.
    /// - Lock-poisoning errors from the mutex.
    pub async fn stop(&self, id: Uuid) -> Result<(), String> {
        let mut guard = self.processes.lock().map_err(|e| e.to_string())?;
        let mut handle = guard
            .remove(&id)
            .ok_or_else(|| format!("Process not found: {}", id))?;
        drop(guard);

        // Send kill signal to the child process
        let _ = handle.child.start_kill();
        // handle drops here; kill_on_drop(true) ensures the process is terminated

        Ok(())
    }

    /// Stop all tracked processes and clear the process map.
    ///
    /// Dropping every `ProcessHandle` triggers `kill_on_drop(true)` for each child.
    /// Call this during application exit / cleanup.
    pub fn cleanup_all(&self) -> Result<(), String> {
        let mut guard = self.processes.lock().map_err(|e| e.to_string())?;
        guard.clear();
        Ok(())
    }

    /// Return a copy of the current rclone binary path, if configured.
    pub fn get_rclone_path(&self) -> Result<Option<PathBuf>, String> {
        let guard = self.rclone_path.lock().map_err(|e| e.to_string())?;
        Ok(guard.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a ProcessManager with test state.
    fn make_manager() -> ProcessManager {
        let processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let rclone_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        ProcessManager::new(processes, rclone_path)
    }

    #[test]
    fn test_new_manager_has_empty_processes() {
        let pm = make_manager();
        let guard = pm.processes.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_new_manager_no_rclone_path() {
        let pm = make_manager();
        assert!(pm.get_rclone_path().unwrap().is_none());
    }

    #[test]
    fn test_spawn_fails_without_binary() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pm = make_manager();
        let result = rt.block_on(pm.spawn(vec!["version".to_string()]));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No rclone binary configured"));
    }

    #[test]
    fn test_stop_nonexistent_process() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pm = make_manager();
        let id = Uuid::new_v4();
        let result = rt.block_on(pm.stop(id));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Process not found"));
    }

    #[test]
    fn test_cleanup_all_empty() {
        let pm = make_manager();
        assert!(pm.cleanup_all().is_ok());
        let guard = pm.processes.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_cleanup_all_with_entries() {
        // Insert a simulated entry directly (no real spawn needed for cleanup test)
        let processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let rclone_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));

        // Start a real child so we have a valid ProcessHandle
        let rt = tokio::runtime::Runtime::new().unwrap();
        let child = rt
            .block_on(async {
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
        let handle = ProcessHandle::new(child, "echo test".to_string());
        let id = Uuid::new_v4();

        processes.lock().unwrap().insert(id, handle);

        let pm = ProcessManager::new(processes, rclone_path);
        assert!(pm.cleanup_all().is_ok());
        let guard = pm.processes.lock().unwrap();
        assert!(guard.is_empty());
    }

    // ------------------------------------------------------------------
    // Phase 6 — Task 6.1: Integration test via echo
    //   Spawn echo via ProcessManager, assert PID tracked,
    //   stop, assert cleanup
    // ------------------------------------------------------------------

    #[test]
    fn test_spawn_echo_track_pid_stop_cleanup() {
        let processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>> =
            Arc::new(Mutex::new(HashMap::new()));
        #[cfg(not(target_os = "windows"))]
        let rclone_path: Arc<Mutex<Option<PathBuf>>> =
            Arc::new(Mutex::new(Some(PathBuf::from("echo"))));
        #[cfg(target_os = "windows")]
        let rclone_path: Arc<Mutex<Option<PathBuf>>> =
            Arc::new(Mutex::new(Some(PathBuf::from("cmd.exe"))));
        let pm = ProcessManager::new(processes.clone(), rclone_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // Spawn echo via ProcessManager — the binary is "echo" with args
        #[cfg(not(target_os = "windows"))]
        let args = vec!["hello".to_string()];
        #[cfg(target_os = "windows")]
        let args = vec!["/c".to_string(), "echo".to_string(), "hello".to_string()];
        let id = rt
            .block_on(pm.spawn(args))
            .expect("failed to spawn echo via ProcessManager");

        // Assert PID is tracked in the HashMap
        {
            let guard = processes.lock().unwrap();
            assert!(guard.contains_key(&id), "process should be in map");
            let handle = &guard[&id];
            assert!(handle.pid > 0, "PID should be > 0, got {}", handle.pid);
            #[cfg(not(target_os = "windows"))]
            assert_eq!(handle.command, "echo hello");
            #[cfg(target_os = "windows")]
            assert_eq!(handle.command, "cmd.exe /c echo hello");
        }

        // Stop the process
        rt.block_on(pm.stop(id))
            .expect("failed to stop echo process");

        // Assert the process is removed from the HashMap
        {
            let guard = processes.lock().unwrap();
            assert!(
                !guard.contains_key(&id),
                "process should be removed after stop"
            );
            assert!(guard.is_empty(), "process map should be empty");
        }
    }
}

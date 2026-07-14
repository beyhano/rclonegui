/// Rclone process lifecycle management.
///
/// Manages async child processes via `tokio::process::Command` with
/// `kill_on_drop(true)`. Process state lives in `Arc<Mutex<HashMap<Uuid, ProcessHandle>>>`
/// inside Tauri's `AppState`.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::state::ProcessHandle;

/// Manages the lifecycle of rclone child processes.
///
/// All process state is held behind `Arc<Mutex<...>>` so it can be safely shared
/// with Tauri command handlers running on the async runtime.
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
}

impl ProcessManager {
    /// Create a new `ProcessManager` bound to the given shared state.
    pub fn new(processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>) -> Self {
        Self { processes }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a ProcessManager with test state.
    fn make_manager() -> ProcessManager {
        let processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>> =
            Arc::new(Mutex::new(HashMap::new()));
        ProcessManager::new(processes)
    }

    #[test]
    fn test_new_manager_has_empty_processes() {
        let pm = make_manager();
        let guard = pm.processes.lock().unwrap();
        assert!(guard.is_empty());
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

        // Start a real child so we have a valid ProcessHandle
        let rt = tokio::runtime::Runtime::new().unwrap();
        let child = rt
            .block_on(async {
                #[cfg(not(target_os = "windows"))]
                {
                    tokio::process::Command::new("echo").arg("test").spawn()
                }
                #[cfg(target_os = "windows")]
                {
                    tokio::process::Command::new("cmd.exe")
                        .args(["/c", "echo", "test"])
                        .spawn()
                }
            })
            .expect("failed to spawn echo");
        let handle = ProcessHandle::new(child);
        let id = Uuid::new_v4();

        processes.lock().unwrap().insert(id, handle);

        let pm = ProcessManager::new(processes);
        assert!(pm.cleanup_all().is_ok());
        let guard = pm.processes.lock().unwrap();
        assert!(guard.is_empty());
    }
}

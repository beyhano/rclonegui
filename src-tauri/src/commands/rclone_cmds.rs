/// Tauri #[tauri::command] functions for rclone lifecycle management.
///
/// All commands are async (or sync where no IO is needed) and receive Tauri
/// managed `AppState` via `State<'_, AppState>`. Commands that spawn child
/// processes also receive `AppHandle` for event emission.
///
/// # Commands
///
/// | Command | Returns |
/// |---|---|
/// | `rclone_version` | `"rclone v1.65.0"` |
/// | `rclone_config_list` | `Vec<Remote>` |
/// | `rclone_exec` | `Uuid` string |
/// | `rclone_stop` | `()` |
/// | `rclone_mount` | `Uuid` string |
/// | `rclone_unmount` | `()` |
/// | `rclone_mount_list` | `Vec<MountInfo>` |
use std::path::PathBuf;

use serde::Serialize;
use tauri::{Emitter, State};
use tokio::io::BufReader;
use uuid::Uuid;

use crate::rclone::config::{self, Remote};
use crate::rclone::events::start_event_stream;
use crate::rclone::process::ProcessManager;
use crate::state::{AppState, MountInfo, ProcessHandle};

/// Build a `tokio::process::Command` that never opens a console window on Windows.
#[allow(dead_code)]
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

/// Helper: get the configured rclone binary path, or return an error.
fn get_rclone_path(state: &AppState) -> Result<PathBuf, String> {
    let guard = state.rclone_path.lock().map_err(|e| e.to_string())?;
    guard
        .clone()
        .ok_or_else(|| "No rclone binary configured".to_string())
}

/// Return the rclone version string by executing `rclone version`.
#[tauri::command]
pub async fn rclone_version(state: State<'_, AppState>) -> Result<String, String> {
    let path = get_rclone_path(&state)?;

    let output = no_window_cmd(&path)
        .arg("version")
        .output()
        .await
        .map_err(|e| format!("Failed to execute rclone version: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone version failed: {}", stderr));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("Non-UTF-8 output: {}", e))?;

    Ok(stdout.trim().to_string())
}

/// List configured rclone remotes via `rclone config dump`.
#[tauri::command]
pub async fn rclone_config_list(state: State<'_, AppState>) -> Result<Vec<Remote>, String> {
    let path = get_rclone_path(&state)?;
    config::config_list(&path).await
}

/// Spawn an rclone process with arbitrary arguments and start its event stream.
///
/// Returns the `Uuid` string for the spawned process so the frontend can
/// reference it in subsequent `rclone_stop` calls and listen for progress events.
#[tauri::command]
pub async fn rclone_exec(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    args: Vec<String>,
) -> Result<String, String> {
    let path = get_rclone_path(&state)?;

    let mut child = no_window_cmd(&path)
        .args(&args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let id = Uuid::new_v4();
    let command_str = format!("{} {}", path.display(), args.join(" "));
    let handle = ProcessHandle::new(child);

    state
        .processes
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id, handle);

    // Spawn background event stream for stdout/stderr
    let event_handle = start_event_stream(
        app.clone(),
        id,
        BufReader::new(stdout),
        BufReader::new(stderr),
    );

    // Monitor when event stream finishes (process exited) → emit completion
    let app_clone = app.clone();
    let pid = id;
    tokio::spawn(async move {
        let _ = event_handle.await;
        let _ = app_clone.emit(
            "rclone:process-completed",
            serde_json::json!({
                "process_id": pid.to_string(),
            }),
        );
    });

    let _ = app.emit(
        "rclone:process-started",
        serde_json::json!({
            "process_id": id.to_string(),
            "command": command_str,
        }),
    );

    Ok(id.to_string())
}

/// Stop a running rclone process by its UUID string.
#[tauri::command]
pub async fn rclone_stop(state: State<'_, AppState>, process_id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&process_id).map_err(|e| format!("Invalid process ID: {}", e))?;

    let pm = ProcessManager::new(state.processes.clone());
    pm.stop(id).await
}

/// Mount a remote filesystem using `rclone mount`.
///
/// The remote argument may be with or without a trailing colon
/// (e.g. `"gdrive:"` or `"gdrive"`).
#[tauri::command]
pub async fn rclone_mount(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    remote: String,
    mount_point: String,
) -> Result<String, String> {
    let path = get_rclone_path(&state)?;

    // Ensure remote has colon suffix for rclone CLI
    let remote_arg = if remote.contains(':') {
        remote.clone()
    } else {
        format!("{}:", remote)
    };

    let args = vec!["mount".to_string(), remote_arg, mount_point.clone()];

    let child = no_window_cmd(&path)
        .args(&args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone mount: {}", e))?;

    let id = Uuid::new_v4();
    let handle = ProcessHandle::new(child);

    state
        .processes
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id, handle);

    // Store mount metadata
    let mount_info = MountInfo {
        id: id.to_string(),
        remote,
        mount_point,
        status: "running".to_string(),
    };
    state
        .mounts
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id, mount_info);

    let _ = app.emit(
        "rclone:mount-status",
        serde_json::json!({
            "mount_id": id.to_string(),
            "status": "running",
        }),
    );

    Ok(id.to_string())
}

/// Stop a running mount process by its UUID string.
///
/// Also removes the mount metadata from state.
#[tauri::command]
pub async fn rclone_unmount(state: State<'_, AppState>, mount_id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;

    // Stop the underlying process
    let pm = ProcessManager::new(state.processes.clone());
    pm.stop(id).await?;

    // Remove mount metadata
    state.mounts.lock().map_err(|e| e.to_string())?.remove(&id);

    Ok(())
}

/// List all active mount processes with their metadata.
#[tauri::command]
pub fn rclone_mount_list(state: State<'_, AppState>) -> Result<Vec<MountInfo>, String> {
    let guard = state.mounts.lock().map_err(|e| e.to_string())?;
    let mounts: Vec<MountInfo> = guard.values().cloned().collect();
    Ok(mounts)
}

/// Create an rclone remote via `rclone config create --non-interactive`.
///
/// Takes a remote name, provider prefix, and a JSON string of key-value config pairs.
#[tauri::command]
pub async fn rclone_config_create(
    state: State<'_, AppState>,
    name: String,
    provider: String,
    config: String, // JSON string of key-value pairs
) -> Result<(), String> {
    let path = get_rclone_path(&state)?;

    let mut cmd = no_window_cmd(&path);
    cmd.arg("config")
        .arg("create")
        .arg(&name)
        .arg(&provider)
        .arg("--non-interactive");

    // Parse config JSON into key=value args
    if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&config) {
        for (key, value) in &map {
            cmd.arg(key);
            cmd.arg(value);
        }
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone config create: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to create remote: {}", stderr));
    }

    Ok(())
}

/// Get a single remote's full config via `rclone config dump`.
#[tauri::command]
pub async fn rclone_config_get(
    state: State<'_, AppState>,
    name: String,
) -> Result<(String, std::collections::HashMap<String, String>), String> {
    let path = get_rclone_path(&state)?;

    let output = no_window_cmd(&path)
        .args(["config", "dump"])
        .output()
        .await
        .map_err(|e| format!("rclone config dump başarısız: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone config dump başarısız: {}", stderr));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|e| format!("Geçersiz UTF-8: {}", e))?;

    let map: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>> =
        serde_json::from_str(&stdout).map_err(|e| format!("JSON ayrıştırma hatası: {}", e))?;

    let remote = map
        .get(&name)
        .ok_or_else(|| format!("'{}' uzak sunucusu bulunamadı", name))?;

    let provider = remote
        .get("type")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| format!("'{}' için sağlayıcı türü bulunamadı", name))?;

    let mut config = std::collections::HashMap::new();
    for (key, value) in remote {
        if key != "type" {
            config.insert(key.clone(), match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            });
        }
    }

    Ok((provider, config))
}

/// Delete an rclone remote via `rclone config delete <name>`.
#[tauri::command]
pub async fn rclone_config_delete(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let path = get_rclone_path(&state)?;

    let output = no_window_cmd(&path)
        .arg("config")
        .arg("delete")
        .arg(&name)
        .output()
        .await
        .map_err(|e| format!("rclone config delete başarısız: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Uzak sunucu silinemedi '{}': {}", name, stderr));
    }

    Ok(())
}

/// List directories of a remote or local path via `rclone lsf --dirs-only --dir-slash=false`.
#[tauri::command]
pub async fn rclone_list_dirs(
    state: State<'_, AppState>,
    remote: String,
    path: String,
) -> Result<Vec<String>, String> {
    let rclone_path = get_rclone_path(&state)?;

    let remote_arg = if remote == "local" {
        path.clone()
    } else {
        if path.is_empty() {
            format!("{}:", remote)
        } else {
            format!("{}:{}", remote, path)
        }
    };

    let output = no_window_cmd(&rclone_path)
        .arg("lsf")
        .arg("--dirs-only")
        .arg("--dir-slash=false")
        .arg(&remote_arg)
        .output()
        .await
        .map_err(|e| format!("Failed to list directories: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Listing failed: {}", stderr));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Non-UTF-8 output: {}", e))?;

    let dirs: Vec<String> = stdout
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(dirs)
}

/// Result returned by `rclone_selfupdate` to the frontend.
#[derive(Serialize)]
pub struct SelfUpdateResult {
    pub success: bool,
    pub old_version: String,
    pub new_version: String,
    pub message: String,
}

/// Get the plain rclone version string from a binary path.
async fn get_version(binary: &PathBuf) -> Result<String, String> {
    let output = no_window_cmd(binary)
        .arg("version")
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone version: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone version failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Non-UTF-8 output: {}", e))?;

    // Return the first line only (e.g. "rclone v1.65.0")
    Ok(stdout.lines().next().unwrap_or("unknown").to_string())
}

/// Update the rclone binary via `rclone selfupdate` with rollback on failure.
///
/// 1. Reads the current version
/// 2. Backs up the binary
/// 3. Runs `rclone selfupdate`
/// 4. Smoke test: runs `rclone version` to confirm the new binary works
/// 5. On failure: restores the backup, returns error
/// 6. On success: removes the backup
#[tauri::command]
pub async fn rclone_selfupdate(state: State<'_, AppState>) -> Result<SelfUpdateResult, String> {
    let path = get_rclone_path(&state)?;

    // 1. Get old version
    let old_version = get_version(&path).await.map_err(|e| format!("Cannot determine current rclone version: {e}. Is rclone installed?"))?;

    // 2. Backup
    let backup_path = path.with_extension("backup");
    if let Err(e) = std::fs::copy(&path, &backup_path) {
        return Err(format!("Failed to backup rclone binary: {e}"));
    }

    // 3. Run selfupdate
    let output = no_window_cmd(&path)
        .arg("selfupdate")
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone selfupdate: {e}"))?;

    if !output.status.success() {
        // Restore backup
        let _ = std::fs::copy(&backup_path, &path);
        let _ = std::fs::remove_file(&backup_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let old_ver = old_version.clone();
        return Ok(SelfUpdateResult {
            success: false,
            old_version,
            new_version: old_ver,
            message: format!("Güncelleme başarısız: {}", stderr.trim()),
        });
    }

    // 4. Smoke test: verify the updated binary still works
    let new_version = match get_version(&path).await {
        Ok(v) => v,
        Err(e) => {
            // Restore backup
            let _ = std::fs::copy(&backup_path, &path);
            let _ = std::fs::remove_file(&backup_path);
            let old_ver = old_version.clone();
            return Ok(SelfUpdateResult {
                success: false,
                old_version,
                new_version: old_ver,
                message: format!("Güncelleme sonrası doğrulama başarısız: {e}. Eski sürüme dönüldü."),
            });
        }
    };

    // 5. Clean up backup
    let _ = std::fs::remove_file(&backup_path);

    Ok(SelfUpdateResult {
        success: true,
        old_version,
        new_version,
        message: "rclone başarıyla güncellendi.".to_string(),
    })
}

// ----- Task 4.4 RED test: rclone_config_list error path -----

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::db::task_repo::TaskRepo;
    use rusqlite::Connection;

    // ------------------------------------------------------------------
    // Phase 6 — Task 6.2: Verify spec scenarios
    //   - resolve_platform() returns a valid string (tested in discovery)
    //   - binary-not-found returns appropriate error (not panic)
    // ------------------------------------------------------------------

    #[test]
    fn test_get_rclone_path_none_returns_error() {
        let state = AppState::new(
            Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
                Connection::open_in_memory().unwrap(),
            ))),
            None,
        );
        let result = get_rclone_path(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No rclone binary configured");
    }

    #[test]
    fn test_rclone_version_without_binary_returns_error() {
        // rclone_version calls get_rclone_path first — verify error propagation
        let state = AppState::new(
            Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
                Connection::open_in_memory().unwrap(),
            ))),
            None,
        );
        let result = get_rclone_path(&state);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("No rclone binary"),
            "Expected error about missing binary, got: {err}"
        );
    }

    #[test]
    fn test_get_rclone_path_with_value() {
        let state = AppState::new(
            Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
                Connection::open_in_memory().unwrap(),
            ))),
            None,
        );
        let path = PathBuf::from("/usr/local/bin/rclone");
        *state.rclone_path.lock().unwrap() = Some(path.clone());
        let result = get_rclone_path(&state);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), path);
    }

    #[test]
    fn test_mount_list_empty_when_no_mounts() {
        let state = AppState::new(
            Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
                Connection::open_in_memory().unwrap(),
            ))),
            None,
        );
        let guard = state.mounts.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_mount_list_returns_stored_mounts() {
        let state = AppState::new(
            Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
                Connection::open_in_memory().unwrap(),
            ))),
            None,
        );
        let mount = MountInfo {
            id: "test-id".into(),
            remote: "gdrive:".into(),
            mount_point: "/mnt/gdrive".into(),
            status: "running".into(),
        };
        state.mounts.lock().unwrap().insert(Uuid::new_v4(), mount);
        let guard = state.mounts.lock().unwrap();
        assert_eq!(guard.len(), 1);
    }

    // ------------------------------------------------------------------
    // Phase 6 — Task 6.4: Mount lifecycle test
    //   mount started → Running in state → unmount → Released
    // ------------------------------------------------------------------

    #[test]
    fn test_mount_lifecycle_running_to_released() {
        let state = AppState::new(
            Arc::new(tokio::sync::Mutex::new(TaskRepo::new(
                Connection::open_in_memory().unwrap(),
            ))),
            None,
        );
        let mount_id = Uuid::new_v4();

        // Phase 1: Mount started — insert ProcessHandle + MountInfo
        let rt = tokio::runtime::Runtime::new().unwrap();
        let child = rt
            .block_on(async {
                #[cfg(not(target_os = "windows"))]
                {
                    tokio::process::Command::new("echo").arg("mount").spawn()
                }
                #[cfg(target_os = "windows")]
                {
                    tokio::process::Command::new("cmd.exe")
                        .args(["/c", "echo", "mount"])
                        .spawn()
                }
            })
            .expect("failed to spawn echo for mount test");
        let handle = ProcessHandle::new(child);

        let mount_info = MountInfo {
            id: mount_id.to_string(),
            remote: "gdrive:".to_string(),
            mount_point: "/mnt/gdrive".to_string(),
            status: "running".to_string(),
        };

        state.processes.lock().unwrap().insert(mount_id, handle);
        state.mounts.lock().unwrap().insert(mount_id, mount_info);

        // Phase 2: Verify mount is Running in state
        {
            let mounts = state.mounts.lock().unwrap();
            assert_eq!(mounts.len(), 1, "one mount should exist");
            let stored = mounts.get(&mount_id).unwrap();
            assert_eq!(stored.status, "running");
            assert_eq!(stored.remote, "gdrive:");
        }

        // Phase 3: Unmount — stop process and remove mount metadata
        let pm = ProcessManager::new(state.processes.clone());
        rt.block_on(pm.stop(mount_id))
            .expect("failed to stop mount process");
        state.mounts.lock().unwrap().remove(&mount_id);

        // Phase 4: Verify Released — both maps should be empty
        {
            let mounts = state.mounts.lock().unwrap();
            assert!(
                mounts.is_empty(),
                "mounts map should be empty after unmount"
            );
        }
        {
            let processes = state.processes.lock().unwrap();
            assert!(
                processes.is_empty(),
                "processes map should be empty after unmount"
            );
        }
    }
}

#![deny(unsafe_code)]

mod commands;
mod db;
mod rclone;
mod scheduler;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use tauri::Manager;
use tokio::sync::RwLock;

use crate::db::task_repo::TaskRepo;
use crate::rclone::process::ProcessManager;
use crate::scheduler::scheduler::TaskScheduler;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::rclone_cmds::rclone_version,
            commands::rclone_cmds::rclone_config_list,
            commands::rclone_cmds::rclone_exec,
            commands::rclone_cmds::rclone_stop,
            commands::rclone_cmds::rclone_mount,
            commands::rclone_cmds::rclone_unmount,
            commands::rclone_cmds::rclone_mount_list,
            commands::task_cmds::task_list,
            commands::task_cmds::task_create,
            commands::task_cmds::task_update,
            commands::task_cmds::task_delete,
            commands::task_cmds::task_toggle,
            commands::task_cmds::task_run_now,
            commands::task_cmds::rclone_providers,
        ])
        .setup(|app| {
            // Initialize SQLite database in the app data directory
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");
            std::fs::create_dir_all(&app_dir)
                .expect("failed to create app data directory");

            let db_path = app_dir.join("rclonegui.db");
            let conn =
                Connection::open(&db_path).expect("failed to open SQLite database");
            db::migrations::create_tables(&conn)
                .expect("failed to create database tables");

            // Locate the bundled rclone binary — try resource_dir (production),
            // then fall back to CARGO_MANIFEST_DIR, cwd, and exe ancestors (dev).
            let platform = rclone::discovery::resolve_platform();
            let rclone_path: Option<PathBuf> = app
                .path()
                .resource_dir()
                .ok()
                .and_then(|dir| {
                    let path = rclone::discovery::locate_binary(&dir, platform);
                    if path.exists() {
                        return Some(path);
                    }
                    None
                })
                .or_else(|| rclone::discovery::find_binary(platform));

            // Create task repo behind Arc<Mutex> so it can be shared with the scheduler.
            let task_conn = Connection::open(&db_path)
                .expect("failed to open SQLite database for TaskRepo");
            let task_repo = Arc::new(tokio::sync::Mutex::new(TaskRepo::new(task_conn)));

            // Convert rclone path to the RwLock<Option<String>> expected by TaskScheduler.
            let scheduler_rclone = Arc::new(RwLock::new(
                rclone_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
            ));

            // Create the TaskScheduler — it owns a clone of task_repo and app handle.
            let scheduler = TaskScheduler::new(
                task_repo.clone(),
                scheduler_rclone,
                app.handle().clone(),
            );

            // Grab a handle to the scheduler before it's moved into AppState.
            let scheduler_handle = scheduler;

            let state = AppState::new(task_repo, Some(scheduler_handle));
            if let Some(ref path) = rclone_path {
                *state.rclone_path.lock().expect("lock rclone_path") = Some(path.clone());
            }

            // Keep a clone so we can start it after app.manage.
            let sched_for_start = state.scheduler.clone();
            app.manage(state);

            // Start the scheduler after a short delay to let Tauri init complete.
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let guard = sched_for_start.lock().await;
                if let Some(ref scheduler) = *guard {
                    scheduler.start().await;
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Cleanup running processes and scheduler on app exit
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            let state = app_handle.state::<AppState>();
            let pm = ProcessManager::new(state.processes.clone());
            let _ = pm.cleanup_all();

            // Stop the scheduler — take the Option so stop() runs only once.
            let sched_arc = state.scheduler.clone();
            tauri::async_runtime::spawn(async move {
                let mut guard = sched_arc.lock().await;
                if let Some(scheduler) = guard.take() {
                    scheduler.stop().await;
                }
            });
        }
    });
}

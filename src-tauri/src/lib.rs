#![deny(unsafe_code)]

mod commands;
mod db;
mod rclone;
mod state;

use std::path::PathBuf;

use rusqlite::Connection;
use tauri::Manager;

use crate::rclone::process::ProcessManager;
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

            let state = AppState::new(conn);
            if let Some(ref path) = rclone_path {
                *state.rclone_path.lock().expect("lock rclone_path") = Some(path.clone());
            }
            app.manage(state);

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Cleanup running processes on app exit
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            let state = app_handle.state::<AppState>();
            let pm = ProcessManager::new(
                state.processes.clone(),
                state.rclone_path.clone(),
            );
            let _ = pm.cleanup_all();
        }
    });
}

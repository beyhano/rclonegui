# Architecture: RcloneGUI

## Technology Stack

| Category | Details |
|---|---|
| Languages | Rust (backend), TypeScript/React (frontend) |
| Frameworks | Tauri v2 (desktop app framework), React 19, Vite 7 |
| Databases | SQLite (via rusqlite, bundled) |
| Auth mechanism | None — local desktop app, no user auth |
| Infrastructure | Tauri v2 bundle (Windows/macOS/Linux), rclone binary bundled per platform |
| External services | rclone CLI (local binary, ~90 storage backends) |

## Architecture Overview

Desktop GUI application wrapping the rclone CLI. **Monolithic** — single process, single window.

Main layers:
- **Rust backend** (src-tauri/): Tauri commands, process management, SQLite persistence, cron-based task scheduler
- **React frontend** (src/): Tab-based UI (Config / Transfer / Mounts / Scheduler), communicates with backend via Tauri `invoke()` and `listen()`

## Data Flow

```
User → React UI → invoke("command", args) → Rust Tauri command handler
  → AppState (processes, mounts, task_repo, scheduler)
  → rclone CLI process (spawned by Rust)
  → stdout/stderr parsed → Tauri events emitted
  → React UI listens via listen("event")
  → Result saved to SQLite
```

Primary flows:
1. **Config**: List remotes (rclone config dump) → create new remote (rclone config create)
2. **Manual transfer**: User enters args → rclone_exec spawns rclone → progress events stream to UI
3. **Scheduled task**: Cron scheduler checks enabled tasks → runs rclone via engine → emits progress events → saves to transfers table

## Entry Points

| Entry Point | Type | Auth Required | Description |
|---|---|---|---|
| rclone_exec | Tauri invoke | No | Execute rclone with arbitrary args |
| rclone_version | Tauri invoke | No | Get rclone version |
| rclone_config_list | Tauri invoke | No | List configured remotes |
| rclone_config_create | Tauri invoke | No | Create new remote |
| rclone_stop | Tauri invoke | No | Stop running process |
| rclone_mount/rclone_unmount | Tauri invoke | No | Mount/unmount remote |
| task_list/create/update/delete/toggle/run_now | Tauri invoke | No | Task CRUD operations |
| rclone_providers | Tauri invoke | No | List all 90+ rclone backends |

## Trust Boundaries

| Boundary | What Crosses It | Risk |
|---|---|---|
| Frontend → Backend | User input via invoke() params | No auth — local app only |
| Backend → rclone CLI | rclone commands with user-supplied args | Command injection via args |
| Backend → SQLite | Transfer records, task definitions | SQL injection via serde_json values stored in DB |
| rclone CLI → OS | File system access, network requests | Path traversal, SSRF via rclone |

## Sensitive Data Inventory

| Data Type | Where Stored | How Accessed | Protection |
|---|---|---|---|
| rclone config (credentials, tokens) | rclone config file (outside app) | Via rclone config dump | rclone obscures passwords |
| Transfer history | SQLite (transfer table) | Via task_list/get_transfer_history | File system permissions only |
| Task definitions | SQLite (tasks table) | Via task_repo | File system permissions only |

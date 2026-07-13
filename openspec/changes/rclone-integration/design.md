# Design: Rclone Integration

## Technical Approach

Layered Rust backend: `rclone/` module for binary lifecycle, `db/` for SQLite persistence, `commands/` for Tauri IPC surface. Frontend consumes events via `listen()` and invokes commands via `invoke()`. SQLite connection lives in Tauri State alongside the process map, initialized on first launch.

## Architecture Decisions

| Decision | Option | Tradeoff | Choice |
|---|---|---|---|
| SQLite lib | `rusqlite` (bundled) vs `sqlx` compile-time checking | sqlx adds async + build-time query checks but requires a build.rs generator; rusqlite is simpler for a local-only DB | `rusqlite` with `bundled` feature — no system SQLite needed |
| Process state sync | `std::sync::Mutex` vs `tokio::sync::Mutex` | Tauri commands are async, but process-map locks are held briefly (insert/remove) — std Mutex is fine | `std::sync::Mutex` — shorter critical sections, no yield cost |
| Regex crate | `regex` vs hand-rolled parser | regex is well-optimized, well-tested; hand-rolled is fragile | `regex` — compiled once via `Regex::new`, named captures |
| Frontend state | `useState` vs Zustand | Three panels share process/mount state; Zustand avoids prop drilling | `useState` per panel initially — refactor to Zustand when >2 panels share state |

## Data Flow

```
User click → invoke("rclone_exec", args)
  → Command handler → ProcessManager::spawn()
    → tokio::process::Command(rclone, args)
    → spawn 2 BufReader tasks (stdout/stderr)
      → regex parse each line → emit("rclone:progress")
    → store Uuid → return Uuid to frontend
  → Frontend maps Uuid → state, listen("rclone:progress") → update ProgressBar

User click "Stop" → invoke("rclone_stop", process_id)
  → start_kill() → wait() → emit("rclone:process-completed")
```

## Module Structure

| Path | Action | Description |
|---|---|---|
| `src-tauri/src/lib.rs` | Modify | Remove greet, add `#![deny(unsafe_code)]`, register modules + state + commands |
| `src-tauri/src/main.rs` | Modify | Add `#![deny(unsafe_code)]` |
| `src-tauri/src/state.rs` | Create | `AppState` — `processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>`, `rclone_path: Arc<Mutex<Option<PathBuf>>>`, `db: Arc<Mutex<rusqlite::Connection>>` |
| `src-tauri/src/rclone/mod.rs` | Create | Re-exports |
| `src-tauri/src/rclone/discovery.rs` | Create | `resolve_platform()`, `locate_binary()`, `verify_executable()` |
| `src-tauri/src/rclone/process.rs` | Create | `ProcessManager` — `spawn()`, `stop()`, `cleanup_all()` |
| `src-tauri/src/rclone/events.rs` | Create | `start_event_stream()` — BufReader + regex + emit |
| `src-tauri/src/rclone/config.rs` | Create | `config_list()` — `rclone config dump` → JSON parse → `Vec<Remote>` |
| `src-tauri/src/db/mod.rs` | Create | `init_db(path)` — open/create, run migrations, return `Connection` |
| `src-tauri/src/db/models.rs` | Create | Transfer record, mount record, AppConfig structs + CRUD |
| `src-tauri/src/db/migrations.rs` | Create | `create_tables()` — CREATE TABLE IF NOT EXISTS |
| `src-tauri/src/commands/mod.rs` | Create | Re-exports |
| `src-tauri/src/commands/rclone_cmds.rs` | Create | All `#[tauri::command]` functions |
| `src/App.tsx` | Modify | Replace greet → tab router |
| `src/ConfigPanel.tsx` | Create | Remote list + badge display |
| `src/TransferPanel.tsx` | Create | Source/dest selectors, progress bar, speed, history table |
| `src/MountPanel.tsx` | Create | Mount list, mount/unmount buttons, status indicators |
| `src/types.ts` | Create | `Remote`, `TransferRecord`, `MountRecord`, `ProgressPayload` |
| `src-tauri/capabilities/` | Create | Capability files for new commands |

## SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS transfers (
    id          TEXT PRIMARY KEY,           -- Uuid string
    remote_src  TEXT NOT NULL,
    remote_dest TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'running', -- running | completed | failed
    progress    REAL DEFAULT 0.0,           -- 0.0 – 100.0
    speed       TEXT,
    started_at  TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT
);

CREATE TABLE IF NOT EXISTS mounts (
    id          TEXT PRIMARY KEY,
    remote      TEXT NOT NULL,
    mount_point TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'running',
    started_at  TEXT NOT NULL,
    pid         INTEGER
);

CREATE TABLE IF NOT EXISTS app_config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

## Tauri Commands

| Command | Signature | Returns |
|---|---|---|
| `rclone_version` | `() -> Result<String>` | `"rclone v1.65.0"` |
| `rclone_config_list` | `(state) -> Result<Vec<Remote>>` | `[{name, type}]` |
| `rclone_exec` | `(app, state, args: Vec<String>) -> Result<String>` | `Uuid` string |
| `rclone_stop` | `(state, process_id: String) -> Result<()>` | Unit or error |
| `rclone_mount` | `(app, state, remote, mount_point) -> Result<String>` | `Uuid` string |
| `rclone_unmount` | `(state, mount_id: String) -> Result<()>` | Unit or error |
| `rclone_mount_list` | `(state) -> Result<Vec<MountInfo>>` | `[{id, remote, mount_point, status}]` |

## Event Namespace

| Event | Payload | Trigger |
|---|---|---|
| `rclone:progress` | `{process_id, transferred, total, percent, speed, eta}` | Progress line matched |
| `rclone:process-started` | `{process_id, command}` | Process spawn succeeds |
| `rclone:process-completed` | `{process_id, exit_code}` | Process exits |
| `rclone:process-error` | `{process_id, exit_code, stderr_lines}` | Non-zero exit |
| `rclone:binary-missing` | `{platform, download_url}` | Binary not found at startup |
| `rclone:mount-status` | `{mount_id, status}` | Mount process state changes |

## Testing Strategy

| Layer | What | How |
|---|---|---|
| Unit | `discovery::resolve_platform()` | Assert `linux-amd64` / `windows-amd64` from `cfg!()` |
| Unit | `events::parse_progress_line()` | Feed fixture lines, assert parsed fields |
| Unit | `db::migrations::create_tables()` | Open `:memory:` SQLite, run migrations, assert tables exist |
| Integration | Process spawn + stop | Spawn `echo`, assert PID tracked, stop, assert cleanup |
| Integration | `rclone_config_list` error path | Mock missing binary, assert `Err` |

## Threat Matrix

All rows N/A — no git/VCS/PR operations, no executable-file classification of user-supplied content. The binary path is a known bundled path; process spawn uses `tokio::process::Command` (no shell injection vector). The only subprocess boundary is ProcessManager, which receives args as `Vec<String>` (not shell strings) and validates binary existence before spawn.

| Boundary | Applicability |
|---|---|
| Documentation-like paths | N/A — no file classification; binary path is hardcoded relative to bundle |
| Git repository selection | N/A — no git operations |
| Commit state | N/A — no git operations |
| Push state | N/A — no git operations |
| PR commands | N/A — no PR operations |

## Migration / Rollout

No data migration. DB tables created on first `init_db()` call via `CREATE TABLE IF NOT EXISTS`. Schema evolves via migration version tracking (future concern). Bundle `rclone-bin/` directory alongside the app binary — CI step to download correct platform binary on build.

## Open Questions

None — all decisions resolved above.

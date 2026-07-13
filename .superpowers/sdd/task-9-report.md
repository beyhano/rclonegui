# Task 9 Report — Wire TaskScheduler into AppState + lib.rs

## Status: ✅ Complete

## Commit

```
ab269c2 feat(core): wire TaskScheduler into AppState and lib.rs setup
```

## Changes

### `src-tauri/src/state.rs`
- Added `scheduler: Arc<tokio::sync::Mutex<Option<TaskScheduler>>>` field to `AppState`
- Changed `task_repo` type from `Arc<std::sync::Mutex<TaskRepo>>` to `Arc<tokio::sync::Mutex<TaskRepo>>` so it matches the `tokio::sync::Mutex` expected by `TaskScheduler::new`
- `AppState::new` now takes `Arc<tokio.sync::Mutex<TaskRepo>>` and `Option<TaskScheduler>`
- Updated the unit test constructor call

### `src-tauri/src/lib.rs`
- Created `task_repo` as `Arc<tokio::sync::Mutex<TaskRepo>>` before AppState, so scheduler gets a clone of the same Arc
- Created a dedicated `Arc<RwLock<Option<String>>>` for the scheduler's rclone path (converting from `PathBuf` to `String`)
- Created `TaskScheduler` during setup
- Stored the scheduler in `AppState`
- After `app.manage()`, spawned a background task that waits 500ms then calls `scheduler.start()`
- On `RunEvent::Exit`, spawns a task that calls `scheduler.stop()` (via `guard.take()` so it runs once)

### `src-tauri/src/commands/task_cmds.rs`
- Made all CRUD commands (`task_list`, `task_create`, `task_update`, `task_delete`, `task_toggle`) async to support `tokio::sync::Mutex::lock().await`
- Scoped DB operations to drop the `tokio::sync::MutexGuard` before locking the scheduler
- `task_create` → calls `scheduler.add_task(&task)` after DB insert
- `task_update` → calls `scheduler.update_task(&task)` after DB update
- `task_delete` → calls `scheduler.remove_task(&id)` after DB delete
- `task_toggle` → calls `scheduler.add_task()` if now enabled, `scheduler.remove_task()` if now disabled

### `src-tauri/src/commands/rclone_cmds.rs`
- Updated test helper constructor calls to pass the new 3-argument signature

## Adaptations

1. **Mutex type**: `TaskScheduler::new` expects `Arc<tokio::sync::Mutex<TaskRepo>>`, not `Arc<std::sync::Mutex<TaskRepo>>`. Changed AppState's `task_repo` to `tokio::sync::Mutex` and updated all callers to use `.lock().await`.

2. **rclone_path conversion**: AppState stores `Arc<Mutex<Option<PathBuf>>>` but scheduler wants `Arc<RwLock<Option<String>>>`. Created a separate `Arc<RwLock<Option<String>>>` in setup, converted via `p.to_string_lossy()`.

## Verification

| Check | Result |
|-------|--------|
| `cargo check` | ✅ Passes (22 pre-existing dead_code warnings, 0 errors) |
| `cargo test` | ✅ 81 pass, 4 pre-existing failures (`echo` not found on Windows) |
| Pre-existing failures identical | ✅ Confirmed via `git stash` — same 4 tests fail on main |

## Report Path

`C:\Users\Beyhan\Desktop\Projeler\Rust\rclonegui\.superpowers\sdd\task-9-report.md`

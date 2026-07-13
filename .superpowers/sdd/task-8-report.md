# Task 8 Report — Task Tauri Commands

## Status: ✅ Complete

**Commit**: `b532182c128dadb3ca28e6fa6ea9a930010c789e`

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `src-tauri/src/commands/task_cmds.rs` | Created | 6 Tauri commands: `task_list`, `task_create`, `task_update`, `task_delete`, `task_toggle`, `rclone_providers` |
| `src-tauri/src/commands/mod.rs` | Modified | Added `pub mod task_cmds;` |
| `src-tauri/src/lib.rs` | Modified | Added `TaskRepo` import, created second SQLite connection for TaskRepo, registered all 6 commands in `invoke_handler` |
| `src-tauri/src/state.rs` | Modified | Added `task_repo: Arc<Mutex<TaskRepo>>` field, updated `AppState::new` signature to accept `(Connection, TaskRepo)` |
| `src-tauri/src/commands/rclone_cmds.rs` | Modified | Updated test code to pass `TaskRepo` to `AppState::new` |

## Commands Implemented

1. **`task_list`** — sync, returns `Vec<Task>` from `TaskRepo::list()`
2. **`task_create`** — sync, validates input, generates slug+UUID, creates task with `enabled: false`
3. **`task_update`** — sync, validates input, preserves original `created_at` and `enabled`, updates rest
4. **`task_delete`** — sync, deletes by ID
5. **`task_toggle`** — sync, toggles `enabled`, updates `updated_at`
6. **`rclone_providers`** — async, runs `rclone config providers`, returns raw JSON

## Adaptations from Brief

| Brief Spec | Adaptation |
|---|---|
| Scheduler wiring (`add_task`, `remove_task`, `update_task`) | **Omitted** — `scheduler` field doesn't exist in AppState yet (Task 9) |
| `state.scheduler.lock().await` pattern | Skipped; commands use sync `state.task_repo.lock()` since AppState uses `std::sync::Mutex` |
| `state.task_repo.lock().await` | Changed to `state.task_repo.lock().map_err()` — AppState uses `std::sync::Mutex`, not `tokio::sync::Mutex` |
| `TaskRepo` not in AppState | Added `task_repo: Arc<Mutex<TaskRepo>>` to AppState with a second SQLite connection |
| Validation rules | Implemented: name/src/dest non-empty, operation ∈ {copy,sync,move,bisync}, cron parsed via `cron::Schedule` |

## Test Results

- `cargo test task`: **18 passed, 0 failed** (all task_repo + scheduler tests pass)
- `cargo test state::tests::test_app_state_creation`: **passed**
- 4 pre-existing test failures (all rclone child-process spawning on Windows — unrelated)
- `cargo check`: compiles with 25 warnings (all pre-existing dead_code/unused_import warnings)

## Cargo Check Result

```
Finished dev profile [unoptimized + debuginfo] target(s) in 3.80s
```
No errors. All warnings are pre-existing (dead_code for unused structs/functions in scheduler, models, process modules).

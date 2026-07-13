# Task 7 Report — TaskScheduler

## Status
✅ Complete — compiles cleanly, committed.

## Commit
`753a972` — `feat(scheduler): add TaskScheduler with cron loop`

## cargo check
Passed. Only pre-existing dead-code warnings (structs/functions not yet wired to consumers).

## Implementation

**File**: `src-tauri/src/scheduler/scheduler.rs` (stub → full implementation)

**Struct**: `TaskScheduler` with:
- `new(repo, rclone_path, app)` — stores shared state
- `start()` — iterates enabled tasks, spawns a tokio loop per task
- `stop()` — sends cancellation signal to every loop via oneshot channels
- `add_task(task)` — spawns a loop if scheduler is started and task is enabled
- `remove_task(task_id)` — cancels the loop for that task via its oneshot sender
- `update_task(task)` — remove + add
- `run_now(task)` — executes immediately via `engine::execute_task`, saves result to `transfers` table, emits Tauri events

**Cron loop** (`spawn_task_loop`):
- `tokio::select!` between cancellation rx and `tokio::time::sleep`
- Calculates next cron time, sleeps until then
- Overlap prevention via `running` Vec (skips if task_id already present)
- Saves transfer record to SQLite via `repo.connection().execute()`
- Emits `task:completed` / `task:error` events via `app.emit()`

## Deviations from Design
- Made `spawn_task_loop` take `&Task` instead of owned `Task` (callers hold references)
- Removed unused `TaskResult` import
- Added `"time"` feature to tokio in Cargo.toml (required for `tokio::time::sleep`)

## Tests
- `test_task_struct_deserialize` — verifies Task construction (passes)

## Report Path
`.superpowers/sdd/task-7-report.md`

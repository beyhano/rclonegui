# Task 8: Task Tauri commands

**Files:**
- Create: `src-tauri/src/commands/task_cmds.rs`
- Modify: `src-tauri/src/commands/mod.rs` (if needed)
- Modify: `src-tauri/Cargo.toml` (if `tokio` features need `sync`)

### Commands

Create `src-tauri/src/commands/task_cmds.rs` with these Tauri commands:

1. **`task_list`** — lists all tasks via `TaskRepo::list()`
2. **`task_create`** — validates input, generates slug, creates task, adds to scheduler
3. **`task_update`** — validates, preserves original `created_at`, updates task, notifies scheduler
4. **`task_delete`** — deletes task, removes from scheduler
5. **`task_toggle`** — toggles enabled/disabled, adds/removes from scheduler
6. **`rclone_providers`** — runs `rclone config providers` and returns the JSON

### Input validation rules
- `name` must not be empty
- `source_provider` and `dest_provider` must not be empty
- `operation` must be one of: `copy`, `sync`, `move`, `bisync`
- `cron_expr` must be a valid cron expression

### Pattern

Follow the same pattern as `rclone_cmds.rs`:
- Use `#[tauri::command]` async pattern
- Access AppState via `State<'_, AppState>`
- For scheduler access: `state.scheduler.lock().await.as_ref().map(|s| s.add_task(...))`

### Important: AppState fields

AppState currently has these relevant fields (defined in state.rs — may need Task 9 to finalize):
- `task_repo: Arc<Mutex<TaskRepo>>` 
- `scheduler: Arc<Mutex<Option<TaskScheduler>>>`
- `rclone_path: Arc<RwLock<Option<String>>>`

Use `state.rclone_path.read().await` for the path, `state.task_repo.lock().await` for the repo, `state.scheduler.lock().await` for the scheduler.

### Registration

Register commands in `src-tauri/lib.rs`'s invoke_handler:
```rust
commands::task_cmds::task_list,
commands::task_cmds::task_create,
commands::task_cmds::task_update,
commands::task_cmds::task_delete,
commands::task_cmds::task_toggle,
commands::task_cmds::rclone_providers,
```

### Verification

```bash
cd src-tauri && cargo check
```
Expected: Compiles (may show dead_code warnings for scheduler field — that's fine, Task 9 wires it)

### Commit

```bash
git add -A && git commit -m "feat(commands): add task CRUD Tauri commands"
```

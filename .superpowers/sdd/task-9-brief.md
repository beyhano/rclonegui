# Task 9: Wire TaskScheduler into AppState + lib.rs setup

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`

### Step 1: Update `state.rs`

Add `scheduler` field to AppState:
```rust
pub scheduler: Arc<Mutex<Option<TaskScheduler>>>,
```

And the `SchedulerStartToken` to stop the scheduler on exit:
```rust
pub scheduler_stop_token: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
```

### Step 2: Update `lib.rs`

In setup, after creating `AppState`:
```rust
// Create TaskScheduler
let scheduler = TaskScheduler::new(
    task_repo.clone(),
    rclone_path_arc.clone(),
    app.handle().clone(),
);
let scheduler_arc = Arc::new(Mutex::new(Some(scheduler)));
```

Pass to AppState, and after `app.manage(state)`:
```rust
// Start the scheduler
let sched = scheduler_arc.clone();
tokio::spawn(async move {
    // Small delay to let Tauri finish initialization
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let guard = sched.lock().unwrap();
    if let Some(ref scheduler) = *guard {
        scheduler.start().await;
    }
});
```

Store the scheduler in AppState.

On Exit, stop the scheduler:
```rust
// In app.run closure on Exit event:
let state = app_handle.state::<AppState>();
let mut guard = state.scheduler.lock().unwrap();
if let Some(scheduler) = guard.take() {
    // scheduler.stop() is async, so spawn it
    tokio::spawn(async move {
        scheduler.stop().await;
    });
}
```

### Key: Reading current state

Read the ACTUAL current state of `state.rs` and `lib.rs` to see how the AppState constructor currently works after Task 8's changes, and adapt accordingly.

### Verification

```bash
cd src-tauri && cargo check
```
Expected: Compiles cleanly with only pre-existing dead_code warnings.

### Commit

```bash
git add -A && git commit -m "feat(core): wire TaskScheduler into AppState and lib.rs setup"
```

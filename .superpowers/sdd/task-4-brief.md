# Task 4: Fix process-completed/error events in events.rs

**Files:**
- Modify: `src-tauri/src/commands/rclone_cmds.rs`

**The Problem:** The frontend listens for `rclone:process-completed` and `rclone:process-error` events, but these are never emitted by the backend. The event stream reads stdout/stderr but when those streams close (process exits), nothing emits a completion event. Transfer panel stays "running" forever.

**The Fix:** In `rclone_exec`, after calling `start_event_stream()`, capture the returned `JoinHandle` and spawn a monitoring task that awaits it, then emits `rclone:process-completed`.

### Step 1: Modify `rclone_exec` in `rclone_cmds.rs`

Current code (lines 106-123):
```rust
// Spawn background event stream for stdout/stderr
start_event_stream(
    app.clone(),
    id,
    BufReader::new(stdout),
    BufReader::new(stderr),
);

let _ = app.emit(
    "rclone:process-started",
    serde_json::json!({ ... }),
);

Ok(id.to_string())
```

Change to:
```rust
// Spawn background event stream for stdout/stderr
let event_handle = start_event_stream(
    app.clone(),
    id,
    BufReader::new(stdout),
    BufReader::new(stderr),
);

// Monitor when event stream finishes → process exited → emit completion
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
```

### Verification

```bash
cd src-tauri && cargo check
```
Expected: No errors (note: the existing tests that spawn "echo" will still fail on Windows — that's pre-existing)

### Commit

```bash
git add -A && git commit -m "fix(events): emit rclone:process-completed when event stream ends"
```

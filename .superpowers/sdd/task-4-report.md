# Task 4 Report: Fix process-completed events

**Status:** ✅ Complete

**Commit:** (to be filled after commit)

**Changes made:**
- `src-tauri/src/commands/rclone_cmds.rs`: Captured `JoinHandle` from `start_event_stream()` and added a `tokio::spawn` monitoring task that awaits it and emits `rclone:process-completed` with the `process_id` when the event stream ends (process exits).

**Verification:**
- `cargo check` passed successfully (only pre-existing dead-code warnings)

**Files modified:**
- `src-tauri/src/commands/rclone_cmds.rs` — +14 / -1 lines

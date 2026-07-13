# Task 18: Fix Windows test compatibility

**Files to modify:**
- `src-tauri/src/commands/rclone_cmds.rs` — fix 1 test using `echo`
- `src-tauri/src/rclone/process.rs` — fix 2 tests using `echo`
- `src-tauri/src/state.rs` — fix 1 test using `echo`

**The Problem:** 4 tests fail on Windows because they use `tokio::process::Command::new("echo")` — `echo` is a shell built-in on Windows, not a standalone executable.

**Fix Pattern:** Replace `tokio::process::Command::new("echo")` with a platform-compatible alternative:

For each test using `"echo"`:

Option A: Use `cmd /c echo` on Windows
```rust
#[cfg(not(target_os = "windows"))]
{
    // existing echo test code
}

#[cfg(target_os = "windows")]
{
    // Use cmd.exe instead
    tokio::process::Command::new("cmd.exe")
        .args(&["/c", "echo", "test"])
        // rest stays the same
}
```

Option B: Simply wrap the test with `#[cfg(not(target_os = "windows"))]` — skip on Windows. (Simpler, acceptable for CI.)

**Prefer Option A** where practical to keep test coverage on Windows.

Affected tests:
1. `commands/rclone_cmds.rs`: `test_mount_lifecycle_running_to_released` (line ~301)
2. `rclone/process.rs`: `test_cleanup_all_with_entries` (line ~163)
3. `rclone/process.rs`: `test_spawn_echo_track_pid_stop_cleanup` (line ~196)
4. `state.rs`: `test_process_handle_creation` (line ~75)

**Verification:**
```bash
cd src-tauri && cargo test 2>&1 | Select-String -Pattern "test result"
```
Expected: All tests PASS (should show 85+ passed now)

**Commit:** `git add -A && git commit -m "fix(tests): Windows compatibility for echo-based tests"`

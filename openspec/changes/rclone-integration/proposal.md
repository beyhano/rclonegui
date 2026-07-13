# Proposal: Rclone Integration

## Intent

Add a Rust backend layer that discovers the rclone binary, manages async processes (config, copy/sync, mount), streams progress events to the frontend, and provides visual UI panels for config, transfers, and mounts. Currently the app is a default Tauri scaffold with only a `greet` command — this change delivers the core value proposition.

## Scope

### In Scope
- Rclone binary discovery in `rclone-bin/{platform}/` with platform auto-detection
- Async process lifecycle: spawn, track PID, graceful stop, cleanup on exit
- Event pipeline: BufReader stdout/stderr → regex parse → `app_handle.emit()`
- Tauri commands: `rclone_version`, `rclone_config_list`, `rclone_exec`, `rclone_stop`, `rclone_mount`, `rclone_unmount`
- React UI: config list page, transfer panel (source/dest selectors + progress bar), mount management
- Tauri capabilities for new commands

### Out of Scope
- macOS binary deployment (deferred; OS detection code present but untested)
- Rclone config wizard (add/edit remote via CLI passthrough only)
- Advanced flags UI (users pass raw args via rclone_exec)
- History/persistence of completed transfers

## Capabilities

### New Capabilities
- `rclone-binary-discovery`: Detect platform, locate `rclone-bin/{platform}/rclone` (or `.exe`), verify executable, emit error + download link if missing
- `rclone-process-management`: `tokio::process::Command` spawn, `Arc<Mutex<HashMap<Uuid, ProcessHandle>>>` state, `kill_on_drop(true)`, graceful shutdown on app exit
- `rclone-event-stream`: BufReader → line-by-line regex parse (`Transferred: ... %, ... MiB/s, ETA`) → `rclone:progress`, `rclone:process-completed`, `rclone:process-error` events
- `rclone-config`: List remotes via `rclone config dump` → JSON parse → frontend display
- `rclone-transfer`: Copy/sync invocation with real-time progress bar and speed indicator
- `rclone-mount`: Mount/unmount management with status display

### Modified Capabilities
- None (no existing specs)

## Approach

Backend: Add `rclone` module (discovery, process, events) to `src-tauri/src/`. Register state and commands in `lib.rs`. Frontend: Replace greet UI with three tabbed panels (Config, Transfer, Mounts). Wire `invoke()` calls and `listen()` handlers.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modified | Remove greet, register new commands and state |
| `src-tauri/src/main.rs` | None | Already minimal |
| `src-tauri/Cargo.toml` | Modified | Add tokio, regex, uuid, chrono deps |
| `src/App.tsx` | Modified | Replace with tabbed router |
| `src/` | New | Config, Transfer, Mount panels and types |
| `rclone-bin/` | New | Pre-bundled binary directories |
| `docs/wiki/Rclone_Integration.md` | Modified | Sync with implementation |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Binary not found on user's platform | Medium | Clear error UI with download link, deferred macOS support acknowledged |
| Zombie processes on crash | Low | `kill_on_drop(true)` + `setup()` hook cleanup |
| Rclone output format changes | Low | Regex parsing in isolated module, easy to patch |

## Rollback Plan

Revert changes to `src-tauri/src/`, `src/`, and `rclone-bin/`. Remove new cargo deps via `cargo remove`. Restore `lib.rs` greet command and `App.tsx` greet form. No DB or config migration needed — stateless integration.

## Dependencies

- tokio (async runtime — already in Cargo.toml deps trace)
- regex (progress parsing)
- uuid (process IDs)
- chrono (timestamps, optional)

## Success Criteria

- [ ] `rclone_version` returns binary version string
- [ ] `rclone_config_list` returns parsed remote list
- [ ] Transfer (copy/sync) shows real-time progress bars on frontend
- [ ] Running processes appear in state and can be stopped via `rclone_stop`
- [ ] Mount/unmount works and status is reflected in UI
- [ ] Missing binary shows error with download link (no crash)
- [ ] `cargo build` passes with `#![deny(unsafe_code)]`

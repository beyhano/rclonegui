# Rclone Process Management Specification

## Purpose

Manage the full lifecycle of async rclone child processes: spawn via `tokio::process::Command`, track PIDs and state in an `Arc<Mutex<HashMap<Uuid, ProcessHandle>>>`, support graceful stop, and ensure cleanup on application exit.

## Requirements

### Requirement: Process Spawn

The system MUST spawn rclone as an async child process using `tokio::process::Command` with `.kill_on_drop(true)` and `Stdio::piped()` for both stdout and stderr.

#### Scenario: Successful spawn

- GIVEN the rclone binary path is cached and valid in State
- WHEN `ProcessManager::spawn(args)` is called with valid arguments
- THEN it MUST return a `Uuid`
- AND the process MUST appear in the process map with status `Running`

#### Scenario: Spawn with missing binary

- GIVEN no rclone binary path is cached
- WHEN `ProcessManager::spawn()` is called
- THEN it MUST return an error: `"No rclone binary configured"`

### Requirement: Process Tracking

The system SHALL store each spawned process in `Arc<Mutex<HashMap<Uuid, ProcessHandle>>>` where `ProcessHandle` contains: `child`, `pid`, `command`, `started_at`.

#### Scenario: Process appears in state

- GIVEN a process is spawned successfully
- WHEN the process map is queried
- THEN the entry MUST show the correct `pid` and `command` string
- AND `status` MUST be `Running`

### Requirement: Graceful Stop

The system MUST support stopping a running process by UUID. It SHALL attempt `child.start_kill()` first, then force-kill after a 5-second timeout.

#### Scenario: Stop running process

- GIVEN a process with UUID `abc-123` is in `Running` state
- WHEN `ProcessManager::stop("abc-123")` is called
- THEN the process MUST receive `SIGTERM` (Unix) or equivalent
- AND on exit the status MUST become `Completed(exit_code)`

#### Scenario: Stop nonexistent process

- GIVEN no process with UUID `xyz-999` exists
- WHEN `ProcessManager::stop("xyz-999")` is called
- THEN it MUST return an error: `"Process not found: xyz-999"`

### Requirement: Application Exit Cleanup

On Tauri `RunEvent::Exit` or `on_window_event(CloseRequested)`, the system MUST iterate all tracked processes, send stop signals, and wait up to 5 seconds before force-killing survivors.

#### Scenario: All processes cleaned on exit

- GIVEN 3 processes are running
- WHEN the application exits
- THEN every child process MUST be terminated within 5 seconds
- AND no orphan/zombie processes remain

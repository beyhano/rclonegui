# Rclone Transfer Specification

## Purpose

Execute rclone copy/sync operations as async processes with real-time progress reporting via the event stream, and allow user-initiated cancellation.

## Requirements

### Requirement: Transfer Command

The system MUST implement a Tauri command `rclone_exec` that accepts `args: Vec<String>`, spawns an rclone process with those arguments, attaches the event stream pipeline, and returns the process `Uuid`.

#### Scenario: Start copy operation

- GIVEN `args: ["copy", "/local/path", "gdrive:backup", "--progress"]`
- WHEN `rclone_exec` is invoked
- THEN it MUST return a `Uuid`
- AND `rclone:process-started` MUST be emitted
- AND the process MUST appear in state with status `Running`

#### Scenario: Start with empty args

- GIVEN `args: []` (empty vector)
- WHEN `rclone_exec` is invoked
- THEN it MUST return `Err("At least one argument required")`

### Requirement: Stop Transfer

The system MUST implement a Tauri command `rclone_stop` that accepts a `process_id: String`, looks up the process, and stops it via `ProcessManager::stop`.

#### Scenario: Stop active transfer

- GIVEN a transfer with UUID `abc-123` is running
- WHEN `rclone_stop("abc-123")` is invoked
- THEN the process MUST receive a stop signal
- AND the frontend MUST receive `rclone:process-completed` with the exit code

#### Scenario: Stop already-completed transfer

- GIVEN a transfer with UUID `abc-123` is already `Completed(0)`
- WHEN `rclone_stop("abc-123")` is invoked
- THEN it MUST return `Err("Process abc-123 is not running")`

### Requirement: Progress Reporting

Every rclone process spawned via `rclone_exec` MUST attach the event stream pipeline. Progress events MUST include the `process_id` for frontend correlation.

#### Scenario: Progress tied to process

- GIVEN a copy process with UUID `abc-123` is running
- WHEN `rclone:progress` events arrive
- THEN each payload MUST contain `process_id: "abc-123"`
- AND the percent value MUST be monotonically increasing

### Requirement: Version Command

The system MUST implement `rclone_version` that runs `rclone version`, captures the first line, and returns the version string.

#### Scenario: Version returned

- GIVEN rclone binary is available
- WHEN `rclone_version` is invoked
- THEN it MUST return a string like `"rclone v1.65.0"`

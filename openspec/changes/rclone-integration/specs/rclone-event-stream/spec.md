# Rclone Event Stream Specification

## Purpose

Read rclone process stdout/stderr via `tokio::io::BufReader`, parse progress lines with regex, and emit structured Tauri events to the frontend in real time.

## Requirements

### Requirement: Stdout/Stderr Reading

For each spawned process, the system MUST spawn two async tasks reading stdout and stderr concurrently via `tokio::io::BufReader`.

#### Scenario: Lines read from stdout

- GIVEN a running rclone copy process with `--progress`
- WHEN lines are emitted to stdout
- THEN each line MUST be available for the parser within 100ms of emission

### Requirement: Progress Line Parsing

The system MUST compile a regex matching rclone progress lines: `Transferred: <transferred> / <total>, <percent>%, <speed>, ETA <eta>`. Named capture groups SHALL extract: `transferred`, `total`, `percent`, `speed`, `eta`.

#### Scenario: Full progress line parsed

- GIVEN stdout line: `Transferred: 1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s`
- WHEN the regex parser matches
- THEN percent MUST be `100`, speed MUST be `"12.034 MiB/s"`, eta MUST be `"0s"`

#### Scenario: Non-progress line (passthrough)

- GIVEN stdout line: `INFO  : file.txt: Copied (new)`
- WHEN the regex does NOT match
- THEN the line MUST be emitted as `rclone:log` instead

### Requirement: Event Emission

When a progress line is parsed, the system MUST emit `rclone:progress` with a `ProgressPayload` (transferred, total, percent, speed, eta, process_id). Other events MUST follow the `rclone:` namespace: `rclone:process-started`, `rclone:process-completed` (with exit code), `rclone:process-error` (with message).

#### Scenario: Progress event reaches frontend

- GIVEN a process is running and emitting progress
- WHEN `rclone:progress` is emitted
- THEN the frontend `listen("rclone:progress")` callback MUST fire with typed payload

#### Scenario: Process completed event

- GIVEN a process exits with code `0`
- WHEN the exit is detected
- THEN `rclone:process-completed` MUST be emitted with `process_id` and `exit_code: 0`

### Requirement: Process Error Event

If a process exits with a non-zero exit code, the system MUST emit `rclone:process-error` with the process ID, exit code, and last 5 stderr lines.

#### Scenario: Non-zero exit

- GIVEN a process exits with code `1`
- WHEN exit is detected
- THEN `rclone:process-error` MUST include `exit_code: 1` and the last stderr lines

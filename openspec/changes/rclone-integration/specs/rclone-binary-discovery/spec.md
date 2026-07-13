# Rclone Binary Discovery Specification

## Purpose

Detect the host platform, locate the rclone binary within the bundled `rclone-bin/{platform}/` directory, verify it is executable, and surface a descriptive error with a download link when the binary is missing. This runs once at startup and caches the resolved path in Tauri State.

## Requirements

### Requirement: Platform Detection

The system MUST auto-detect the host OS and architecture at startup using `std::env::consts::OS` and `std::env::consts::ARCH`.

#### Scenario: Linux x86_64 detection

- GIVEN the app starts on a Linux x86_64 host
- WHEN the binary discovery module initializes
- THEN it MUST resolve the platform string to `"linux-amd64"`

#### Scenario: Windows x86_64 detection

- GIVEN the app starts on a Windows x86_64 host
- WHEN the binary discovery module initializes
- THEN it MUST resolve the platform string to `"windows-amd64"`

### Requirement: Binary Location

The system MUST locate the rclone binary at `rclone-bin/{platform}/rclone` (Unix) or `rclone-bin/{platform}/rclone.exe` (Windows), relative to the application resource directory, using `std::path::PathBuf` for cross-platform paths.

#### Scenario: Binary found at expected path

- GIVEN the platform resolves to `"linux-amd64"` and `rclone-bin/linux-amd64/rclone` exists
- WHEN discovery runs
- THEN the system MUST return the absolute path to the binary

#### Scenario: Binary missing

- GIVEN the platform resolves to `"linux-amd64"` and `rclone-bin/linux-amd64/rclone` does NOT exist
- WHEN discovery runs
- THEN the system MUST emit `rclone:binary-missing` event with an error payload
- AND the payload MUST include a `download_url` field pointing to the official rclone releases page

### Requirement: Executable Verification

The system MUST verify the binary is executable via `std::fs::metadata().permissions()` before accepting it. On Unix, this checks the executable bit; on Windows, `.exe` extension suffices.

#### Scenario: Binary is executable

- GIVEN the located binary has execute permissions
- WHEN discovery validates it
- THEN the system MUST store the path in Tauri `State<RcloneState>`

#### Scenario: Binary lacks execute permission

- GIVEN the located binary exists but is NOT executable
- WHEN discovery validates it
- THEN the system MUST return an error: `"Binary found but is not executable: {path}"`

### Requirement: Cached Path

The resolved binary path MUST be written to Tauri State as `rclone_path: Arc<Mutex<Option<PathBuf>>>` and remain available for all subsequent commands.

#### Scenario: Path accessible from commands

- GIVEN discovery completed successfully and path is cached
- WHEN any rclone command runs
- THEN it MUST read the cached path from State
- AND it MUST NOT re-discover on each invocation

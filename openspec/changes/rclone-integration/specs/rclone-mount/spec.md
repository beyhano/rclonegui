# Rclone Mount Specification

## Purpose

Manage rclone mount and unmount operations as long-running background processes, track their status, and allow user-initiated unmount.

## Requirements

### Requirement: Mount Command

The system MUST implement a Tauri command `rclone_mount` that accepts `remote: String` and `mount_point: String`, constructs `rclone mount <remote>: <mount_point> [flags]`, spawns it as a managed process, and returns the process `Uuid`.

#### Scenario: Mount started successfully

- GIVEN `remote: "gdrive"` and `mount_point: "/mnt/gdrive"`
- WHEN `rclone_mount` is invoked
- THEN it MUST return a `Uuid`
- AND `rclone:process-started` MUST be emitted with type `mount`
- AND the mount status MUST appear as `Running` in state

#### Scenario: Mount with nonexistent remote

- GIVEN `remote: "nonexistent"` and a valid mount point
- WHEN `rclone_mount` is invoked
- THEN the process MAY start and fail
- AND on failure `rclone:process-error` MUST be emitted with the stderr output

### Requirement: Unmount Command

The system MUST implement a Tauri command `rclone_unmount` that accepts `process_id: String`, stops the mount process, and verifies the mount point is released.

#### Scenario: Unmount successful

- GIVEN a mount process with UUID `abc-123` is running at `/mnt/gdrive`
- WHEN `rclone_unmount("abc-123")` is invoked
- THEN the process MUST be stopped
- AND the mount point `/mnt/gdrive` MUST be released
- AND `rclone:process-completed` MUST be emitted

#### Scenario: Unmount nonexistent process

- GIVEN no mount with UUID `xyz-000` exists
- WHEN `rclone_unmount("xyz-000")` is invoked
- THEN it MUST return `Err("Mount process not found: xyz-000")`

### Requirement: Mount Status Tracking

The system SHOULD track mount-specific metadata (remote name, mount point path, uptime) alongside standard process info. This SHALL be queryable via an `rclone_mount_list` command.

#### Scenario: List active mounts

- GIVEN two active mount processes for `gdrive` and `dropbox`
- WHEN `rclone_mount_list` is invoked
- THEN it MUST return a list with 2 entries
- AND each entry MUST contain `remote`, `mount_point`, and `status`

#### Scenario: No active mounts

- GIVEN no mount processes are running
- WHEN `rclone_mount_list` is invoked
- THEN it MUST return an empty list

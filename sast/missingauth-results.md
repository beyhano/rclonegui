# Missing Auth/Authz Analysis Results: RcloneGUI

## Executive Summary
- Endpoints analyzed: 15
- Vulnerable: 0
- Likely Vulnerable: 0
- Not Vulnerable: 15
- Needs Manual Review: 0

## Findings

### [NOT VULNERABLE] rclone_version
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 39-58)
- **Endpoint**: `tauri::invoke -> rclone_version`
- **Protection**: No auth required, but this is a local desktop application (Tauri v2) with no network-exposed endpoints. The architecture explicitly documents "Auth mechanism: None - local desktop app, no user auth." The Tauri IPC is sandboxed and only accessible from the app's own WebView context. OS-level user account access is the implicit authentication boundary. This command returns the rclone version string (read-only, informational).

### [NOT VULNERABLE] rclone_config_list
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 61-65)
- **Endpoint**: `tauri::invoke -> rclone_config_list`
- **Protection**: Returns rclone remote config (with obfuscated credentials by rclone). Can only be invoked from within the Tauri WebView via IPC. No network endpoint exposed. The risk surface is limited to a local attacker who already has desktop/machine access. Read-only operation.

### [NOT VULNERABLE] rclone_exec
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 72-136)
- **Endpoint**: `tauri::invoke -> rclone_exec`
- **Protection**: This is the most sensitive command (executes rclone with arbitrary user-supplied arguments), but it is protected by the Tauri security model. Only the app's own frontend can invoke it via IPC. No HTTP/network API is exposed. An attacker would need local machine access to reach this command. The Tauri runtime provides capabilities-based security scoping for invoke handlers.

### [NOT VULNERABLE] rclone_stop
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 139-146)
- **Endpoint**: `tauri::invoke -> rclone_stop`
- **Protection**: Stops a running rclone process by UUID. Only invocable from within the Tauri IPC context. No network exposure. Validates UUID format before proceeding.

### [NOT VULNERABLE] rclone_mount
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 153-208)
- **Endpoint**: `tauri::invoke -> rclone_mount`
- **Protection**: Mounts a remote filesystem to a local mount point. Protected by Tauri IPC scope - no HTTP endpoint. Requires local OS access to invoke. The mount operation is the user's intended use of the application.

### [NOT VULNERABLE] rclone_unmount
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 214-227)
- **Endpoint**: `tauri::invoke -> rclone_unmount`
- **Protection**: Unmounts a running mount process by UUID. Protected by Tauri IPC - not network-exposed. Only accessible from app WebView. Validates UUID format.

### [NOT VULNERABLE] rclone_mount_list
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 230-235)
- **Endpoint**: `tauri::invoke -> rclone_mount_list`
- **Protection**: Lists active mount processes with metadata (read-only). Same Tauri IPC protection as other commands. No sensitive data exposure beyond what the app's own UI already displays.

### [NOT VULNERABLE] rclone_config_create
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 240-275)
- **Endpoint**: `tauri::invoke -> rclone_config_create`
- **Protection**: Creates/modifies rclone remote config with provider credentials. Protected by Tauri IPC boundary. Only the app's own frontend can invoke. The credentials eventually stored in rclone's config file (outside the app) are already the user's own data. This is core application functionality.

### [NOT VULNERABLE] task_list
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 76-80)
- **Endpoint**: `tauri::invoke -> task_list`
- **Protection**: Lists all scheduled tasks from SQLite. Only accessible via Tauri IPC. No network API. Read-only operation returning the user's own data.

### [NOT VULNERABLE] task_create
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 87-139)
- **Endpoint**: `tauri::invoke -> task_create`
- **Protection**: Creates scheduled tasks in SQLite + notifies scheduler. Protected by Tauri IPC. Includes input validation for cron expressions and operation types (copy/sync/move/bisync).

### [NOT VULNERABLE] task_update
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 147-206)
- **Endpoint**: `tauri::invoke -> task_update`
- **Protection**: Updates scheduled task configuration. Protected by Tauri IPC. Input validation for cron syntax and operation type. Scheduler is notified of changes.

### [NOT VULNERABLE] task_delete
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 209-223)
- **Endpoint**: `tauri::invoke -> task_delete`
- **Protection**: Deletes tasks from SQLite and removes from scheduler. Protected by Tauri IPC. No network exposure.

### [NOT VULNERABLE] task_toggle
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 229-259)
- **Endpoint**: `tauri::invoke -> task_toggle`
- **Protection**: Enables/disables a scheduled task. Protected by Tauri IPC. Only the app's own frontend can invoke via IPC. Validates task existence before toggle.

### [NOT VULNERABLE] task_run_now
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 262-281)
- **Endpoint**: `tauri::invoke -> task_run_now`
- **Protection**: Runs a scheduled task immediately (bypassing cron schedule). Protected by Tauri IPC. Validates task existence. Only accessible from within the application WebView context.

### [NOT VULNERABLE] rclone_providers
- **File**: `src-tauri/src/commands/task_cmds.rs` (lines 286-306)
- **Endpoint**: `tauri::invoke -> rclone_providers`
- **Protection**: Lists available rclone backend providers (read-only). Protected by Tauri IPC. Returns public rclone metadata - no sensitive data exposure.

## Architecture Context

This application is a **local desktop GUI** built with Tauri v2. Key architectural properties that make missing authentication a non-issue by design:

1. **No network attack surface**: All 15 commands are Tauri `invoke` handlers, NOT HTTP/REST endpoints. They are only accessible via Tauri's IPC mechanism from within the app's own WebView context.

2. **Single-user desktop app**: The app runs under a single OS user account. The OS user boundary IS the authentication boundary. There are no multi-user sessions, no user accounts within the app, and no network services.

3. **Sandboxed by Tauri**: Tauri v2 provides a capability-based security model. Commands can only be invoked by the app's own frontend code. An external attacker cannot call `invoke("rclone_exec", ...)` without first compromising the user's system.

4. **Explicit design**: The architecture document states "Auth mechanism: None - local desktop app, no user auth" and "No auth - local app only" for every trust boundary crossing.

## Remediation (if architecture changes)

If this app ever becomes network-facing (e.g., adding an HTTP server for remote access), ALL 15 endpoints would become Vulnerable - missing authentication. In that scenario, every endpoint would need:
- Authentication middleware (JWT/session validation)
- Role-based access control for admin operations (rclone_exec, rclone_config_create, task_run_now)
- Scoped capability model (read-only vs read-write vs admin)

# SSRF Analysis Results: rcloneGUI

## Executive Summary
- Outbound call sites analyzed: 5
- Vulnerable: 0
- Likely Vulnerable: 0
- Not Vulnerable: 3
- Needs Manual Review: 2

## Architecture Context

This is a **Tauri v2 local desktop application**. Important architectural properties that affect SSRF analysis:

1. **No inbound network listener**: There is no HTTP server or any other network listener. The application communicates solely via Tauri IPC (inter-process communication) between its own Rust backend and React frontend within the same process. A remote attacker cannot send requests to this application.

2. **No privilege boundary**: The user supplying input via the UI has the same machine-level and network-level access as the application itself. The app does not sit in a privileged network position relative to its user.

3. **All outbound network requests go through rclone**: The app does not make direct HTTP requests. It spawns the rclone CLI binary as a child process, which then connects to the configured storage backends. This is by design — rclone is a file synchronization tool.

4. **SSRF threat model mismatch**: Traditional SSRF (as defined by OWASP) requires a server-side component that an attacker can induce to make unauthorized requests to internal resources. Since this is a local desktop app with no remote attacker-facing surface, the traditional SSRF attack is not applicable. The "server" and "attacker" are the same user on the same machine.

## Findings

### [NEEDS MANUAL REVIEW] rclone_exec — user-controlled args passed to rclone subprocess
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 72-136)
- **Endpoint / function**: `rclone_exec` (Tauri command)
- **Issue**: The entire `args: Vec<String>` parameter is passed directly from the frontend `invoke()` call. In `TransferPanel.tsx:124-126`, the frontend constructs `args` as `["copy", source, dest]` where `source` and `dest` are free-text user inputs. These are passed verbatim to `tokio::process::Command::new(&path).args(&args)`, which spawns rclone. rclone can make outbound network connections to any of its ~90 supported storage backends based on these args (S3, HTTP, SFTP, WebDAV, etc.).
- **Taint trace**: `TransferPanel.tsx:124-126` → `invoke("rclone_exec", { args: ["copy", source, dest] })` → Rust `rclone_exec(args)` → `Command::new(&path).args(&args).spawn()` → rclone subprocess makes outbound connections
- **Impact**: The code pattern matches SSRF (user input → outbound network call destination). However, this is a local desktop app with no remote attack surface. Exploitation requires physical or remote desktop access to the user's machine. The "attacker" would already have the same network access as the application, making this a **command injection / arbitrary subprocess args** concern rather than traditional SSRF.
- **Uncertainty**: Whether this constitutes SSRF depends entirely on whether a privilege boundary exists between input sources and the outbound call. In a server-side context, this would be a clear SSRF vulnerability. In this local desktop app, it is not exploitable for SSRF because there is no pathway for a remote attacker to inject requests. Manual review should determine if the app is ever deployed in a server-like context (e.g., as a remote desktop application, web-wrapped, or with shared session access).
- **Suggestion**: Review the deployment model. If this app is ever used in a server context (served over the web, accessible to multiple users, or running with elevated network access), this would become a critical SSRF vulnerability. For the current local desktop deployment model, consider hardening against command injection instead.
- **Remediation (if deployed in server context)**: Apply a strict allowlist of permitted destination prefixes for rclone operations, or validate that source/dest follow expected `remote_name:path` patterns.

### [NEEDS MANUAL REVIEW] execute_task — scheduled task execution with user-supplied source/dest
- **File**: `src-tauri/src/scheduler/engine.rs` (lines 21-91)
- **Endpoint / function**: `execute_task` (internal, called by scheduler)
- **Issue**: The `task.source_provider` and `task.dest_provider` fields from the Task struct are used directly as rclone args. These were originally supplied by the user through `task_create` / `task_update` and persist in SQLite before being read back by the scheduler. rclone makes outbound connections to the specified providers. Same architectural context as rclone_exec — local desktop, no remote attacker surface.
- **Taint trace**: `TaskFormModal.tsx:62-70` → `invoke("task_create", { sourceProvider, destProvider, ... })` → SQLite → `TaskScheduler` → `execute_task(task)` → `Command::new(rclone_path).args(&args).spawn()` → rclone outbound connections
- **Uncertainty**: Same as rclone_exec — code pattern matches SSRF but architectural context makes traditional SSRF impossible for the current deployment model. The stored/persistent nature adds a delayed execution vector.
- **Suggestion**: Same review as rclone_exec — deployment model determines SSRF applicability. Additionally, stored values in SQLite could be exploited if another attack vector (e.g., SQL injection) allows modifying task data. Currently the tasks table is only written through validated Tauri commands, so stored-value SSRF requires another compromise first.

### [NOT VULNERABLE] rclone_config_create — configures a remote, no outbound request at call time
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 241-275)
- **Endpoint / function**: `rclone_config_create` (Tauri command)
- **Reason**: This function runs `rclone config create --non-interactive` which writes a remote configuration to rclone's config file. The actual outbound network request would occur only later when the configured remote is used in an operation (copy, sync, mount). The `rclone config create` command itself only performs local file I/O. The stored remote configuration is a precondition for SSRF but not an SSRF vector itself.

### [NOT VULNERABLE] Hardcoded rclone commands (version, config dump, config providers)
- **File**: `src-tauri/src/commands/rclone_cmds.rs` / `task_cmds.rs`
- **Endpoint / function**: `rclone_version`, `rclone_config_list`, `rclone_providers`
- **Reason**: These functions call `rclone version`, `rclone config dump`, and `rclone config providers` with completely hardcoded arguments. Zero user influence over the destination. These are informational commands with no network requests.

### [NOT VULNERABLE] rclone_mount — user-controlled remote passed to rclone subprocess
- **File**: `src-tauri/src/commands/rclone_cmds.rs` (lines 153-209)
- **Endpoint / function**: `rclone_mount` (Tauri command)
- **Reason**: Same architectural context as rclone_exec and execute_task. The `remote` and `mount_point` are user-controlled and passed to rclone mount. For traditional SSRF, this is not applicable because the app runs locally with no remote attacker surface. The user providing input already has full network access to their machine. Classified as Not Vulnerable given the local desktop context; reclassify if deployment model changes.

## Summary

The codebase exhibits the *code pattern* of SSRF in 3 locations (rclone_exec, execute_task, rclone_mount) where user input reaches an outbound network call through rclone subprocess spawning. However, **this is not exploitable for SSRF** in the current deployment context because:

1. No inbound network listener exists (no web server, no API endpoint)
2. All commands are triggered exclusively from the local app UI via Tauri IPC
3. The user providing input has the same network access as the application
4. There is no privilege boundary between input source and outbound request

The primary security concern for these code paths is **command injection** rather than SSRF — user-controlled strings flow into a subprocess command, which could allow arbitrary argument injection against the rclone binary. This is a separate vulnerability class (covered by the RCE analysis).

**Recommendation**: If the application is ever deployed in a server context (served to remote users, running as a network-accessible service), all 3 "Needs Manual Review" findings would become critical SSRF vulnerabilities and require immediate remediation with strict host/prefix allowlists.

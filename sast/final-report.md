# Security Assessment Final Report

**Project**: RcloneGUI
**Generated**: 2026-07-14
**Scans completed**: IDOR, SQLi, SSRF, XSS, RCE, XXE, File Upload, Path Traversal, SSTI, JWT, Missing Auth, Business Logic, GraphQL

---

## Executive Summary

| Severity | Count |
|----------|-------|
| Critical | 0 |
| High     | 0 |
| Medium   | 2 |
| Low      | 0 |
| **Total confirmed findings** | **2** |

Scans with no confirmed vulnerabilities: IDOR, SQLi, SSRF, XSS, RCE, XXE, File Upload, Path Traversal, SSTI, JWT, Missing Auth, GraphQL
Findings requiring manual review: 2 (see SSRF results for details)

---

## Vulnerability Index

| # | Title | Type | Severity | Endpoint / File |
|---|-------|------|----------|----------------|
| 1 | Disabled task execution via `task_run_now` | Business Logic | Medium | `task_cmds.rs` — `task_run_now` |
| 2 ⚠ | Provider name flag injection in task CRUD | Business Logic | Medium | `task_cmds.rs` — `task_create` / `task_update` |

---

## Findings

### Medium

#### Disabled Task Execution via `task_run_now` — Business Logic

- **Source scan**: `sast/businesslogic-results.md`
- **Classification**: Vulnerable
- **Endpoint / File**: `src-tauri/src/commands/task_cmds.rs:task_run_now`
- **Severity rationale**: Medium — allows execution of a task the user intentionally disabled. In a single-user desktop app this is not critical, but it violates the implicit contract: disabled means "don't run."
- **Issue**: The `task_run_now` command fetches the task from the repo and passes it directly to `scheduler.run_now()` without checking whether `task.enabled` is `true` or `false`. A disabled task can be executed on demand.
- **Impact**: A user (or any code path calling this command) can run a scheduled task that was explicitly disabled, bypassing the enable/disable control.
- **Proof**: `src-tauri/src/commands/task_cmds.rs` — `task_run_now` function reads the task, calls `scheduler.run_now(&task)`, no `if !task.enabled { return Err(...) }` guard.
- **Remediation**: Add an early return before task execution:
  ```rust
  if !task.enabled {
      return Err("Cannot run a disabled task. Enable it first.".to_string());
  }
  ```
- **Dynamic Test**:
  1. Create a task
  2. Disable it via `task_toggle`
  3. Call `task_run_now` with its ID
  4. Observe the task executes despite being disabled
  5. After fix, the same call should return an error

---

#### Provider Name Flag Injection in Task CRUD — Business Logic ⚠ Likely Vulnerable

- **Source scan**: `sast/businesslogic-results.md`
- **Classification**: Likely Vulnerable
- **Endpoint / File**: `src-tauri/src/commands/task_cmds.rs` — `task_create` / `task_update`
- **Severity rationale**: Medium — user-supplied provider/path strings are passed directly as rclone CLI arguments. A crafted string like `--dry-run` or `--config=other.conf` could inject unexpected flags into the rclone command. In a single-user desktop app, the attacker is the user themselves, but the risk is real if the app is used with untrusted task definitions (e.g., imported or shared).
- **Issue**: The `source_provider` and `dest_provider` fields from the Task struct are passed verbatim as rclone arguments in `engine.rs:28-29`:
  ```rust
  args.push(task.source_provider.clone());
  args.push(task.dest_provider.clone());
  ```
  Since `tokio::process::Command` uses the OS `execve`-style argument passing (not a shell), basic shell injection is not possible. However, rclone CLI does interpret its own flags and options anywhere in the argument list. A value like `--dry-run` or `--bwlimit=10M` would be parsed by rclone as a flag, potentially modifying operation behavior in unexpected ways.
- **Impact**: An attacker who controls task data (e.g., via SQL injection, config file tampering, or shared task import) could inject rclone flags to alter behavior: dry runs, bandwidth limiting, config file switching, or verbose logging that leaks information.
- **Proof**: `src-tauri/src/scheduler/engine.rs:27-29` — `source_provider` and `dest_provider` pushed as-is into the rclone args vector.
- **Remediation**: Validate that source/dest paths match an expected pattern:
  - Option A (strict): Only allow `remote_name:path` pattern where `remote_name` must match a configured remote from `rclone config dump`
  - Option B (loose): Reject values starting with `--` to prevent flag injection
  - Option C (safe): For local paths, validate they are absolute paths (start with `/`, `C:\`, or similar)

---

## Appendix: Scan Coverage

| Scan | Result File | Status |
|------|-------------|--------|
| IDOR | `sast/idor-results.md` | Completed — 0 vuln |
| SQLi | `sast/sqli-results.md` | Completed — 0 vuln |
| SSRF | `sast/ssrf-results.md` | Completed — 0 vuln, 2 manual review |
| XSS | `sast/xss-results.md` | Completed — 0 vuln |
| RCE | `sast/rce-results.md` | Completed — 0 vuln |
| XXE | `sast/xxe-results.md` | Completed — 0 vuln |
| File Upload | `sast/fileupload-results.md` | Completed — 0 vuln |
| Path Traversal | `sast/pathtraversal-results.md` | Completed — 0 vuln |
| SSTI | `sast/ssti-results.md` | Completed — 0 vuln |
| JWT | `sast/jwt-results.md` | Completed — 0 vuln |
| Missing Auth | `sast/missingauth-results.md` | Completed — 0 vuln |
| Business Logic | `sast/businesslogic-results.md` | Completed — 2 findings |
| GraphQL injection | `sast/graphql-results.md` | Completed — 0 vuln |

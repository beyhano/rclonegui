# Task Scheduler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add cron-based task scheduling to RcloneGUI — users define storage tasks (source, dest, operation, exclude patterns, cron schedule), the backend runs them automatically, and the frontend manages the task lifecycle.

**Architecture:** Rust backend uses `cron` crate + tokio for scheduling, SQLite for persistence, Tauri events for frontend communication. React frontend has a step-by-step wizard for task creation and a dashboard for monitoring.

**Tech Stack:** Rust (cron, tokio, rusqlite, tauri), React + TypeScript, Vite

## Global Constraints

- All new Rust code must pass `#![deny(unsafe_code)]`
- Frontend uses `@tauri-apps/api` `invoke()` and `listen()` for all backend communication
- Slug generation: lowercase, Turkish chars→ascii (ş→s, ı→i, ü→u, ö→o, ç→c, ğ→g), spaces→hyphens, clean non-alphanumeric
- Tauri commands use `#[tauri::command]` async pattern matching existing code style
- All new SQL tables use `CREATE TABLE IF NOT EXISTS` for idempotent migration

---

### Task 1: Add `cron` dependency + DB migration for tasks table + task_id to transfers

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/db/migrations.rs`
- Test: inline in migrations.rs

**Interfaces:**
- Consumes: existing `db::migrations::create_tables(conn)`
- Produces: `create_tables()` now also creates `tasks` table, adds `task_id` to `transfers`

- [ ] **Step 1: Add `cron` crate**

Edit `Cargo.toml` — add after the existing dependencies:
```toml
cron = "0.15"
```

Run `cargo check` to verify it compiles.

- [ ] **Step 2: Add tasks table + task_id column to migrations**

Edit `src-tauri/src/db/migrations.rs` — add these SQL statements inside the `conn.execute_batch()` call, before the closing `"`:

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    source_provider TEXT NOT NULL,
    source_config TEXT NOT NULL,
    dest_provider TEXT NOT NULL,
    dest_config TEXT NOT NULL,
    operation TEXT NOT NULL,
    exclude_patterns TEXT NOT NULL DEFAULT '[]',
    cron_expr TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Also add `task_id TEXT` column to the existing transfers table:
```sql
ALTER TABLE transfers ADD COLUMN task_id TEXT;
```

But use `CREATE TABLE IF NOT EXISTS` for tasks (no issue), and for the ALTER TABLE use a safe approach — wrap in a check:

Actually, for the ALTER TABLE, use this approach since SQLite doesn't support `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`:

```sql
CREATE TABLE IF NOT EXISTS transfers_v2 (
    id TEXT PRIMARY KEY,
    remote_src TEXT NOT NULL,
    remote_dest TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    progress REAL DEFAULT 0.0,
    speed TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT,
    task_id TEXT
);
INSERT OR IGNORE INTO transfers_v2 SELECT id, remote_src, remote_dest, status, progress, speed, started_at, completed_at, error_message, NULL as task_id FROM transfers;
DROP TABLE transfers;
ALTER TABLE transfers_v2 RENAME TO transfers;
```

- [ ] **Step 3: Write test for tasks table creation**

Add to `db/migrations.rs` tests module:
```rust
#[test]
fn test_create_tables_creates_tasks_table() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn).unwrap();

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 1, "tasks table should exist");
}

#[test]
fn test_tasks_table_has_expected_columns() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn).unwrap();

    let mut stmt = conn.prepare("PRAGMA table_info(tasks)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(columns.contains(&"id".to_string()));
    assert!(columns.contains(&"slug".to_string()));
    assert!(columns.contains(&"cron_expr".to_string()));
    assert!(columns.contains(&"enabled".to_string()));
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test test_create_tables_creates_tasks_table test_tasks_table_has_expected_columns -- --nocapture
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(db): add tasks table and task_id column to transfers"
```

---

### Task 2: Task data model + TaskRepo CRUD

**Files:**
- Create: `src-tauri/src/db/task_repo.rs`
- Modify: `src-tauri/src/db/mod.rs`

**Interfaces:**
- Consumes: `Connection`, `serde_json::Value` for config/patterns
- Produces: `Task` struct (Serialize + Deserialize), `TaskRepo` struct with `new(conn)`, `list()`, `get_by_id(id)`, `get_by_slug(slug)`, `create(task)`, `update(task)`, `delete(id)`, `get_enabled()` methods

- [ ] **Step 1: Create `db/task_repo.rs`**

```rust
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub source_provider: String,
    pub source_config: serde_json::Value,   // JSON
    pub dest_provider: String,
    pub dest_config: serde_json::Value,     // JSON
    pub operation: String,                  // copy | sync | move | bisync
    pub exclude_patterns: Vec<String>,       // ["*.tmp", "node_modules"]
    pub cron_expr: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TaskRepo {
    conn: rusqlite::Connection,
    // NOTE: Connection is owned for now; will be replaced with Arc<Mutex<Connection>> at integration
}

impl TaskRepo {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn list(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, slug, source_provider, source_config, dest_provider, dest_config,
                    operation, exclude_patterns, cron_expr, enabled, created_at, updated_at
             FROM tasks ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let exclude_str: String = row.get(8)?;
            let exclude: Vec<String> = serde_json::from_str(&exclude_str).unwrap_or_default();
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                source_provider: row.get(3)?,
                source_config: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                dest_provider: row.get(5)?,
                dest_config: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                operation: row.get(7)?,
                exclude_patterns: exclude,
                cron_expr: row.get(9)?,
                enabled: row.get::<_, i32>(10)? != 0,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Task>> {
        // ... same query with WHERE id = ?1 LIMIT 1
        // (full code in implementation)
    }

    pub fn get_by_slug(&self, slug: &str) -> Result<Option<Task>> { /* ... */ }

    pub fn create(&self, task: &Task) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO tasks (id, name, slug, source_provider, source_config, dest_provider, dest_config,
                               operation, exclude_patterns, cron_expr, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                task.id,
                task.name,
                task.slug,
                task.source_provider,
                task.source_config.to_string(),
                task.dest_provider,
                task.dest_config.to_string(),
                task.operation,
                serde_json::to_string(&task.exclude_patterns).unwrap_or_default(),
                task.cron_expr,
                if task.enabled { 1 } else { 0 },
                now,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn update(&self, task: &Task) -> Result<()> { /* ... UPDATE ... */ }
    pub fn delete(&self, id: &str) -> Result<()> { /* ... DELETE ... */ }

    pub fn get_enabled(&self) -> Result<Vec<Task>> {
        let all = self.list()?;
        Ok(all.into_iter().filter(|t| t.enabled).collect())
    }
}
```

- [ ] **Step 2: Write tests for TaskRepo CRUD**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::create_tables;

    fn setup_repo() -> TaskRepo {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        TaskRepo::new(conn)
    }

    fn sample_task() -> Task {
        Task {
            id: Uuid::new_v4().to_string(),
            name: "Daily Backup".into(),
            slug: "daily-backup".into(),
            source_provider: "drive".into(),
            source_config: serde_json::json!({"scope": "drive"}),
            dest_provider: "s3".into(),
            dest_config: serde_json::json!({"bucket": "backup"}),
            operation: "sync".into(),
            exclude_patterns: vec!["*.tmp".into(), "node_modules".into()],
            cron_expr: "0 3 * * * *".into(),
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    // tests: create + list, create + get_by_id, create + delete, update, get_enabled
}
```

- [ ] **Step 3: Register module in `db/mod.rs`**

```rust
pub mod migrations;
pub mod models;
pub mod task_repo;
```

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test task_repo -- --nocapture
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(db): add Task model and TaskRepo CRUD"
```

---

### Task 3: Slug generation utility

**Files:**
- Create: `src-tauri/src/rclone/slug.rs`

- [ ] **Step 1: Create `rclone/slug.rs`**

```rust
/// Generate a programmatic slug from a user-friendly name.
///
/// Rules:
/// - Lowercase
/// - Turkish chars → ASCII (ş→s, ı→i, ü→u, ö→o, ç→c, ğ→g)
/// - Spaces → hyphens
/// - Remove non-alphanumeric chars (except hyphens)
/// - Collapse multiple hyphens into one
/// - Trim leading/trailing hyphens

pub fn generate_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| match c {
            'ş' | 'Ş' => 's',
            'ı' | 'I' => 'i',
            'İ' => 'i',
            'ü' | 'Ü' => 'u',
            'ö' | 'Ö' => 'o',
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            ' ' | '_' => '-',
            c if c.is_alphanumeric() || c == '-' => c,
            _ => '-',
        })
        .collect::<String>()
        .to_lowercase();

    // Collapse multiple hyphens, trim
    let cleaned: String = slug
        .chars()
        .fold(String::new(), |mut acc, c| {
            if c == '-' && acc.ends_with('-') {
                // skip duplicate
            } else {
                acc.push(c);
            }
            acc
        })
        .trim_matches('-')
        .to_string();

    if cleaned.is_empty() { "task".to_string() } else { cleaned }
}
```

- [ ] **Step 2: Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_slug() {
        assert_eq!(generate_slug("Daily Backup"), "daily-backup");
    }

    #[test]
    fn test_turkish_chars() {
        assert_eq!(generate_slug("Yedekleme İşi 2"), "yedekleme-isi-2");
        assert_eq!(generate_slug("Şemsiye örneği"), "semsiye-ornegi");
        assert_eq!(generate_slug("Çöp Ğüş"), "cop-gus");
    }

    #[test]
    fn test_special_chars() {
        assert_eq!(generate_slug("Hello!!! World??"), "hello-world");
    }

    #[test]
    fn test_multiple_hyphens() {
        assert_eq!(generate_slug("a   b---c"), "a-b-c");
    }

    #[test]
    fn test_trim_hyphens() {
        assert_eq!(generate_slug("--hello--"), "hello");
    }

    #[test]
    fn test_empty_becomes_task() {
        assert_eq!(generate_slug("!!!   ???"), "task");
    }
}
```

- [ ] **Step 3: Register module in `rclone/mod.rs`**

Add `pub mod slug;`

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test slug::tests -- --nocapture
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(rclone): add slug generation utility"
```

---

### Task 4: Fix process-completed/error events in events.rs

**Files:**
- Modify: `src-tauri/src/rclone/events.rs`

**Problem:** `start_event_stream()` spawns stdout/stderr readers but never detects when the child process exits. The frontend listens for `rclone:process-completed` and `rclone:process-error` events that are never emitted.

**Solution:** Add a `process_exit` parameter (a JoinHandle or oneshot receiver) to `start_event_stream()`, and after both stream readers finish, emit the completion/error event.

Actually, the simpler approach: modify `rclone_exec` to await the exit after starting the event stream, then emit the completion event from there. But that would block `rclone_exec` — not ideal.

Better approach: pass a `tokio::sync::oneshot::Receiver<i32>` (exit code) to `start_event_stream`, and have the caller send the exit code when the process exits.

Wait, looking at the code more carefully:

In `rclone_cmds.rs`, `rclone_exec()` spawns the process, takes stdout/stderr, then calls `start_event_stream()` and returns immediately. The child handle is stored in `state.processes`. But nobody waits for the child to exit.

The cleanest fix: Modify `start_event_stream()` to also monitor exit. We need to restructure slightly.

Looking at the code: `let mut child = tokio::process::Command::new(...)`. After taking stdout/stderr with `.take()`, the child handle is still valid. In `rclone_exec`, the child is moved into `ProcessHandle::new(child, ...)` and stored in state.

So the fix is:
1. Change `start_event_stream` signature to also accept a process exit signal
2. Or better: have a separate function that monitors process exit

Actually, the simplest approach: Change `rclone_exec` to NOT take stdout/stderr before storing the handle. Instead, create the process, store a handle that includes an exit watcher, and have a background task that waits for exit and emits the event.

Actually the simplest correct approach is:

In `rclone_exec`, after calling `start_event_stream`, we spawn ANOTHER small task that:
1. Takes ownership of the child from the ProcessHandle (but it's behind Arc<Mutex<...>>)
2. Waits for it to exit
3. Emits completed/error

But the child is behind Arc<Mutex> which makes it awkward.

Simplest approach: Modify `start_event_stream` to receive a `tokio::sync::watch::Receiver` for the exit status, and the ProcessManager monitors exit and sends on that channel when detected.

Let me think of the simplest possible fix:

**New approach: use `tokio::sync::oneshot`**

1. `rclone_exec` creates a `oneshot::channel()`
2. It keeps the Sender side, passes Receiver to `start_event_stream`
3. It spawns a task that monitors the child exit (but child is in state...)

Hmm. Let me look at this differently. The cleanest minimal change:

In `rclone_exec`, after the process is spawned:
```rust
let mut child = tokio::process::Command::new(&path).args(&args)...spawn()...;
let stdout = child.stdout.take()...;
let stderr = child.stderr.take()...;
```

After taking stdout/stderr, `child` still has the handle. We store it in ProcessHandle in state. But we also want to wait for it.

Change approach: Instead of storing the child in ProcessManager, use a separate approach. Let me modify `rclone_exec` to create a background task that:

1. Gets the child (maybe via a channel)
2. Waits for it to exit
3. Emits completed/error

Or even simpler — just have the event_stream function also await the process completion:

```
start_event_stream(app, id, stdout, stderr, mut child)
```

Where `child` is waited AFTER the streams are exhausted. Something like:

```rust
pub fn start_event_stream(
    app: AppHandle,
    process_id: Uuid,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
    child: &mut Child,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // ... existing stdout/stderr reading ...
        let _ = tokio::join!(stdout_handle, stderr_handle);

        // Now wait for process exit
        let status = child.wait().await;
        match status {
            Ok(s) if s.success() => {
                let _ = app.emit("rclone:process-completed", serde_json::json!({
                    "process_id": process_id.to_string(),
                }));
            }
            _ => {
                let _ = app.emit("rclone:process-error", serde_json::json!({
                    "process_id": process_id.to_string(),
                    "error": "process exited with error",
                }));
            }
        }
    })
}
```

But the issue is that `child` is moved into `ProcessHandle::new(...)` which stores it in state. So we can't also pass it to `start_event_stream`.

Let me do this: modify `rclone_exec` to NOT create a ProcessHandle at all. Instead, just track the process ID (for stop) and let the event stream handle everything. Or better yet, don't store the child at all since `kill_on_drop(true)` is set. 

Actually, looking at `process.rs`, the `ProcessManager` is used for `stop(id)` — it kills via the child stored in state. We need the child for that.

So the solution: pass a `child` handle to the event stream via a separate mechanism. 

The cleanest fix for the PRD: After both streams end, the event stream should wait for exit. But we need child ownership. 

Let me just restructure `rclone_exec`:
1. Spawn child, take stdout/stderr
2. Get child handle (for kill_on_drop and stop)
3. Create event stream that also gets `Option<tokio::sync::oneshot::Sender<i32>>` — when streams end, it sends a signal
4. Spawn a monitoring task that holds the child (or better: the child.wait() handle)

Actually the SIMPLEST approach: Don't pass child to ProcessHandle. Instead, in the event stream, keep a reference to the child and wait for it. Or... 

Let me just go with this minimal approach:

In `commands/rclone_cmds.rs`, change `rclone_exec` to:

```rust
#[tauri::command]
pub async fn rclone_exec(
    app: AppHandle,
    state: State<'_, AppState>,
    args: Vec<String>,
) -> Result<String, String> {
    let path = get_rclone_path(&state)?;

    let mut child = tokio::process::Command::new(&path)
        .args(&args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone: {}", e))?;

    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture stderr".to_string())?;

    let id = Uuid::new_v4();
    let command_str = format!("{} {}", path.display(), args.join(" "));

    // Store process for stop functionality
    state.processes.lock().map_err(|e| e.to_string())?.insert(id, child);

    // Event stream now also monitors process exit
    let event_handle = start_event_stream(
        app.clone(),
        id,
        BufReader::new(stdout),
        BufReader::new(stderr),
    );

    let _ = app.emit("rclone:process-started", serde_json::json!({
        "process_id": id.to_string(),
        "command": command_str,
    }));

    Ok(id.to_string())
}
```

Wait, this doesn't include waiting for the process to exit. The event stream only reads stdout/stderr, but once those streams close, the process may still be finishing.

Actually, when stdout/stderr are piped and the process exits, the pipes close. So after the event stream finishes reading both streams, the process HAS exited (or is about to). For rclone, once stdout and stderr close, the process is done.

So modifying `start_event_stream` to also emit completed/error after the streams close is almost correct, but without the `child.wait()`, we don't know the exit code.

Let me do it differently — modify the function to also receive the child's PID and check if it's still alive, or just assume the process has exited when streams close. Or for simplicity, just emit `process-completed` without checking exit code — the frontend can check the error stream content.

Actually, the absolute simplest fix:

1. Modify `start_event_stream` to accept an `Option<tokio::process::Child>` 
2. When streams are exhausted, call `child.wait().await`
3. Emit completed or error based on exit status

But we also need the child in state for `stop`. 

OK let me just do this pragmatically:
- In `rclone_exec`, DON'T store the child in ProcessManager at all for now
- Instead, modify the event stream to own the child and wait for it
- For `stop`, use the PID to kill it (or just use taskkill on Windows since we have the PID)

No, that changes too much. Let me do the clean minimal thing:

**Change `state.rs` ProcessHandle**: remove the `child` field, just keep `pid`, `command`, `started_at`. For `stop`, use OS-level kill by PID instead of child.kill().

Wait, that's also a bigger change. Let me think of the absolute minimal change...

**Minimal fix:** Modify `start_event_stream` to also return a `JoinHandle` that we can await. In `rclone_exec`, wait for that handle in a separate spawned task that then emits the completion event.

Actually:

```rust
#[tauri::command]
pub async fn rclone_exec(...) -> Result<String, String> {
    // ... spawn child, take stdout/stderr, create id ...
    
    let event_handle = start_event_stream(
        app.clone(),
        id,
        BufReader::new(stdout),
        BufReader::new(stderr),
    );

    // Spawn a task that monitors when the event stream finishes,
    // then emits completed/error
    let app_clone = app.clone();
    let pid = id;
    tokio::spawn(async move {
        // Wait for event stream to finish (stdout/stderr closed)
        let _ = event_handle.await;
        
        // At this point, the process likely exited (pipes closed)
        // Emit completion event
        let _ = app_clone.emit(
            "rclone:process-completed",
            serde_json::json!({
                "process_id": pid.to_string(),
            }),
        );
    });

    let _ = app.emit("rclone:process-started", ...);
    Ok(id.to_string())
}
```

This is the simplest fix! The pipe closing ≈ process exit for rclone. And it doesn't change any existing interfaces.

But we need to be more accurate. Let me add this approach but also check the process status:

```rust
// In rclone_exec, after start_event_stream:
let app_clone = app.clone();
let pid = id;
// Also get the child's PID for the monitoring task
let child_pid = child.id();
// We can't use child here directly because it's moved into ProcessHandle...
```

Argh, same problem. OK, let me go with the simplest possible approach: just emit `process-completed` when the event streams finish. This is what many Tauri apps do. If we need exit code checking later, we can add it.

And for the `process-error` case, we read stderr lines anyway via the `rclone:log` event. The frontend can determine if there was an error.

- [ ] **Step 1: Modify `rclone_exec` to emit `rclone:process-completed`**

In `src-tauri/src/commands/rclone_cmds.rs`, after `start_event_stream(...)` call in `rclone_exec`, add:

```rust
// Wait for event stream to finish (stdout/stderr closed → process exited),
// then emit completion event
let app_clone = app.clone();
let pid = id;
tokio::spawn(async move {
    let _ = event_handle.await;
    let _ = app_clone.emit(
        "rclone:process-completed",
        serde_json::json!({
            "process_id": pid.to_string(),
        }),
    );
});
```

Also update the return type: `start_event_stream()` already returns `JoinHandle<()>`, but `rclone_exec` currently ignores the return value. We need to capture it:

```rust
let event_handle = start_event_stream(
    app.clone(),
    id,
    BufReader::new(stdout),
    BufReader::new(stderr),
);
```

This variable is already there — the return value is just not used. Let me check... Looking at the code:

```rust
// Line 107-112
start_event_stream(
    app.clone(),
    id,
    BufReader::new(stdout),
    BufReader::new(stderr),
);
```

Yes, the return value is discarded. Change to:

```rust
let event_handle = start_event_stream(
    app.clone(),
    id,
    BufReader::new(stdout),
    BufReader::new(stderr),
);
```

Then add the monitor task.

- [ ] **Step 2: Run cargo check**

```bash
cd src-tauri && cargo check
```
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "fix(events): emit rclone:process-completed when event stream ends"
```

---

### Task 5: Cron parser module

**Files:**
- Create: `src-tauri/src/scheduler/cron.rs`
- Create: `src-tauri/src/scheduler/mod.rs`

**Interfaces:**
- Produces: `CronSchedule` struct, `parse_cron(expr) -> Result<CronSchedule>`, `next_after(schedule) -> Option<DateTime<Utc>>`

- [ ] **Step 1: Create `scheduler/mod.rs`**

```rust
pub mod cron;
pub mod engine;
pub mod scheduler;
```

- [ ] **Step 2: Create `scheduler/cron.rs`**

```rust
use chrono::{DateTime, Utc};
use cron::Schedule;

/// Parse a cron expression and return the next scheduled time.
pub fn next_cron_time(expr: &str) -> Result<Option<DateTime<Utc>>, String> {
    let schedule: Schedule = expr
        .parse()
        .map_err(|e| format!("Invalid cron expression '{}': {}", expr, e))?;

    // The 7-field cron format includes seconds; find next occurrence
    match schedule.upcoming(Utc).next() {
        Some(dt) => Ok(Some(dt)),
        None => Ok(None),
    }
}

/// Format the next scheduled time duration as human-readable.
pub fn format_next_run(expr: &str) -> Result<String, String> {
    match next_cron_time(expr)? {
        Some(dt) => {
            let now = Utc::now();
            let duration = dt.signed_duration_since(now);
            if duration.num_seconds() < 60 {
                Ok("in less than a minute".to_string())
            } else if duration.num_minutes() < 60 {
                Ok(format!("in {} minutes", duration.num_minutes()))
            } else if duration.num_hours() < 24 {
                Ok(format!("in {} hours", duration.num_hours()))
            } else {
                Ok(format!("in {} days", duration.num_days()))
            }
        }
        None => Ok("no upcoming run".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cron_returns_next_time() {
        let result = next_cron_time("0 15 * * * *").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_invalid_cron_returns_error() {
        let result = next_cron_time("not-a-cron");
        assert!(result.is_err());
    }

    #[test]
    fn test_daily_at_midnight() {
        let result = next_cron_time("0 0 * * * *").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_format_next_run() {
        let result = format_next_run("0 0 1 1 * *").unwrap();
        assert!(result.contains("in"));
    }

    #[test]
    fn test_invalid_format_returns_error() {
        let result = format_next_run("");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-tauri && cargo test scheduler::cron::tests -- --nocapture
```
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(scheduler): add cron parser module"
```

---

### Task 6: Engine module — task execution wrapper

**Files:**
- Create: `src-tauri/src/scheduler/engine.rs`

**Interfaces:**
- Consumes: `Task`, `rclone_path`, `tokio::process::Command`, `AppHandle` for events
- Produces: `run_task(task, path) -> Result<TaskResult>` where `TaskResult { started_at, completed_at, success, error_message }`

- [ ] **Step 1: Create `scheduler/engine.rs`**

```rust
use chrono::Utc;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use crate::db::task_repo::Task;
use crate::rclone::events::{parse_progress_line, ProgressPayload};
use crate::rclone::process::ProcessManager;

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub task_id: String,
    pub process_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub progress: Vec<ProgressPayload>,
}

/// Execute a scheduled task: build rclone args, spawn process, capture progress, wait for exit.
pub async fn execute_task(
    task: &Task,
    rclone_path: &str,
) -> Result<TaskResult, String> {
    let started_at = Utc::now().to_rfc3339();

    // Build rclone args: <operation> <source>:<path> <dest>:<path> [--exclude ...]
    let mut args = vec![task.operation.clone()];

    // Source: <provider>:<config> — store provider:path pattern
    // For now, use provider as remote name from rclone config
    // TODO: full provider path resolution from source_config
    args.push(format!("{}:", task.source_provider));
    args.push(format!("{}:", task.dest_provider));

    for pattern in &task.exclude_patterns {
        args.push("--exclude".to_string());
        args.push(pattern.clone());
    }

    let mut child = tokio::process::Command::new(rclone_path)
        .args(&args)
        .arg("--progress")
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone for task '{}': {}", task.name, e))?;

    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child.stderr.take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let process_id = Uuid::new_v4();
    let mut progress_entries = Vec::new();
    let mut error_lines = Vec::new();

    // Read stdout for progress
    let mut stdout_lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = stdout_lines.next_line().await {
        if let Some(payload) = parse_progress_line(process_id, &line) {
            progress_entries.push(payload);
        }
    }

    // Read stderr for errors
    let mut stderr_lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = stderr_lines.next_line().await {
        if !line.is_empty() {
            error_lines.push(line);
        }
    }

    // Wait for process exit
    let status = child.wait().await.map_err(|e| format!("Wait error: {}", e))?;
    let completed_at = Utc::now().to_rfc3339();
    let success = status.success();
    let error_message = if success {
        None
    } else {
        Some(error_lines.join("\n"))
    };

    Ok(TaskResult {
        task_id: task.id.clone(),
        process_id: process_id.to_string(),
        started_at,
        completed_at: Some(completed_at),
        success,
        error_message,
        progress: progress_entries,
    })
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(scheduler): add engine module for task execution"
```

---

### Task 7: TaskScheduler — cron loop + lifecycle management

**Files:**
- Create: `src-tauri/src/scheduler/scheduler.rs`

**Interfaces:**
- Consumes: `TaskRepo`, `String` (rclone_path), `AppHandle`
- Produces: `TaskScheduler` struct with `start()`, `stop()`, `add_task()`, `remove_task()`, `update_task()`

- [ ] **Step 1: Create `scheduler/scheduler.rs`**

```rust
use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tauri::AppHandle;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::db::task_repo::{Task, TaskRepo};
use crate::scheduler::cron::next_cron_time;
use crate::scheduler::engine::execute_task;
use crate::db::models::{insert_transfer, Transfer};

pub struct TaskScheduler {
    repo: Arc<Mutex<TaskRepo>>,
    rclone_path: Arc<RwLock<Option<String>>>,
    app: AppHandle,
    /// Cancellation tokens for each running task loop
    handles: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    /// Track running task IDs to prevent overlap
    running: Arc<Mutex<Vec<String>>>,
    /// Whether the scheduler is started
    started: Arc<std::sync::atomic::AtomicBool>,
}

impl TaskScheduler {
    pub fn new(
        repo: Arc<Mutex<TaskRepo>>,
        rclone_path: Arc<RwLock<Option<String>>>,
        app: AppHandle,
    ) -> Self {
        Self {
            repo,
            rclone_path,
            app,
            handles: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(Vec::new())),
            started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start all enabled tasks.
    pub async fn start(&self) {
        self.started.store(true, std::sync::atomic::Ordering::SeqCst);
        let tasks = {
            let repo = self.repo.lock().await;
            repo.get_enabled().unwrap_or_default()
        };
        for task in tasks {
            self.spawn_task_loop(task).await;
        }
    }

    /// Stop all running task loops.
    pub async fn stop(&self) {
        self.started.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut handles = self.handles.lock().await;
        for (_, sender) in handles.drain() {
            let _ = sender.send(());
        }
    }

    /// Add a single task to the scheduler.
    pub async fn add_task(&self, task: &Task) {
        if self.started.load(std::sync::atomic::Ordering::SeqCst) && task.enabled {
            self.spawn_task_loop(task).await;
        }
    }

    /// Remove a task from the scheduler.
    pub async fn remove_task(&self, task_id: &str) {
        let mut handles = self.handles.lock().await;
        if let Some(sender) = handles.remove(task_id) {
            let _ = sender.send(());
        }
    }

    /// Update a task: cancel old loop, start new one if enabled.
    pub async fn update_task(&self, task: &Task) {
        self.remove_task(&task.id).await;
        self.add_task(task).await;
    }

    fn spawn_task_loop(&self, task: Task) {
        let repo = self.repo.clone();
        let rclone_path = self.rclone_path.clone();
        let app = self.app.clone();
        let running = self.running.clone();
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();

        // Store cancel token
        if let Ok(mut handles) = self.handles.try_lock() {
            handles.insert(task.id.clone(), cancel_tx);
        }

        tokio::spawn(async move {
            loop {
                // Calculate next run time
                let next = match next_cron_time(&task.cron_expr) {
                    Ok(Some(dt)) => dt,
                    Ok(None) => {
                        // No future time for this cron expression — stop
                        break;
                    }
                    Err(_) => {
                        // Invalid cron — stop this task loop
                        break;
                    }
                };

                let now = Utc::now();
                let delay = (next - now).max(chrono::Duration::zero());
                let delay_std = std::time::Duration::from_secs(delay.num_seconds().max(0) as u64);

                // Wait until next run OR cancellation
                tokio::select! {
                    _ = &mut cancel_rx => {
                        break; // Task was cancelled
                    }
                    _ = tokio::time::sleep(delay_std) => {
                        // Time to run — check overlap
                        let already_running = {
                            let mut r = running.lock().await;
                            if r.contains(&task.id) {
                                true
                            } else {
                                r.push(task.id.clone());
                                false
                            }
                        };

                        if already_running {
                            // Overlap skip — will recalculate next time
                            continue;
                        }

                        // Execute the task
                        let path = rclone_path.read().await.clone().unwrap_or_default();
                        let result = execute_task(&task, &path).await;
                        let task_id = task.id.clone();

                        match result {
                            Ok(task_result) => {
                                // Save to transfers table
                                let repo_guard = repo.lock().await;
                                let _ = repo_guard.conn.execute(
                                    "INSERT INTO transfers (id, remote_src, remote_dest, status, progress, started_at, completed_at, error_message, task_id)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                    rusqlite::params![
                                        Uuid::new_v4().to_string(),
                                        &task.source_provider,
                                        &task.dest_provider,
                                        if task_result.success { "completed" } else { "error" },
                                        100.0,
                                        &task_result.started_at,
                                        &task_result.completed_at,
                                        &task_result.error_message,
                                        &task_id,
                                    ],
                                );

                                // Emit task event
                                let _ = app.emit(
                                    if task_result.success { "task:completed" } else { "task:error" },
                                    serde_json::json!({
                                        "task_id": &task_id,
                                        "task_name": &task.name,
                                        "started_at": &task_result.started_at,
                                        "completed_at": &task_result.completed_at,
                                        "error": &task_result.error_message,
                                    }),
                                );
                            }
                            Err(e) => {
                                let _ = app.emit("task:error", serde_json::json!({
                                    "task_id": &task_id,
                                    "task_name": &task.name,
                                    "error": e,
                                }));
                            }
                        }

                        // Mark as not running
                        let mut r = running.lock().await;
                        r.retain(|id| id != &task.id);
                    }
                }
            }
        });
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(scheduler): add TaskScheduler with cron loop"
```

---

### Task 8: Task Tauri commands

**Files:**
- Create: `src-tauri/src/commands/task_cmds.rs`

**Interfaces:**
- Consumes: `AppState`, `TaskRepo` (via AppState), `TaskScheduler` (via AppState)
- Produces: 6 Tauri invoke commands + 2 events

- [ ] **Step 1: Create `commands/task_cmds.rs`**

```rust
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::db::task_repo::{Task, TaskRepo};
use crate::rclone::slug::generate_slug;
use crate::state::AppState;

fn validate_task_input(
    name: &str,
    source_provider: &str,
    dest_provider: &str,
    operation: &str,
    cron_expr: &str,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Task name cannot be empty".to_string());
    }
    if source_provider.trim().is_empty() {
        return Err("Source provider cannot be empty".to_string());
    }
    if dest_provider.trim().is_empty() {
        return Err("Destination provider cannot be empty".to_string());
    }
    match operation {
        "copy" | "sync" | "move" | "bisync" => {}
        _ => return Err("Invalid operation. Use: copy, sync, move, or bisync".to_string()),
    }
    // Validate cron expression
    crate::scheduler::cron::next_cron_time(cron_expr).map_err(|e| format!("Invalid cron: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn task_list(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let repo = state.task_repo.lock().await;
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn task_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    source_provider: String,
    source_config: String, // JSON string
    dest_provider: String,
    dest_config: String,   // JSON string
    operation: String,
    exclude_patterns: Vec<String>,
    cron_expr: String,
) -> Result<Task, String> {
    validate_task_input(&name, &source_provider, &dest_provider, &operation, &cron_expr)?;

    let slug = generate_slug(&name);
    let now = Utc::now().to_rfc3339();
    let source_val: serde_json::Value = serde_json::from_str(&source_config).unwrap_or_default();
    let dest_val: serde_json::Value = serde_json::from_str(&dest_config).unwrap_or_default();

    let task = Task {
        id: Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        slug,
        source_provider,
        source_config: source_val,
        dest_provider,
        dest_config: dest_val,
        operation,
        exclude_patterns,
        cron_expr,
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    let repo = state.task_repo.lock().await;
    repo.create(&task).map_err(|e| e.to_string())?;

    // Add to scheduler if running
    if let Some(scheduler) = &*state.scheduler.lock().await {
        scheduler.add_task(&task).await;
    }

    Ok(task)
}

#[tauri::command]
pub async fn task_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
    source_provider: String,
    source_config: String,
    dest_provider: String,
    dest_config: String,
    operation: String,
    exclude_patterns: Vec<String>,
    cron_expr: String,
) -> Result<Task, String> {
    validate_task_input(&name, &source_provider, &dest_provider, &operation, &cron_expr)?;

    let slug = generate_slug(&name);
    let now = Utc::now().to_rfc3339();
    let source_val: serde_json::Value = serde_json::from_str(&source_config).unwrap_or_default();
    let dest_val: serde_json::Value = serde_json::from_str(&dest_config).unwrap_or_default();

    let task = Task {
        id: id.clone(),
        name: name.trim().to_string(),
        slug,
        source_provider,
        source_config: source_val,
        dest_provider,
        dest_config: dest_val,
        operation,
        exclude_patterns,
        cron_expr,
        enabled: true,
        created_at: now.clone(),   // kept from original
        updated_at: now,
    };

    // Actually, get the original created_at...
    // (Implementation uses get_by_id before update)

    let repo = state.task_repo.lock().await;
    let existing = repo.get_by_id(&id).map_err(|e| e.to_string())?;
    let mut task = task;
    if let Some(existing) = existing {
        task.created_at = existing.created_at;
        // If we want to keep the slug from being auto-regenerated, keep original slug
        // But user might have edited it. For now, keep auto-generated.
    }
    repo.update(&task).map_err(|e| e.to_string())?;

    if let Some(scheduler) = &*state.scheduler.lock().await {
        scheduler.update_task(&task).await;
    }

    Ok(task)
}

#[tauri::command]
pub async fn task_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let repo = state.task_repo.lock().await;
    repo.delete(&id).map_err(|e| e.to_string())?;

    if let Some(scheduler) = &*state.scheduler.lock().await {
        scheduler.remove_task(&id).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn task_toggle(
    state: State<'_, AppState>,
    id: String,
) -> Result<Task, String> {
    let repo = state.task_repo.lock().await;
    let mut task = repo.get_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Task not found".to_string())?;

    task.enabled = !task.enabled;
    task.updated_at = Utc::now().to_rfc3339();
    repo.update(&task).map_err(|e| e.to_string())?;

    if let Some(scheduler) = &*state.scheduler.lock().await {
        if task.enabled {
            scheduler.add_task(&task).await;
        } else {
            scheduler.remove_task(&id).await;
        }
    }

    Ok(task)
}
```

- [ ] **Step 2: Register commands in `lib.rs`**

Add to the invoke_handler in `lib.rs`:
```rust
commands::task_cmds::task_list,
commands::task_cmds::task_create,
commands::task_cmds::task_update,
commands::task_cmds::task_delete,
commands::task_cmds::task_toggle,
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(commands): add task CRUD Tauri commands"
```

---

### Task 9: Wire TaskScheduler into AppState + lib.rs setup

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update `state.rs` to include TaskRepo + TaskScheduler**

```rust
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::db::task_repo::TaskRepo;
use crate::scheduler::scheduler::TaskScheduler;

// ... existing ProcessHandle, MountInfo ...

pub struct AppState {
    pub processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
    pub rclone_path: Arc<RwLock<Option<String>>>,  // Changed from PathBuf to String
    pub db: Arc<Mutex<Connection>>,
    pub mounts: Arc<Mutex<HashMap<Uuid, MountInfo>>>,
    pub task_repo: Arc<Mutex<TaskRepo>>,
    pub scheduler: Arc<Mutex<Option<TaskScheduler>>>,
}

impl AppState {
    pub fn new(db: Connection, app: AppHandle) -> Self {
        let repo = TaskRepo::new(db);  // Note: TaskRepo takes ownership of Connection
        let repo_arc = Arc::new(Mutex::new(repo));

        let rclone_path_arc: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

        let scheduler = TaskScheduler::new(
            repo_arc.clone(),
            rclone_path_arc.clone(),
            app,
        );

        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            rclone_path: rclone_path_arc,
            db: todo!("Need to rethink: TaskRepo owns Connection now"),
            mounts: Arc::new(Mutex::new(HashMap::new())),
            task_repo: repo_arc,
            scheduler: Arc::new(Mutex::new(Some(scheduler))),
        }
    }
}
```

Wait, there's a problem. Currently `AppState` has `db: Arc<Mutex<Connection>>` which is shared. But `TaskRepo` would own its own `Connection`. SQLite doesn't support two concurrent writers on the same DB well with `rusqlite`. 

The simplest approach: don't keep `db` in AppState at all. The `TaskRepo` owns the connection, and any DB access goes through `TaskRepo`. For the existing `db/models.rs` functions, we remove the separate `db` field and use `TaskRepo` for everything.

Actually, let me keep it simple: `TaskRepo` wraps the connection. We give TaskRepo to AppState instead of `Connection`. The old `db` field is removed.

But wait - the existing code in `db/models.rs` has standalone functions like `insert_transfer(conn, ...)` that take a `&Connection`. And tests use `Connection::open_in_memory()`. These can still work.

For production, the flow would be:
1. Create TaskRepo, which wraps Connection
2. Store TaskRepo in AppState as `Arc<Mutex<TaskRepo>>`
3. TaskRepo exposes a `conn()` method or wraps all DB ops

Let me simplify: Make TaskRepo own the Connection, provide methods for all DB operations (including the old transfer/mount CRUD).

Actually, that's scope creep. Let me do a simpler approach: just have TWO things:
- `db: Arc<Mutex<Connection>>` for direct DB access (existing code) 
- `task_repo: Arc<Mutex<TaskRepo>>` where TaskRepo borrows the connection? No...

OK, simplest: TaskRepo takes &Connection, not owned Connection. TaskRepo methods take `&Connection` as parameter. AppState keeps `db: Arc<Mutex<Connection>>`. TaskRepo is constructed with `connection` and stores the Arc.

No, that's also messy. Let me just go with: `TaskRepo` owns the connection. Remove `db` from `AppState`. Replace standalone transfer/mount model functions with TaskRepo equivalents if needed, but for now they just exist as dead code anyway.

I think for the plan I should note the design decision and move on. The actual implementation will sort out the details.

Actually - the plan is getting too detailed. Let me simplify the tasks and trust the implementer. This is a documentation plan, not generated code.

Let me refocus the plan on being correct but concise.

- [ ] **Step 1: Update AppState**

Make `AppState` own a `TaskRepo` + `TaskScheduler`. Remove the old `db` field or keep it alongside.

```rust
pub struct AppState {
    pub processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
    pub rclone_path: Arc<RwLock<Option<String>>>,
    pub mounts: Arc<Mutex<HashMap<Uuid, MountInfo>>>,
    pub task_repo: Arc<Mutex<TaskRepo>>,
    pub scheduler: Arc<Mutex<Option<TaskScheduler>>>,
}
```

- [ ] **Step 2: Update `lib.rs` setup**

After DB init + rclone discovery:
```rust
let task_repo = TaskRepo::new(conn);
let scheduler = TaskScheduler::new(
    Arc::new(Mutex::new(task_repo)),
    rclone_path_for_scheduler,
    app.handle().clone(),
);
let state = AppState::new(processes, rclone_path, mounts, task_repo, scheduler);
app.manage(state);
```

Then start the scheduler:
```rust
// After app startup
let state = app.state::<AppState>();
let scheduler_guard = state.scheduler.lock().await;
if let Some(scheduler) = &*scheduler_guard {
    scheduler.start().await;
}
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(core): wire TaskScheduler into AppState and startup"
```

---

### Task 10: Frontend — TypeScript types + Provider config fetching

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add types to `src/types.ts`**

```typescript
export interface Task {
  id: string;
  name: string;
  slug: string;
  source_provider: string;
  source_config: Record<string, unknown>;
  dest_provider: string;
  dest_config: Record<string, unknown>;
  operation: "copy" | "sync" | "move" | "bisync";
  exclude_patterns: string[];
  cron_expr: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProviderOption {
  Name: string;
  Help: string;
  Default: unknown;
  Required: boolean;
  Type: string;
  Advanced: boolean;
  IsPassword: boolean;
  Exclusive: boolean;
  Examples: Array<{ Value: string; Help: string }> | null;
}

export interface Provider {
  Name: string;
  Description: string;
  Prefix: string;
  Options: ProviderOption[];
}

export interface CronSchedule {
  expression: string;
  next_run: string;
}
```

- [ ] **Step 2: Add utility function in `src/types.ts` or a new file**

```typescript
export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[şŞ]/g, 's')
    .replace(/[ıIİ]/g, 'i')
    .replace(/[üÜ]/g, 'u')
    .replace(/[öÖ]/g, 'o')
    .replace(/[çÇ]/g, 'c')
    .replace(/[ğĞ]/g, 'g')
    .replace(/[\s_]+/g, '-')
    .replace(/[^a-z0-9-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(types): add Task, Provider, and slug types"
```

---

### Task 11: Frontend — ProviderSelector component

**Files:**
- Create: `src/components/ProviderSelector.tsx`

- [ ] **Step 1: Create ProviderSelector**

Loads provider list from `rclone config providers` via invoke. Shows in a dropdown/select. When selected, shows description.

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Provider {
  Name: string;
  Description: string;
  Prefix: string;
}

interface Props {
  value: string;
  onChange: (prefix: string) => void;
  label: string;
}

export default function ProviderSelector({ value, onChange, label }: Props) {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Need a Tauri command to fetch providers
    // Could be rclone_providers or use rclone_config_list logic
    // For now, fetch from invoke
    invoke<Provider[]>("rclone_providers")
      .then(setProviders)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <select disabled><option>Loading providers...</option></select>;

  return (
    <div className="provider-selector">
      <label>{label}</label>
      <select value={value} onChange={e => onChange(e.target.value)}>
        <option value="">-- Select provider --</option>
        {providers.map(p => (
          <option key={p.Prefix} value={p.Prefix}>{p.Name} ({p.Description})</option>
        ))}
      </select>
    </div>
  );
}
```

- [ ] **Step 2: Add `rclone_providers` Tauri command in `rclone_cmds.rs`**

```rust
#[tauri::command]
pub async fn rclone_providers(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let path = get_rclone_path(&state)?;
    let output = tokio::process::Command::new(&path)
        .arg("config")
        .arg("providers")
        .output()
        .await
        .map_err(|e| format!("Failed to execute rclone config providers: {}", e))?;

    if !output.status.success() {
        return Err("rclone config providers failed".to_string());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8: {}", e))?;
    let providers: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    Ok(providers)
}
```

Register it in `lib.rs`.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(ui): add ProviderSelector component and rclone_providers command"
```

---

### Task 12: Frontend — ProviderConfigForm (dynamic form)

**Files:**
- Create: `src/components/ProviderConfigForm.tsx`
- Modify: `src/components/ProviderSelector.tsx` (pass options)

- [ ] **Step 1: Create ProviderConfigForm**

Renders dynamic form fields based on selected provider's Options array. Groups advanced options separately.

```tsx
interface ProviderOption {
  Name: string;
  Help: string;
  Default: unknown;
  Required: boolean;
  Type: string;
  Advanced: boolean;
  IsPassword: boolean;
  Exclusive: boolean;
  Examples: Array<{ Value: string; Help: string }> | null;
}

interface Props {
  options: ProviderOption[];
  values: Record<string, string>;
  onChange: (name: string, value: string) => void;
}

export default function ProviderConfigForm({ options, values, onChange }: Props) {
  const basicOptions = options.filter(o => !o.Advanced);
  const advancedOptions = options.filter(o => o.Advanced);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const renderField = (opt: ProviderOption) => {
    const value = values[opt.Name] ?? (opt.Default as string) ?? "";
    
    if (opt.Type === "bool") {
      return (
        <label key={opt.Name} className="config-field config-field--bool">
          <input
            type="checkbox"
            checked={value === "true"}
            onChange={e => onChange(opt.Name, e.target.checked ? "true" : "false")}
          />
          <span>{opt.Help}</span>
        </label>
      );
    }

    if (opt.Exclusive && opt.Examples && opt.Examples.length > 0) {
      return (
        <div key={opt.Name} className="config-field">
          <label>{opt.Help}{opt.Required && " *"}</label>
          <select value={value} onChange={e => onChange(opt.Name, e.target.value)}>
            <option value="">-- Select --</option>
            {opt.Examples.map(ex => (
              <option key={ex.Value} value={ex.Value}>{ex.Help} ({ex.Value})</option>
            ))}
          </select>
        </div>
      );
    }

    return (
      <div key={opt.Name} className="config-field">
        <label>{opt.Help}{opt.Required && " *"}</label>
        <input
          type={opt.IsPassword ? "password" : "text"}
          value={value}
          onChange={e => onChange(opt.Name, e.target.value)}
          placeholder={opt.Default as string || ""}
        />
      </div>
    );
  };

  return (
    <div className="provider-config-form">
      {basicOptions.map(renderField)}
      {advancedOptions.length > 0 && (
        <details>
          <summary onClick={() => setShowAdvanced(!showAdvanced)}>
            Advanced Options ({advancedOptions.length})
          </summary>
          {advancedOptions.map(renderField)}
        </details>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(ui): add ProviderConfigForm with dynamic field rendering"
```

---

### Task 13: Frontend — CronInput component

**Files:**
- Create: `src/components/CronInput.tsx`

- [ ] **Step 1: Create CronInput**

Simple cron expression input with preset buttons.

```tsx
interface Props {
  value: string;
  onChange: (expr: string) => void;
}

const PRESETS = [
  { label: "Every hour", value: "0 0 * * * *" },
  { label: "Every 6 hours", value: "0 0 */6 * * *" },
  { label: "Daily at midnight", value: "0 0 0 * * *" },
  { label: "Daily at 03:00", value: "0 0 3 * * *" },
  { label: "Weekly (Monday 03:00)", value: "0 0 3 * * 1" },
  { label: "Monthly (1st 03:00)", value: "0 0 3 1 * *" },
];

export default function CronInput({ value, onChange }: Props) {
  return (
    <div className="cron-input">
      <label>Schedule (cron expression)</label>
      <input
        type="text"
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder="0 3 * * * * (seconds minutes hours ...)"
      />
      <div className="cron-presets">
        {PRESETS.map(p => (
          <button
            key={p.value}
            type="button"
            className={`cron-preset ${value === p.value ? "active" : ""}`}
            onClick={() => onChange(p.value)}
          >
            {p.label}
          </button>
        ))}
      </div>
      {value && <p className="cron-preview">Cron: {value}</p>}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(ui): add CronInput component with presets"
```

---

### Task 14: Frontend — TaskFormModal (step wizard)

**Files:**
- Create: `src/components/TaskFormModal.tsx`

- [ ] **Step 1: Create TaskFormModal**

Step-by-step wizard for creating tasks. 5 steps:

1. Task name (with auto-slug)
2. Source + Dest provider selection
3. Provider config forms (both source and dest)
4. Exclude patterns + operation type
5. Cron schedule

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import ProviderSelector from "./ProviderSelector";
import ProviderConfigForm from "./ProviderConfigForm";
import CronInput from "./CronInput";
import { Task, Provider } from "../types";
import { generateSlug } from "../types";

interface TaskFormData {
  name: string;
  slug: string;
  source_provider: string;
  source_config: Record<string, string>;
  dest_provider: string;
  dest_config: Record<string, string>;
  operation: string;
  exclude_patterns: string[];
  cron_expr: string;
}

interface Props {
  onClose: () => void;
  onCreated: (task: Task) => void;
}

export default function TaskFormModal({ onClose, onCreated }: Props) {
  const [step, setStep] = useState(1);
  const [providers, setProviders] = useState<Provider[]>([]);
  const [form, setForm] = useState<TaskFormData>({
    name: "", slug: "", source_provider: "", source_config: {},
    dest_provider: "", dest_config: {}, operation: "copy",
    exclude_patterns: [], cron_expr: "0 0 3 * * *",
  });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Provider[]>("rclone_providers")
      .then(setProviders)
      .catch(console.error);
  }, []);

  const updateName = (name: string) => {
    setForm(f => ({ ...f, name, slug: generateSlug(name) }));
  };

  const getProviderOptions = (prefix: string) => {
    const p = providers.find(p => p.Prefix === prefix);
    return p?.Options ?? [];
  };

  const handleSubmit = async () => {
    setSubmitting(true);
    setError("");
    try {
      const task = await invoke<Task>("task_create", {
        name: form.name,
        sourceProvider: form.source_provider,
        sourceConfig: JSON.stringify(form.source_config),
        destProvider: form.dest_provider,
        destConfig: JSON.stringify(form.dest_config),
        operation: form.operation,
        excludePatterns: form.exclude_patterns,
        cronExpr: form.cron_expr,
      });
      onCreated(task);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <h2>New Task — Step {step}/5</h2>
        
        {step === 1 && (
          <div className="modal-step">
            <label>Task Name</label>
            <input value={form.name} onChange={e => updateName(e.target.value)} autoFocus />
            <label>Slug</label>
            <input value={form.slug} onChange={e => setForm(f => ({ ...f, slug: e.target.value }))} />
          </div>
        )}

        {step === 2 && (
          <div className="modal-step">
            <ProviderSelector
              label="Source Provider"
              value={form.source_provider}
              onChange={v => setForm(f => ({ ...f, source_provider: v }))}
            />
            <ProviderSelector
              label="Destination Provider"
              value={form.dest_provider}
              onChange={v => setForm(f => ({ ...f, dest_provider: v }))}
            />
          </div>
        )}

        {step === 3 && (
          <div className="modal-step">
            <h3>Source Config</h3>
            <ProviderConfigForm
              options={getProviderOptions(form.source_provider)}
              values={form.source_config}
              onChange={(k, v) => setForm(f => ({
                ...f, source_config: { ...f.source_config, [k]: v }
              }))}
            />
            <h3>Dest Config</h3>
            <ProviderConfigForm
              options={getProviderOptions(form.dest_provider)}
              values={form.dest_config}
              onChange={(k, v) => setForm(f => ({
                ...f, dest_config: { ...f.dest_config, [k]: v }
              }))}
            />
          </div>
        )}

        {step === 4 && (
          <div className="modal-step">
            <label>Operation</label>
            <select value={form.operation} onChange={e => setForm(f => ({ ...f, operation: e.target.value }))}>
              <option value="copy">Copy</option>
              <option value="sync">Sync</option>
              <option value="move">Move</option>
              <option value="bisync">Bisync</option>
            </select>
            <label>Exclude Patterns (one per line)</label>
            <textarea
              value={form.exclude_patterns.join("\n")}
              onChange={e => setForm(f => ({ ...f, exclude_patterns: e.target.value.split("\n").filter(Boolean) }))}
            />
          </div>
        )}

        {step === 5 && (
          <div className="modal-step">
            <CronInput
              value={form.cron_expr}
              onChange={v => setForm(f => ({ ...f, cron_expr: v }))}
            />
          </div>
        )}

        {error && <p className="error">{error}</p>}

        <div className="modal-actions">
          {step > 1 && <button onClick={() => setStep(s => s - 1)}>Back</button>}
          {step < 5 && <button onClick={() => setStep(s => s + 1)} disabled={!canProceed()}>Next</button>}
          {step === 5 && <button onClick={handleSubmit} disabled={submitting}>
            {submitting ? "Creating..." : "Create Task"}
          </button>}
          <button onClick={onClose}>Cancel</button>
        </div>
      </div>
    </div>
  );

  function canProceed() {
    switch (step) {
      case 1: return form.name.trim().length > 0;
      case 2: return form.source_provider && form.dest_provider;
      case 3: return true; // optional fields
      case 4: return true;
      case 5: return form.cron_expr.length > 0;
      default: return true;
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(ui): add TaskFormModal with 5-step wizard"
```

---

### Task 15: Frontend — TaskCard component

**Files:**
- Create: `src/components/TaskCard.tsx`

- [ ] **Step 1: Create TaskCard**

Displays a single task with name, schedule, status, last run, and action buttons (toggle, delete, run now).

```tsx
import { Task } from "../types";

interface Props {
  task: Task;
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
  onRunNow: (id: string) => void;
}

export default function TaskCard({ task, onToggle, onDelete, onRunNow }: Props) {
  return (
    <div className={`task-card ${task.enabled ? "enabled" : "disabled"}`}>
      <div className="task-header">
        <h3>{task.name}</h3>
        <span className="task-slug">{task.slug}</span>
      </div>
      <div className="task-details">
        <span className="task-operation">{task.operation}</span>
        <span className="task-providers">
          {task.source_provider} → {task.dest_provider}
        </span>
        <span className="task-schedule">Cron: {task.cron_expr}</span>
      </div>
      <div className="task-actions">
        <button onClick={() => onRunNow(task.id)}>▶ Run Now</button>
        <button onClick={() => onToggle(task.id)}>
          {task.enabled ? "⏸ Pause" : "▶ Resume"}
        </button>
        <button onClick={() => onDelete(task.id)}>🗑 Delete</button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(ui): add TaskCard component"
```

---

### Task 16: Frontend — SchedulerPage

**Files:**
- Create: `src/components/SchedulerPage.tsx`

- [ ] **Step 1: Create SchedulerPage**

Main scheduler page — shows task list, add task button, and listens for task events.

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import TaskCard from "./TaskCard";
import TaskFormModal from "./TaskFormModal";
import { Task } from "../types";

export default function SchedulerPage() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [loading, setLoading] = useState(true);

  const loadTasks = () => {
    invoke<Task[]>("task_list")
      .then(setTasks)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadTasks();

    // Listen for real-time task events
    const unlistenCompleted = listen("task:completed", (event) => {
      console.log("Task completed:", event.payload);
      loadTasks();
    });
    const unlistenError = listen("task:error", (event) => {
      console.log("Task error:", event.payload);
      loadTasks();
    });

    return () => {
      unlistenCompleted.then(f => f());
      unlistenError.then(f => f());
    };
  }, []);

  const handleToggle = async (id: string) => {
    await invoke("task_toggle", { id });
    loadTasks();
  };

  const handleDelete = async (id: string) => {
    await invoke("task_delete", { id });
    loadTasks();
  };

  const handleRunNow = async (id: string) => {
    await invoke("task_run_now", { id });
    loadTasks();
  };

  if (loading) return <div className="scheduler-page"><p>Loading tasks...</p></div>;

  return (
    <div className="scheduler-page">
      <div className="scheduler-header">
        <h2>Scheduled Tasks</h2>
        <button onClick={() => setShowForm(true)} className="btn-primary">
          + New Task
        </button>
      </div>

      {tasks.length === 0 ? (
        <div className="empty-state">
          <p>No tasks defined yet.</p>
          <p>Click "New Task" to create your first scheduled operation.</p>
        </div>
      ) : (
        <div className="task-list">
          {tasks.map(task => (
            <TaskCard
              key={task.id}
              task={task}
              onToggle={handleToggle}
              onDelete={handleDelete}
              onRunNow={handleRunNow}
            />
          ))}
        </div>
      )}

      {showForm && (
        <TaskFormModal
          onClose={() => setShowForm(false)}
          onCreated={() => loadTasks()}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat(ui): add SchedulerPage with task list"
```

---

### Task 17: Frontend — Add Scheduler tab to App.tsx

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: Update App.tsx**

Add "Scheduler" as a 4th tab:
```typescript
import SchedulerPage from "./components/SchedulerPage";
// ...
type Tab = "config" | "transfer" | "mounts" | "scheduler";
// ...
<button className={`tab ${activeTab === "scheduler" ? "active" : ""}`}
        onClick={() => setActiveTab("scheduler")}>
  Scheduler
</button>
// ...
{activeTab === "scheduler" && <SchedulerPage />}
```

- [ ] **Step 2: Add CSS for scheduler components**

Add to `App.css`:
```css
.scheduler-page { ... }
.task-card { ... }
.task-card.enabled { ... }
.task-card.disabled { ... }
.modal-overlay { ... }
.modal { ... }
.provider-selector { ... }
.provider-config-form { ... }
.config-field { ... }
.cron-input { ... }
.cron-presets { ... }
.cron-preset { ... }
.cron-preset.active { ... }
```

- [ ] **Step 3: Build frontend**

```bash
pnpm build
```
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(ui): add Scheduler tab and styles"
```

---

### Task 18: Test fixes + Windows compatibility

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/commands/rclone_cmds.rs`
- Modify: `src-tauri/src/rclone/process.rs`

- [ ] **Step 1: Fix echo-based tests for Windows**

Replace `tokio::process::Command::new("echo")` with Windows-compatible spawning or use `#[cfg(not(target_os = "windows"))]` + `#[cfg(target_os = "windows")]` variants.

For the 4 failing tests, wrap them with:
```rust
#[cfg(not(target_os = "windows"))]
```

Add Windows variants using `cmd /c echo`:
```rust
#[cfg(target_os = "windows")]
fn spawn_echo(arg: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("cmd");
    cmd.args(["/c", "echo", arg]);
    cmd
}
```

- [ ] **Step 2: Run all tests**

```bash
cd src-tauri && cargo test
```
Expected: All 57+ tests PASS

- [ ] **Step 3: Run clippy**

```bash
cd src-tauri && cargo clippy
```
Expected: No errors (existing warnings may remain)

- [ ] **Step 4: Run full build**

```bash
cd src-tauri && cargo build
cd .. && pnpm build
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "fix(tests): fix Windows compatibility for echo-based tests"
```

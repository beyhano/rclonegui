# Task 1: Add `cron` dependency + DB migration for tasks table + task_id to transfers

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/db/migrations.rs`
- Test: inline in migrations.rs

**Interfaces:**
- Consumes: existing `db::migrations::create_tables(conn)`
- Produces: `create_tables()` now also creates `tasks` table, adds `task_id` to `transfers`

### Step 1: Add `cron` crate

Edit `Cargo.toml` — add after the existing dependencies:
```toml
cron = "0.15"
```

Run `cargo check` to verify it compiles.

### Step 2: Add tasks table + task_id column to migrations

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

For the transfers table `task_id` column — since SQLite doesn't support `ALTER TABLE ADD COLUMN IF NOT EXISTS`, use this safe migration pattern:

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

### Step 3: Write test for tasks table creation

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

### Step 4: Run tests

```bash
cd src-tauri && cargo test test_create_tables_creates_tasks_table test_tasks_table_has_expected_columns -- --nocapture
```
Expected: PASS

### Step 5: Commit

```bash
git add -A && git commit -m "feat(db): add tasks table and task_id column to transfers"
```

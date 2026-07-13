# Task 2: Task data model + TaskRepo CRUD

**Files:**
- Create: `src-tauri/src/db/task_repo.rs`
- Modify: `src-tauri/src/db/mod.rs`

**Interfaces:**
- Consumes: `Connection`, `serde_json::Value` for config/patterns
- Produces: `Task` struct (Serialize + Deserialize), `TaskRepo` struct with:
  - `new(conn)`
  - `list() -> Result<Vec<Task>>`
  - `get_by_id(id) -> Result<Option<Task>>`
  - `get_by_slug(slug) -> Result<Option<Task>>`
  - `create(task) -> Result<()>`
  - `update(task) -> Result<()>`
  - `delete(id) -> Result<()>`
  - `get_enabled() -> Result<Vec<Task>>`

### Task struct

The Task struct must match the SQL schema exactly:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub source_provider: String,
    pub source_config: serde_json::Value,   // JSON object
    pub dest_provider: String,
    pub dest_config: serde_json::Value,     // JSON object
    pub operation: String,                  // copy | sync | move | bisync
    pub exclude_patterns: Vec<String>,       // ["*.tmp", "node_modules"]
    pub cron_expr: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

Note: `chrono::Utc::now()` is used for `created_at` and `updated_at` during creation, but for updates only `updated_at` changes (preserve original `created_at`).

### TaskRepo implementation

```rust
pub struct TaskRepo {
    conn: rusqlite::Connection,
}

impl TaskRepo {
    pub fn new(conn: Connection) -> Self { Self { conn } }
    pub fn list(&self) -> Result<Vec<Task>> { /* SELECT * FROM tasks ORDER BY created_at DESC */ }
    pub fn get_by_id(&self, id: &str) -> Result<Option<Task>> { /* WHERE id = ?1 */ }
    pub fn get_by_slug(&self, slug: &str) -> Result<Option<Task>> { /* WHERE slug = ?1 */ }
    pub fn create(&self, task: &Task) -> Result<()> { /* INSERT INTO tasks ... */ }
    pub fn update(&self, task: &Task) -> Result<()> { /* UPDATE tasks SET ... WHERE id = ?1 */ }
    pub fn delete(&self, id: &str) -> Result<()> { /* DELETE FROM tasks WHERE id = ?1 */ }
    pub fn get_enabled(&self) -> Result<Vec<Task>> { /* WHERE enabled = 1 */ }
}
```

For `source_config` and `dest_config`, use `serde_json::to_string()` to convert to string for storage and `serde_json::from_str()` to parse from string. For `exclude_patterns`, use the same JSON serialization approach.

### Tests

Write comprehensive tests in the same file:

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

    fn sample_task() -> Task { /* create a full Task with UUID id, valid cron etc */ }

    #[test]
    fn test_create_and_list() { /* create a task, list, assert it's there */ }
    #[test]
    fn test_get_by_id() { /* create, get_by_id, assert fields match */ }
    #[test]
    fn test_get_by_slug() { /* create, get_by_slug, assert found */ }
    #[test]
    fn test_get_by_id_not_found() { /* non-existent id returns None */ }
    #[test]
    fn test_update() { /* create, update name, verify */ }
    #[test]
    fn test_delete() { /* create, delete, list empty */ }
    #[test]
    fn test_get_enabled() { /* create one enabled, one disabled, get_enabled returns 1 */ }
    #[test]
    fn test_empty_list() { /* no tasks, list returns empty vec */ }
}
```

### Register module

In `src-tauri/src/db/mod.rs`:
```rust
pub mod migrations;
pub mod models;
pub mod task_repo;
```

## Verification

```bash
cd src-tauri && cargo test task_repo -- --nocapture
```

Expected: All tests PASS.

## Commit

```bash
git add -A && git commit -m "feat(db): add Task model and TaskRepo CRUD"
```

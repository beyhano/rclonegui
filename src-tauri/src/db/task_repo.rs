/// Task data model and TaskRepo CRUD for scheduled transfer definitions.
///
/// Provides the `Task` struct matching the `tasks` SQL schema, and `TaskRepo`
/// with full CRUD operations backed by SQLite.
///
/// JSON fields (`source_config`, `dest_config`, `exclude_patterns`) are
/// serialized to TEXT in SQLite using `serde_json`.

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

/// A scheduled transfer task definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub source_provider: String,
    pub source_config: serde_json::Value,
    pub dest_provider: String,
    pub dest_config: serde_json::Value,
    pub operation: String,
    pub exclude_patterns: Vec<String>,
    pub cron_expr: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Repository for CRUD operations on the `tasks` table.
pub struct TaskRepo {
    conn: rusqlite::Connection,
}

impl TaskRepo {
    /// Create a new `TaskRepo` that owns the given SQLite connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Return a reference to the inner connection (for use with migrations etc.).
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// List all tasks ordered by `created_at` descending.
    pub fn list(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, slug, source_provider, source_config, dest_provider, \
                    dest_config, operation, exclude_patterns, cron_expr, enabled, \
                    created_at, updated_at \
             FROM tasks ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], map_row)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    /// Get a single task by its primary key.
    pub fn get_by_id(&self, id: &str) -> Result<Option<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, slug, source_provider, source_config, dest_provider, \
                    dest_config, operation, exclude_patterns, cron_expr, enabled, \
                    created_at, updated_at \
             FROM tasks WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_row(row)?)),
            None => Ok(None),
        }
    }

    /// Get a single task by its unique slug.
    pub fn get_by_slug(&self, slug: &str) -> Result<Option<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, slug, source_provider, source_config, dest_provider, \
                    dest_config, operation, exclude_patterns, cron_expr, enabled, \
                    created_at, updated_at \
             FROM tasks WHERE slug = ?1",
        )?;

        let mut rows = stmt.query(params![slug])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_row(row)?)),
            None => Ok(None),
        }
    }

    /// Insert a new task row.
    pub fn create(&self, task: &Task) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tasks \
             (id, name, slug, source_provider, source_config, dest_provider, \
              dest_config, operation, exclude_patterns, cron_expr, enabled, \
              created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                task.id,
                task.name,
                task.slug,
                task.source_provider,
                serde_json::to_string(&task.source_config).unwrap_or_default(),
                task.dest_provider,
                serde_json::to_string(&task.dest_config).unwrap_or_default(),
                task.operation,
                serde_json::to_string(&task.exclude_patterns).unwrap_or_default(),
                task.cron_expr,
                task.enabled as i32,
                task.created_at,
                task.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Update an existing task. Preserves the original `created_at` value;
    /// only `updated_at` is changed to reflect the modification time.
    pub fn update(&self, task: &Task) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET name = ?1, slug = ?2, source_provider = ?3, \
                    source_config = ?4, dest_provider = ?5, dest_config = ?6, \
                    operation = ?7, exclude_patterns = ?8, cron_expr = ?9, \
                    enabled = ?10, updated_at = ?11 \
             WHERE id = ?12",
            params![
                task.name,
                task.slug,
                task.source_provider,
                serde_json::to_string(&task.source_config).unwrap_or_default(),
                task.dest_provider,
                serde_json::to_string(&task.dest_config).unwrap_or_default(),
                task.operation,
                serde_json::to_string(&task.exclude_patterns).unwrap_or_default(),
                task.cron_expr,
                task.enabled as i32,
                task.updated_at,
                task.id,
            ],
        )?;
        Ok(())
    }

    /// Delete a task by its primary key.
    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// List all enabled tasks, ordered by `created_at` descending.
    pub fn get_enabled(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, slug, source_provider, source_config, dest_provider, \
                    dest_config, operation, exclude_patterns, cron_expr, enabled, \
                    created_at, updated_at \
             FROM tasks WHERE enabled = 1 ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], map_row)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let source_config_str: String = row.get(4)?;
    let dest_config_str: String = row.get(6)?;
    let exclude_patterns_str: String = row.get(8)?;
    let enabled_int: i32 = row.get(10)?;

    Ok(Task {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        source_provider: row.get(3)?,
        source_config: serde_json::from_str(&source_config_str).unwrap_or_default(),
        dest_provider: row.get(5)?,
        dest_config: serde_json::from_str(&dest_config_str).unwrap_or_default(),
        operation: row.get(7)?,
        exclude_patterns: serde_json::from_str(&exclude_patterns_str).unwrap_or_default(),
        cron_expr: row.get(9)?,
        enabled: enabled_int != 0,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::create_tables;

    fn setup_repo() -> TaskRepo {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        TaskRepo::new(conn)
    }

    fn sample_task(id: &str, slug: &str) -> Task {
        Task {
            id: id.to_string(),
            name: format!("Test Task {}", id),
            slug: slug.to_string(),
            source_provider: "local".into(),
            source_config: serde_json::json!({"path": "/tmp/src"}),
            dest_provider: "gdrive".into(),
            dest_config: serde_json::json!({"folder": "backup"}),
            operation: "sync".into(),
            exclude_patterns: vec!["*.tmp".into(), "node_modules".into()],
            cron_expr: "0 0 * * *".into(),
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        }
    }

    // -- create + list --

    #[test]
    fn test_create_and_list() {
        let repo = setup_repo();
        let task = sample_task("t1", "test-task-t1");
        repo.create(&task).unwrap();

        let tasks = repo.list().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "t1");
        assert_eq!(tasks[0].name, "Test Task t1");
    }

    #[test]
    fn test_empty_list() {
        let repo = setup_repo();
        let tasks = repo.list().unwrap();
        assert!(tasks.is_empty());
    }

    // -- get_by_id --

    #[test]
    fn test_get_by_id() {
        let repo = setup_repo();
        let task = sample_task("t2", "test-task-t2");
        repo.create(&task).unwrap();

        let found = repo.get_by_id("t2").unwrap();
        assert!(found.is_some());
        let t = found.unwrap();
        assert_eq!(t.name, "Test Task t2");
        assert_eq!(t.slug, "test-task-t2");
        assert_eq!(t.source_provider, "local");
        assert_eq!(t.dest_provider, "gdrive");
        assert_eq!(t.operation, "sync");
        assert_eq!(t.exclude_patterns, vec!["*.tmp", "node_modules"]);
        assert_eq!(t.cron_expr, "0 0 * * *");
        assert!(t.enabled);
        // JSON fields round-trip
        assert_eq!(t.source_config, serde_json::json!({"path": "/tmp/src"}));
        assert_eq!(t.dest_config, serde_json::json!({"folder": "backup"}));
    }

    #[test]
    fn test_get_by_id_not_found() {
        let repo = setup_repo();
        let found = repo.get_by_id("nonexistent").unwrap();
        assert!(found.is_none());
    }

    // -- get_by_slug --

    #[test]
    fn test_get_by_slug() {
        let repo = setup_repo();
        let task = sample_task("t3", "unique-slug");
        repo.create(&task).unwrap();

        let found = repo.get_by_slug("unique-slug").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "t3");
    }

    #[test]
    fn test_get_by_slug_not_found() {
        let repo = setup_repo();
        let found = repo.get_by_slug("no-such-slug").unwrap();
        assert!(found.is_none());
    }

    // -- update --

    #[test]
    fn test_update() {
        let repo = setup_repo();
        let mut task = sample_task("t4", "update-me");
        repo.create(&task).unwrap();

        // Update name + enabled
        task.name = "Updated Name".into();
        task.enabled = false;
        task.updated_at = "2024-06-01T00:00:00Z".into();
        repo.update(&task).unwrap();

        let found = repo.get_by_id("t4").unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
        assert!(!found.enabled);
        // Original created_at must be preserved
        assert_eq!(found.created_at, "2024-01-01T00:00:00Z");
        // updated_at changed
        assert_eq!(found.updated_at, "2024-06-01T00:00:00Z");
    }

    // -- delete --

    #[test]
    fn test_delete() {
        let repo = setup_repo();
        let task = sample_task("t5", "delete-me");
        repo.create(&task).unwrap();

        repo.delete("t5").unwrap();
        let tasks = repo.list().unwrap();
        assert!(tasks.is_empty());
    }

    // -- get_enabled --

    #[test]
    fn test_get_enabled() {
        let repo = setup_repo();

        let mut enabled_task = sample_task("t-en-1", "enabled-one");
        enabled_task.enabled = true;
        repo.create(&enabled_task).unwrap();

        let mut disabled_task = sample_task("t-dis-1", "disabled-one");
        disabled_task.enabled = false;
        disabled_task.slug = "disabled-one".into();
        repo.create(&disabled_task).unwrap();

        let enabled = repo.get_enabled().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "t-en-1");
    }

    // -- list ordering --

    #[test]
    fn test_list_ordered_by_created_at_desc() {
        let repo = setup_repo();

        let mut earlier = sample_task("t-e", "earlier");
        earlier.created_at = "2024-01-01T00:00:00Z".into();
        earlier.updated_at = "2024-01-01T00:00:00Z".into();
        repo.create(&earlier).unwrap();

        let mut later = sample_task("t-l", "later");
        later.created_at = "2024-06-01T00:00:00Z".into();
        later.updated_at = "2024-06-01T00:00:00Z".into();
        repo.create(&later).unwrap();

        let tasks = repo.list().unwrap();
        assert_eq!(tasks[0].id, "t-l");
        assert_eq!(tasks[1].id, "t-e");
    }

    // -- slug uniqueness enforced by DB --

    #[test]
    fn test_duplicate_slug_errors() {
        let repo = setup_repo();
        let task1 = sample_task("dup-1", "same-slug");
        repo.create(&task1).unwrap();

        let task2 = sample_task("dup-2", "same-slug");
        let result = repo.create(&task2);
        assert!(result.is_err(), "duplicate slug should cause UNIQUE constraint violation");
    }

    // -- null / missing JSON fields --

    #[test]
    fn test_json_fields_round_trip_empty() {
        let repo = setup_repo();
        let mut task = sample_task("t-json", "json-test");
        task.source_config = serde_json::Value::Null;
        task.dest_config = serde_json::Value::Null;
        task.exclude_patterns = vec![];
        repo.create(&task).unwrap();

        let found = repo.get_by_id("t-json").unwrap().unwrap();
        assert_eq!(found.source_config, serde_json::Value::Null);
        assert_eq!(found.dest_config, serde_json::Value::Null);
        assert!(found.exclude_patterns.is_empty());
    }
}

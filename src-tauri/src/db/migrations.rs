/// SQLite schema migrations for rclonegui.
///
/// Creates all tables on first initialization using `CREATE TABLE IF NOT EXISTS`
/// so the function is idempotent and safe to call on every app launch.

use rusqlite::Connection;

/// Create all application tables if they do not already exist.
///
/// # Tables
///
/// - `transfers` — Tracks rclone copy/sync operations with status and progress.
/// - `mounts` — Tracks rclone mount processes and their state.
/// - `app_config` — Simple key-value store for application settings.
/// - `tasks` — Scheduled task definitions for automated transfers.
///
/// # Errors
///
/// Propagates any `rusqlite` error from executing the batch SQL.
pub fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS transfers (
            id            TEXT PRIMARY KEY,
            remote_src    TEXT NOT NULL,
            remote_dest   TEXT NOT NULL,
            status        TEXT NOT NULL DEFAULT 'running',
            progress      REAL DEFAULT 0.0,
            speed         TEXT,
            started_at    TEXT NOT NULL,
            completed_at  TEXT,
            error_message TEXT
        );

        CREATE TABLE IF NOT EXISTS mounts (
            id          TEXT PRIMARY KEY,
            remote      TEXT NOT NULL,
            mount_point TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'running',
            started_at  TEXT NOT NULL,
            pid         INTEGER
        );

        CREATE TABLE IF NOT EXISTS app_config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id              TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            slug            TEXT NOT NULL UNIQUE,
            source_provider TEXT NOT NULL,
            source_config   TEXT NOT NULL,
            dest_provider   TEXT NOT NULL,
            dest_config     TEXT NOT NULL,
            operation       TEXT NOT NULL,
            exclude_patterns TEXT NOT NULL DEFAULT '[]',
            cron_expr       TEXT NOT NULL,
            enabled         INTEGER NOT NULL DEFAULT 1,
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        );
        ",
    )?;

    // Only run transfer migration if transfers doesn't already have task_id
    let has_task_id: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('transfers') WHERE name='task_id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_task_id {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS transfers_v2 (
                id            TEXT PRIMARY KEY,
                remote_src    TEXT NOT NULL,
                remote_dest   TEXT NOT NULL,
                status        TEXT NOT NULL DEFAULT 'running',
                progress      REAL DEFAULT 0.0,
                speed         TEXT,
                started_at    TEXT NOT NULL,
                completed_at  TEXT,
                error_message TEXT,
                task_id       TEXT
            );
            INSERT OR IGNORE INTO transfers_v2 SELECT id, remote_src, remote_dest, status, progress, speed, started_at, completed_at, error_message, NULL as task_id FROM transfers;
            DROP TABLE transfers;
            ALTER TABLE transfers_v2 RENAME TO transfers;
            ",
        )?;
    }

    Ok(())
}

// ----- create_tables asserts all 4 tables exist (transfers, mounts, app_config, tasks) -----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tables_creates_transfers_table() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='transfers'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1, "transfers table should exist");
    }

    #[test]
    fn test_create_tables_creates_mounts_table() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='mounts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1, "mounts table should exist");
    }

    #[test]
    fn test_create_tables_creates_app_config_table() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_config'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1, "app_config table should exist");
    }

    #[test]
    fn test_create_tables_all_four_tables_exist_via_pragma() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        // Use PRAGMA table_info to confirm each table has the expected structure
        for table in &["transfers", "mounts", "app_config", "tasks"] {
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            let columns: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            assert!(!columns.is_empty(), "{table} should have at least one column");
        }
    }

    #[test]
    fn test_create_tables_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        // Running a second time must not error
        create_tables(&conn).unwrap();

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('transfers', 'mounts', 'app_config', 'tasks')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 4, "all 4 tables should still exist after second call");
    }

    #[test]
    fn test_transfers_table_has_expected_schema() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        let mut stmt = conn.prepare("PRAGMA table_info(transfers)").unwrap();
        let columns: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // name
                    row.get::<_, String>(2)?, // type
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(columns.len(), 10, "transfers should have 10 columns (added task_id)");
        assert_eq!(columns[0], ("id".to_string(), "TEXT".to_string()));
        assert_eq!(columns[1], ("remote_src".to_string(), "TEXT".to_string()));
        assert_eq!(columns[9], ("task_id".to_string(), "TEXT".to_string()));
    }

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
}

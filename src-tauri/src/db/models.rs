/// Data models and CRUD operations for rclone persistence.
///
/// Provides `Transfer`, `Mount`, and `AppConfig` structs with
/// the corresponding insert/update/query functions backed by SQLite.

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

/// A single rclone copy/sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub remote_src: String,
    pub remote_dest: String,
    pub status: String,
    pub progress: f64,
    pub speed: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

/// A single rclone mount process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub id: String,
    pub remote: String,
    pub mount_point: String,
    pub status: String,
    pub started_at: String,
    pub pid: Option<i64>,
}

/// A key-value application configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub key: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// Transfer CRUD
// ---------------------------------------------------------------------------

/// Insert a new transfer record into the database.
pub fn insert_transfer(conn: &Connection, transfer: &Transfer) -> Result<()> {
    conn.execute(
        "INSERT INTO transfers (id, remote_src, remote_dest, status, progress, speed, started_at, completed_at, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            transfer.id,
            transfer.remote_src,
            transfer.remote_dest,
            transfer.status,
            transfer.progress,
            transfer.speed,
            transfer.started_at,
            transfer.completed_at,
            transfer.error_message,
        ],
    )?;
    Ok(())
}

/// Update the status and optionally progress fields for a transfer.
pub fn update_transfer_status(
    conn: &Connection,
    id: &str,
    status: &str,
    progress: Option<f64>,
    speed: Option<&str>,
    completed_at: Option<&str>,
    error_message: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE transfers SET status = ?1, progress = COALESCE(?2, progress), speed = COALESCE(?3, speed), completed_at = COALESCE(?4, completed_at), error_message = COALESCE(?5, error_message) WHERE id = ?6",
        params![status, progress, speed, completed_at, error_message, id],
    )?;
    Ok(())
}

/// Retrieve all transfer records ordered by started_at descending.
pub fn get_transfer_history(conn: &Connection) -> Result<Vec<Transfer>> {
    let mut stmt = conn.prepare(
        "SELECT id, remote_src, remote_dest, status, progress, speed, started_at, completed_at, error_message
         FROM transfers ORDER BY started_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Transfer {
            id: row.get(0)?,
            remote_src: row.get(1)?,
            remote_dest: row.get(2)?,
            status: row.get(3)?,
            progress: row.get(4)?,
            speed: row.get(5)?,
            started_at: row.get(6)?,
            completed_at: row.get(7)?,
            error_message: row.get(8)?,
        })
    })?;

    let mut transfers = Vec::new();
    for row in rows {
        transfers.push(row?);
    }
    Ok(transfers)
}

// ---------------------------------------------------------------------------
// Mount CRUD
// ---------------------------------------------------------------------------

/// Insert a new mount record into the database.
pub fn insert_mount(conn: &Connection, mount: &Mount) -> Result<()> {
    conn.execute(
        "INSERT INTO mounts (id, remote, mount_point, status, started_at, pid)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            mount.id,
            mount.remote,
            mount.mount_point,
            mount.status,
            mount.started_at,
            mount.pid,
        ],
    )?;
    Ok(())
}

/// Update the status and optionally pid for a mount.
pub fn update_mount_status(
    conn: &Connection,
    id: &str,
    status: &str,
    pid: Option<i64>,
) -> Result<()> {
    conn.execute(
        "UPDATE mounts SET status = ?1, pid = COALESCE(?2, pid) WHERE id = ?3",
        params![status, pid, id],
    )?;
    Ok(())
}

/// Retrieve all mount records.
pub fn get_mounts(conn: &Connection) -> Result<Vec<Mount>> {
    let mut stmt = conn.prepare(
        "SELECT id, remote, mount_point, status, started_at, pid FROM mounts",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Mount {
            id: row.get(0)?,
            remote: row.get(1)?,
            mount_point: row.get(2)?,
            status: row.get(3)?,
            started_at: row.get(4)?,
            pid: row.get(5)?,
        })
    })?;

    let mut mounts = Vec::new();
    for row in rows {
        mounts.push(row?);
    }
    Ok(mounts)
}

// ---------------------------------------------------------------------------
// AppConfig CRUD
// ---------------------------------------------------------------------------

/// Set a configuration value (insert or update).
pub fn set_config(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Get a configuration value by key.
pub fn get_config(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM app_config WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

// ----- Integration-style unit tests for CRUD operations -----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::create_tables;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        conn
    }

    // -- Transfer CRUD --

    #[test]
    fn test_insert_and_query_transfer() {
        let conn = setup_db();
        let t = Transfer {
            id: "test-id-1".into(),
            remote_src: "src:".into(),
            remote_dest: "dest:".into(),
            status: "running".into(),
            progress: 42.5,
            speed: Some("10 MiB/s".into()),
            started_at: "2024-01-01T00:00:00Z".into(),
            completed_at: None,
            error_message: None,
        };
        insert_transfer(&conn, &t).unwrap();

        let history = get_transfer_history(&conn).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "test-id-1");
        assert_eq!(history[0].status, "running");
        assert!((history[0].progress - 42.5).abs() < f64::EPSILON);
        assert_eq!(history[0].speed, Some("10 MiB/s".into()));
    }

    #[test]
    fn test_update_transfer_status() {
        let conn = setup_db();
        let t = Transfer {
            id: "test-id-2".into(),
            remote_src: "src:".into(),
            remote_dest: "dest:".into(),
            status: "running".into(),
            progress: 0.0,
            speed: None,
            started_at: "2024-01-01T00:00:00Z".into(),
            completed_at: None,
            error_message: None,
        };
        insert_transfer(&conn, &t).unwrap();

        update_transfer_status(
            &conn,
            "test-id-2",
            "completed",
            Some(100.0),
            Some("0 B/s"),
            Some("2024-01-01T01:00:00Z"),
            None,
        )
        .unwrap();

        let history = get_transfer_history(&conn).unwrap();
        assert_eq!(history[0].status, "completed");
        assert!((history[0].progress - 100.0).abs() < f64::EPSILON);
        assert_eq!(history[0].speed, Some("0 B/s".into()));
        assert_eq!(history[0].completed_at, Some("2024-01-01T01:00:00Z".into()));
    }

    #[test]
    fn test_get_transfer_history_empty() {
        let conn = setup_db();
        let history = get_transfer_history(&conn).unwrap();
        assert!(history.is_empty());
    }

    // -- Mount CRUD --

    #[test]
    fn test_insert_and_query_mount() {
        let conn = setup_db();
        let m = Mount {
            id: "mount-id-1".into(),
            remote: "gdrive:".into(),
            mount_point: "/mnt/gdrive".into(),
            status: "running".into(),
            started_at: "2024-01-01T00:00:00Z".into(),
            pid: Some(12345),
        };
        insert_mount(&conn, &m).unwrap();

        let mounts = get_mounts(&conn).unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].id, "mount-id-1");
        assert_eq!(mounts[0].remote, "gdrive:");
        assert_eq!(mounts[0].pid, Some(12345));
    }

    #[test]
    fn test_update_mount_status() {
        let conn = setup_db();
        let m = Mount {
            id: "mount-id-2".into(),
            remote: "s3:".into(),
            mount_point: "/mnt/s3".into(),
            status: "running".into(),
            started_at: "2024-01-01T00:00:00Z".into(),
            pid: None,
        };
        insert_mount(&conn, &m).unwrap();

        update_mount_status(&conn, "mount-id-2", "unmounted", Some(99999)).unwrap();

        let mounts = get_mounts(&conn).unwrap();
        assert_eq!(mounts[0].status, "unmounted");
        assert_eq!(mounts[0].pid, Some(99999));
    }

    #[test]
    fn test_get_mounts_empty() {
        let conn = setup_db();
        let mounts = get_mounts(&conn).unwrap();
        assert!(mounts.is_empty());
    }

    // -- AppConfig CRUD --

    #[test]
    fn test_set_and_get_config() {
        let conn = setup_db();
        set_config(&conn, "theme", "dark").unwrap();

        let val = get_config(&conn, "theme").unwrap();
        assert_eq!(val, Some("dark".into()));
    }

    #[test]
    fn test_set_config_upsert() {
        let conn = setup_db();
        set_config(&conn, "lang", "en").unwrap();
        set_config(&conn, "lang", "tr").unwrap();

        let val = get_config(&conn, "lang").unwrap();
        assert_eq!(val, Some("tr".into()));
    }

    #[test]
    fn test_get_config_missing_key() {
        let conn = setup_db();
        let val = get_config(&conn, "nonexistent").unwrap();
        assert_eq!(val, None);
    }
}

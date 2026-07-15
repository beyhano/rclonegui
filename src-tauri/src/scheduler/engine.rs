use chrono::Utc;
use serde::Serialize;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

pub use crate::db::task_repo::Task;
use crate::rclone::events::parse_progress_line;

/// Build a `tokio::process::Command` that never opens a console window on Windows.
fn no_window_cmd(program: impl AsRef<std::ffi::OsStr>) -> tokio::process::Command {
    let cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    let cmd = {
        let mut cmd = cmd;
        cmd.creation_flags(0x0800_0000);
        cmd
    };
    cmd
}

/// Start a null WebDAV sink and return (server_handle, rclone_webdav_args).
///
/// On Unix:    binds a Unix domain socket — no TCP stack overhead at all.
/// On Windows: binds a random TCP loopback port (tokio UnixListener not available).
async fn start_null_webdav(process_id: Uuid) -> Result<(tokio::task::JoinHandle<()>, Vec<String>), String> {
    #[cfg(unix)]
    {
        use tokio::net::UnixListener;
        let socket_path = std::env::temp_dir().join(format!("karadelik_{}.sock", process_id));
        // Remove stale socket if present
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)
            .map_err(|e| format!("Karadelik: Unix socket bind failed: {}", e))?;
        let path_clone = socket_path.clone();
        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                tokio::spawn(handle_null_connection(stream));
            }
            let _ = std::fs::remove_file(&path_clone);
        });
        let args = vec![
            format!("--webdav-unix-socket={}", socket_path.display()),
            "--webdav-url=http://localhost".to_string(),
        ];
        Ok((handle, args))
    }
    #[cfg(not(unix))]
    {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Karadelik: TCP bind failed: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Karadelik: local_addr failed: {}", e))?
            .port();
        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                let _pid = process_id;
                tokio::spawn(handle_null_connection(stream));
            }
        });
        let args = vec![
            format!("--webdav-url=http://127.0.0.1:{}", port),
        ];
        Ok((handle, args))
    }
}

/// Handle one WebDAV connection: parse minimal HTTP, discard PUT bodies, reply.
/// Generic over the stream type so it works for both Unix sockets and TCP.
async fn handle_null_connection<S>(mut stream: S)
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    const BUF: usize = 64 * 1024;
    let mut buf = vec![0u8; BUF];

    loop {
        // Read headers byte-by-byte until \r\n\r\n (up to 8 KB)
        let mut header_buf = vec![0u8; 8192];
        let mut n = 0usize;
        loop {
            if n >= header_buf.len() { return; }
            let Ok(r) = stream.read(&mut header_buf[n..n + 1]).await else { return };
            if r == 0 { return; }
            n += 1;
            if n >= 4 && &header_buf[n - 4..n] == b"\r\n\r\n" { break; }
        }
        let raw = std::str::from_utf8(&header_buf[..n]).unwrap_or("");

        // Parse method, path, Content-Length from headers
        let first_line = raw.lines().next().unwrap_or("");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let path   = parts.next().unwrap_or("/");

        let content_length: u64 = raw.lines()
            .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);

        // Drain PUT body in 64 KB streaming chunks — zero accumulation
        if method == "PUT" && content_length > 0 {
            let mut remaining = content_length;
            while remaining > 0 {
                let to_read = (remaining as usize).min(buf.len());
                let Ok(r) = stream.read(&mut buf[..to_read]).await else { return };
                if r == 0 { return; }
                remaining -= r as u64;
            }
        }

        // rclone WebDAV sequence:
        //   PROPFIND /          → 207 (root collection exists, proceed)
        //   PROPFIND /file.txt  → 404 (file absent, rclone will PUT it)
        //   MKCOL  /dir/        → 201
        //   PUT    /file.txt    → 201 (body already drained above)
        let is_root = path == "/" || path.is_empty();

        let (status_line, body) = match method {
            "OPTIONS" => (
                "HTTP/1.1 200 OK\r\nAllow: OPTIONS,GET,PUT,DELETE,MKCOL,PROPFIND,HEAD\r\nDAV: 1\r\n",
                None,
            ),
            "PROPFIND" if is_root => {
                // Minimal 207 so rclone believes the collection is present
                let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
                    <D:multistatus xmlns:D=\"DAV:\">\
                    <D:response><D:href>/</D:href>\
                    <D:propstat><D:prop>\
                    <D:resourcetype><D:collection/></D:resourcetype>\
                    </D:prop><D:status>HTTP/1.1 200 OK</D:status>\
                    </D:propstat></D:response></D:multistatus>";
                (
                    "HTTP/1.1 207 Multi-Status\r\nContent-Type: application/xml; charset=utf-8\r\n",
                    Some(xml),
                )
            }
            "PROPFIND" => {
                // File-level 207 so post-PUT verification passes (--ignore-size skips size check)
                let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
                    <D:multistatus xmlns:D=\"DAV:\">\
                    <D:response><D:href>/f</D:href>\
                    <D:propstat><D:prop>\
                    <D:resourcetype/>\
                    <D:getcontentlength>0</D:getcontentlength>\
                    </D:prop><D:status>HTTP/1.1 200 OK</D:status>\
                    </D:propstat></D:response></D:multistatus>";
                (
                    "HTTP/1.1 207 Multi-Status\r\nContent-Type: application/xml; charset=utf-8\r\n",
                    Some(xml),
                )
            }
            "HEAD" | "GET" => ("HTTP/1.1 200 OK\r\n", None),
            "MKCOL" => ("HTTP/1.1 201 Created\r\n", None),
            "PUT"   => ("HTTP/1.1 201 Created\r\n", None),
            "DELETE" => ("HTTP/1.1 204 No Content\r\n", None),
            _ => ("HTTP/1.1 405 Method Not Allowed\r\n", None),
        };

        let response = if let Some(xml) = body {
            format!("{}Content-Length: {}\r\n\r\n{}", status_line, xml.len(), xml)
        } else {
            format!("{}Content-Length: 0\r\n\r\n", status_line)
        };

        if stream.write_all(response.as_bytes()).await.is_err() { return; }

        if raw.to_ascii_lowercase().contains("connection: close") { return; }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub task_id: String,
    pub process_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Execute a scheduled task: spawn rclone, capture progress, wait for exit.
pub async fn execute_task(
    task: &Task,
    rclone_path: &str,
    app: Option<&tauri::AppHandle<tauri::Wry>>,
) -> Result<TaskResult, String> {
    let started_at = Utc::now().to_rfc3339();
    let process_id = Uuid::new_v4();

    // --- Karadelik (Black Hole) handler ---
    // rclone has no null backend in this build. NUL/dev/null fail as local paths
    // (rclone tries mkdir \\?\UNC\NUL → "The specified path is invalid.").
    //
    // Strategy per operation:
    //   copy / bisync → local null WebDAV sink: streaming discard, 0 RAM, 0 disk
    //   move / sync   → rclone delete <source>: actually deletes source files (true black hole)
    let is_karadelik = task.dest_provider == "(karadelik)";
    let is_destructive = matches!(task.operation.as_str(), "move" | "sync");

    // Start null WebDAV server for copy+karadelik (before we build args)
    let null_server = if is_karadelik && !is_destructive {
        match start_null_webdav(process_id).await {
            Ok(s) => Some(s),
            Err(e) => {
                let msg = format!("WARN: Karadelik WebDAV sink başlatılamadı: {}", e);
                eprintln!("{}", msg);
                if let Some(a) = app {
                    let _ = a.emit("rclone:log", serde_json::json!({
                        "process_id": process_id.to_string(),
                        "line": &msg,
                    }));
                }
                None
            }
        }
    } else {
        None
    };

    if is_karadelik {
        let msg = if is_destructive {
            format!(
                "WARN: Karadelik + {} — kaynak dosyalar kalici olarak silinecek: {}",
                task.operation, task.source_provider
            )
        } else {
            let sink_info = null_server.as_ref()
                .map(|(_, args)| args.join(" "))
                .unwrap_or_default();
            format!(
                "INFO: Karadelik + {} — null WebDAV sink ({})  (0 RAM, 0 disk)",
                task.operation, sink_info
            )
        };
        eprintln!("{}", msg);
        if let Some(a) = app {
            let _ = a.emit("rclone:log", serde_json::json!({
                "process_id": process_id.to_string(),
                "line": &msg,
            }));
        }
    }

    // Build rclone args — source/dest are full paths (e.g. "gdrive:/backups" or "C:\Users\me")
    let args: Vec<String> = if is_karadelik && is_destructive {
        // move/sync to black hole → delete source files + empty dirs
        // --rmdirs: removes empty directories after deleting files
        let mut a = vec!["delete".to_string(), task.source_provider.clone(), "--rmdirs".to_string()];
        for pattern in &task.exclude_patterns {
            a.push("--exclude".to_string());
            a.push(pattern.clone());
        }
        a.push("--progress".to_string());
        a
    } else if is_karadelik {
        // copy/bisync to black hole → null WebDAV sink (streaming discard, 0 RAM, 0 disk)
        // Flags:
        //   --no-check-dest  : skip pre-upload PROPFIND (no destination check)
        //   --ignore-size    : skip post-upload size verification (our null server reports 0)
        let mut a = vec![
            task.operation.clone(),
            task.source_provider.clone(),
            ":webdav:".to_string(),
            "--webdav-vendor=other".to_string(),
            "--no-check-dest".to_string(),
            "--ignore-size".to_string(),
        ];
        // Append platform-specific sink args (--webdav-url / --webdav-unix-socket)
        if let Some((_, ref webdav_args)) = null_server {
            a.extend(webdav_args.iter().cloned());
        }
        for pattern in &task.exclude_patterns {
            a.push("--exclude".to_string());
            a.push(pattern.clone());
        }
        a.push("--progress".to_string());
        a
    } else {
        // Normal transfer
        let mut a = vec![
            task.operation.clone(),
            task.source_provider.clone(),
            task.dest_provider.clone(),
        ];
        for pattern in &task.exclude_patterns {
            a.push("--exclude".to_string());
            a.push(pattern.clone());
        }
        a.push("--progress".to_string());
        a
    };

    // Log the exact command for debugging
    let cmd_str = format!("CMD: rclone {}", args.join(" "));
    eprintln!("{}", cmd_str);
    if let Some(a) = app {
        let _ = a.emit("rclone:log", serde_json::json!({
            "process_id": process_id.to_string(),
            "line": &cmd_str,
        }));
    }

    let mut child = no_window_cmd(rclone_path)
        .args(&args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone for task '{}': {}", task.name, e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    // Register PID in AppState (task_id → PID) for targeted stop capability
    let pid = child.id().unwrap_or(0);
    if let Some(a) = app {
        let state = a.state::<crate::state::AppState>();
        let mut pids = state.task_pids.lock().await;
        pids.insert(task.id.clone(), pid);
    }

    let mut error_lines = Vec::new();

    // Read stdout, parse progress, and emit events
    let mut stdout_lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = stdout_lines.next_line().await {
        if let Some(payload) = parse_progress_line(process_id, &line) {
            if let Some(app) = app {
                let _ = app.emit("rclone:progress", &payload);
            }
        }
    }

    // Read stderr for errors and emit log events
    let mut stderr_lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = stderr_lines.next_line().await {
        if !line.is_empty() {
            if let Some(app) = app {
                let _ = app.emit(
                    "rclone:log",
                    serde_json::json!({
                        "process_id": process_id.to_string(),
                        "line": &line,
                    }),
                );
            }
            error_lines.push(line);
        }
    }

    // Wait for process exit
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Wait error: {}", e))?;
    let completed_at = Utc::now().to_rfc3339();
    let success = status.success();

    // Remove PID from tracking
    if let Some(a) = app {
        let state = a.state::<crate::state::AppState>();
        let mut pids = state.task_pids.lock().await;
        pids.remove(&task.id);
    }

    // Shut down the null WebDAV server if we started one
    if let Some((server_handle, _)) = null_server {
        server_handle.abort();
    }

    Ok(TaskResult {
        task_id: task.id.clone(),
        process_id: process_id.to_string(),
        started_at,
        completed_at: Some(completed_at),
        success,
        error_message: if success {
            None
        } else {
            Some(error_lines.join("\n"))
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::task_repo::Task;

    fn sample_task() -> Task {
        Task {
            id: "test-id".into(),
            name: "Test Task".into(),
            slug: "test-task".into(),
            source_provider: "local".into(),
            source_config: serde_json::Value::Null,
            dest_provider: "local".into(),
            dest_config: serde_json::Value::Null,
            operation: "copy".into(),
            exclude_patterns: vec![],
            cron_expr: "0 * * * * *".into(),
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_execute_task_invalid_path_returns_error() {
        let task = sample_task();
        let result = execute_task(&task, "/nonexistent/rclone", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_task_empty_path_returns_error() {
        let task = sample_task();
        let result = execute_task(&task, "", None).await;
        assert!(result.is_err());
    }
}

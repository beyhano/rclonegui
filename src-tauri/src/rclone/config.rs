/// Rclone configuration management.
///
/// Parses the output of `rclone config dump` and returns a list of configured
/// remotes with their names and types. Sensitive fields (tokens, passwords, etc.)
/// are stripped — only `name` and `type` are returned.
use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

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

/// A single rclone remote with its name and type.
///
/// Sensitive configuration fields are intentionally excluded from this struct.
#[derive(Debug, Clone, Serialize)]
pub struct Remote {
    pub name: String,
    #[serde(rename = "type")]
    pub remote_type: String,
}

/// Run `rclone config dump` and parse the JSON output into a list of remotes.
///
/// # Errors
///
/// - If the rclone binary cannot be executed.
/// - If `rclone config dump` exits with a non-zero status.
/// - If the JSON output cannot be parsed.
pub async fn config_list(rclone_path: &Path) -> Result<Vec<Remote>, String> {
    let output = no_window_cmd(rclone_path)
        .args(["config", "dump"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute rclone config dump: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone config dump failed: {}", stderr));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("Non-UTF-8 output: {}", e))?;

    let map: HashMap<String, HashMap<String, serde_json::Value>> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse rclone config: {}", e))?;

    let mut remotes: Vec<Remote> = map
        .into_iter()
        .map(|(name, mut props)| {
            let remote_type = props
                .remove("type")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            Remote { name, remote_type }
        })
        .collect();

    // Sort by name for deterministic output
    remotes.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(remotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_serialization() {
        let remote = Remote {
            name: "my_drive".into(),
            remote_type: "drive".into(),
        };
        let json = serde_json::to_string(&remote).unwrap();
        assert!(json.contains(r#""name":"my_drive""#));
        assert!(json.contains(r#""type":"drive""#));
    }

    #[test]
    fn test_config_list_binary_not_found() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let bad_path = Path::new("/nonexistent/rclone");
        let result = rt.block_on(config_list(bad_path));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to execute") || err.contains("No such file"),
            "Expected IO error, got: {err}"
        );
    }

    #[test]
    fn test_config_list_parse_valid_json() {
        // Simulate rclone config dump output with valid JSON
        let json = r#"{
            "remote1": { "type": "drive", "client_id": "secret" },
            "remote2": { "type": "s3", "secret_key": "hidden" }
        }"#;

        let remotes: Vec<Remote> = {
            let map: HashMap<String, HashMap<String, serde_json::Value>> =
                serde_json::from_str(json).unwrap();
            let mut result: Vec<Remote> = map
                .into_iter()
                .map(|(name, mut props)| {
                    let remote_type = props
                        .remove("type")
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    Remote { name, remote_type }
                })
                .collect();
            result.sort_by(|a, b| a.name.cmp(&b.name));
            result
        };

        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "remote1");
        assert_eq!(remotes[0].remote_type, "drive");
        assert_eq!(remotes[1].name, "remote2");
        assert_eq!(remotes[1].remote_type, "s3");
    }

    #[test]
    fn test_config_list_empty_config() {
        let json = "{}";
        let map: HashMap<String, HashMap<String, serde_json::Value>> =
            serde_json::from_str(json).unwrap();

        assert!(map.is_empty());
    }

    #[test]
    fn test_config_list_strips_sensitive_fields() {
        let json = r#"{
            "gdrive": {
                "type": "drive",
                "token": "{\"access_token\":\"secret\"}",
                "client_id": "123.apps.googleusercontent.com",
                "client_secret": "my_secret",
                "scope": "drive.readonly"
            }
        }"#;

        let remotes: Vec<Remote> = {
            let map: HashMap<String, HashMap<String, serde_json::Value>> =
                serde_json::from_str(json).unwrap();
            let mut result: Vec<Remote> = map
                .into_iter()
                .map(|(name, mut props)| {
                    let remote_type = props
                        .remove("type")
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    Remote { name, remote_type }
                })
                .collect();
            result.sort_by(|a, b| a.name.cmp(&b.name));
            result
        };

        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "gdrive");
        assert_eq!(remotes[0].remote_type, "drive");
        // The serialized Remote should NOT contain sensitive fields
        let serialized = serde_json::to_string(&remotes[0]).unwrap();
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("client_secret"));
        assert!(!serialized.contains("access_token"));
        assert!(!serialized.contains("client_id"));
    }
}

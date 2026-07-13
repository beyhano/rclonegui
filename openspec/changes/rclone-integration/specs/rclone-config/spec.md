# Rclone Config Specification

## Purpose

List configured rclone remotes by running `rclone config dump`, parsing the JSON output, and returning a structured list to the frontend via a Tauri command.

## Requirements

### Requirement: Config Dump Command

The system MUST implement a Tauri command `rclone_config_list` that spawns `rclone config dump` synchronously (short-lived process), captures stdout, and parses it as JSON.

#### Scenario: Remotes listed successfully

- GIVEN the rclone binary is found and `rclone config dump` returns valid JSON with keys `"gdrive"` and `"dropbox"`
- WHEN `rclone_config_list` is invoked
- THEN it MUST return a `Vec<Remote>` where each `Remote` has `name: "gdrive"` and `name: "dropbox"`
- AND the `type` field from the config MUST be present on each entry

#### Scenario: No remotes configured

- GIVEN `rclone config dump` returns `{}` (empty object)
- WHEN `rclone_config_list` is invoked
- THEN it MUST return an empty `Vec`

### Requirement: Config Dump Failure

If `rclone config dump` fails (non-zero exit), the system MUST return a descriptive error to the frontend including the stderr content.

#### Scenario: Config command fails

- GIVEN the rclone binary is found but config dump exits with code `1` and stderr: `"Failed to open config file"`
- WHEN `rclone_config_list` is invoked
- THEN the command MUST return `Err("Failed to list remotes: Failed to open config file")`

### Requirement: JSON Parse Error

If `rclone config dump` succeeds but returns malformed JSON, the system MUST return a parse error.

#### Scenario: Malformed JSON

- GIVEN stdout is `{invalid json`
- WHEN the parser attempts `serde_json::from_str`
- THEN the command MUST return `Err` with a message containing `"JSON parse error"`

### Requirement: Remote Structure

Each remote in the returned list MUST contain at least `name: String` and `type: String`. Additional fields (e.g. `token`, `client_id`) SHOULD be omitted for security unless explicitly requested.

#### Scenario: Sensitive fields excluded

- GIVEN the config dump includes `"token": "secret123"` for a remote
- WHEN the remote is serialized for the frontend
- THEN the `token` field MUST NOT be present in the response

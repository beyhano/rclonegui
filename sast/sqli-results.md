# SQLi Analysis Results: RcloneGUI

## Executive Summary
- Construction sites analyzed: 1
- Vulnerable: 0
- Likely Vulnerable: 0
- Not Vulnerable: 1
- Needs Manual Review: 0

## Findings

### [NOT VULNERABLE] Dynamic table name via format! in test helper
- **File**: `C:\Users\Beyhan\Desktop\Projeler\Rust\rclonegui\src-tauri\src\db\migrations.rs` (line 160)
- **Endpoint / function**: `test_create_tables_all_four_tables_exist_via_pragma` (test function, `#[cfg(test)]`)
- **Reason**: The interpolated variable `table` is sourced from a compile-time hardcoded list `&["transfers", "mounts", "app_config", "tasks"]` defined on line 158 within the same test function. This is a static constant list — no user input path exists. Additionally, the entire function is gated behind `#[cfg(test)]`, meaning it is compiled only during `cargo test` and never included in production binaries.

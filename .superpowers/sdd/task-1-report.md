# Task 1 Report: Add cron dependency + DB migration for tasks table + task_id to transfers

## What was implemented

1. **Added `cron = "0.15"` dependency** to `src-tauri/Cargo.toml` (and auto-updated `Cargo.lock`)
2. **Created the `tasks` table** in `src-tauri/src/db/migrations.rs` with 13 columns: `id`, `name`, `slug` (UNIQUE), `source_provider`, `source_config`, `dest_provider`, `dest_config`, `operation`, `exclude_patterns`, `cron_expr`, `enabled`, `created_at`, `updated_at`
3. **Added `task_id` column to `transfers`** using the safe SQLite rename/swap pattern: create `transfers_v2` with all original columns + `task_id`, copy data, drop original, rename

## Tests added/updated

- `test_create_tables_creates_tasks_table` — verifies tasks table exists after `create_tables()`
- `test_tasks_table_has_expected_columns` — verifies id, slug, cron_expr, enabled columns exist
- `test_create_tables_all_four_tables_exist_via_pragma` — updated from 3 to 4 tables
- `test_create_tables_is_idempotent` — updated from 3 to 4 table count
- `test_transfers_table_has_expected_schema` — updated from 9 to 10 columns

## Test results

All 8 migration tests pass (3 original + 5 new/updated = 8 total). Full suite: 55 pass, 4 fail (pre-existing process/mount test failures on Windows — "echo: program not found").

## Files changed

- `src-tauri/Cargo.toml` — added `cron = "0.15"`
- `src-tauri/Cargo.lock` — auto-updated
- `src-tauri/src/db/migrations.rs` — added tasks table DDL, transfers_v2 migration, new tests

## Self-review

- **Schema correctness**: tasks table has all required columns with correct types and constraints. Transfers gains `task_id` TEXT column (nullable, no FK constraint — FKs will be enforced at app level per the plan)
- **Idempotency**: `CREATE TABLE IF NOT EXISTS` for tasks; the transfers_v2 rename/swap is also idempotent since it re-creates the intermediate table on each call
- **Migration safety**: The rename/swap pattern is standard for SQLite column additions — data is preserved through INSERT OR IGNORE + RENAME
- **Edge case**: On first run (empty DB), the transfers_v2 is created, copies 0 rows, drops empty transfers, renames — harmless
- **No concerns**

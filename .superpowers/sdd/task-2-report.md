# Task 2 Report: Task data model + TaskRepo CRUD

## What I Implemented

Created the `Task` struct and `TaskRepo` CRUD at `src-tauri/src/db/task_repo.rs`:

- **`Task` struct** — matches the `tasks` SQL schema exactly, with JSON serialization for `source_config`, `dest_config` (both `serde_json::Value`), and `exclude_patterns` (`Vec<String>`). Fields: `id`, `name`, `slug`, `source_provider`, `source_config`, `dest_provider`, `dest_config`, `operation`, `exclude_patterns`, `cron_expr`, `enabled`, `created_at`, `updated_at`.

- **`TaskRepo` struct** — owns `rusqlite::Connection`, provides:
  - `new(conn)` — constructor
  - `connection()` — expose inner connection reference
  - `list()` — `SELECT * FROM tasks ORDER BY created_at DESC`
  - `get_by_id(id)` — lookup by primary key
  - `get_by_slug(slug)` — lookup by unique slug
  - `create(task)` — INSERT with JSON serialization for config/patterns
  - `update(task)` — UPDATE preserving original `created_at`, updating `updated_at`
  - `delete(id)` — DELETE by primary key
  - `get_enabled()` — `WHERE enabled = 1`

- **`db/mod.rs`** — added `pub mod task_repo;` registration

- **Helper** — `map_row()` handles the 13-column row-to-Task mapping including JSON deserialization and bool/i32 conversion

## Test Results

**Command:** `cargo test task_repo -- --nocapture`

**Result:** 12/12 tests PASSED in 0.02s

| Test | Status |
|---|---|
| `test_create_and_list` | ✅ |
| `test_empty_list` | ✅ |
| `test_get_by_id` | ✅ (verifies all fields including JSON round-trip) |
| `test_get_by_id_not_found` | ✅ |
| `test_get_by_slug` | ✅ |
| `test_get_by_slug_not_found` | ✅ |
| `test_update` | ✅ (preserves created_at, changes updated_at) |
| `test_delete` | ✅ |
| `test_get_enabled` | ✅ (1 enabled + 1 disabled returns only enabled) |
| `test_list_ordered_by_created_at_desc` | ✅ |
| `test_duplicate_slug_errors` | ✅ (UNIQUE constraint enforced) |
| `test_json_fields_round_trip_empty` | ✅ (null JSON, empty vec round-trip) |

## Files Changed

- **Created:** `src-tauri/src/db/task_repo.rs` — ~195 lines (Task struct + TaskRepo + 12 tests)
- **Modified:** `src-tauri/src/db/mod.rs` — added `pub mod task_repo;` (1 line)

## Self-Review Findings

- **Design decisions**: TaskRepo owns the Connection rather than borrowing, matching the brief. JSON fields handled via `serde_json::to_string/from_str` with `unwrap_or_default()` fallback on parse failure.
- **Edge cases covered**: Not-found queries return None (not error), empty list returns empty vec, JSON null/empty arrays round-trip correctly, duplicate slugs hit the DB constraint.
- **Code style**: Consistent with existing `models.rs` pattern (query_map, params!, explicit row field indexing).
- **No regressions**: All existing tests (migrations, models) continue to pass.
- **Dead code warnings**: Pre-existing `models.rs` structs (`Transfer`, `Mount`, `AppConfig`) still show as dead code — same as before, not caused by this change. The new `connection()` method on TaskRepo also shows as unused; it's there for future consumers.

## Commits

`feat(db): add Task model and TaskRepo CRUD`

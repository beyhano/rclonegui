# Tasks: Rclone Integration

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1100–1200 |
| 800-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception accepted) |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Focused test command | Runtime harness | Rollback boundary |
|------|------|---------------------|-----------------|-------------------|
| All | Single PR — whole rclone integration | `cargo test` + `pnpm lint` | `cargo tauri dev` | Revert all files in `src-tauri/src/`, `src/`, revert `Cargo.toml` |

## Phase 1: Foundation

- [x] 1.1 Add deps via `cargo add` in `src-tauri/`: tokio (process, io-util, sync), regex, uuid (v4), chrono (serde), rusqlite (bundled)
- [x] 1.2 Create `src-tauri/src/state.rs` — `AppState` struct with `processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>`, `rclone_path: Arc<Mutex<Option<PathBuf>>>`, `db: Arc<Mutex<Connection>>`
- [x] 1.3 Create `src-tauri/src/rclone/mod.rs` — re-export all sub-modules
- [x] 1.4 Create `src-tauri/src/db/mod.rs` — re-export models, migrations
- [x] 1.5 Create `src-tauri/src/commands/mod.rs` — re-export rclone_cmds
- [x] 1.6 Add `#![deny(unsafe_code)]` to `src-tauri/src/main.rs`

## Phase 2: Core Backend — Discovery, Process, Config

- [x] 2.1 Create `rclone/discovery.rs` — `resolve_platform()`, `locate_binary()`, `verify_executable()`, `find_binary()` multi-path search
- [x] 2.2 Create `rclone/process.rs` — `ProcessManager::spawn()`, `stop()`, `cleanup_all()` with `tokio::process::Command`, `kill_on_drop(true)`, std `Mutex<HashMap<Uuid, ProcessHandle>>`
- [x] 2.3 Create `rclone/config.rs` — `config_list()`: spawn `rclone config dump`, parse JSON `HashMap<String, Value>`, map to `Vec<Remote>` (name + type only, strip sensitive fields)
- [x] 2.4 RED test: write unit tests for `resolve_platform()` — assert `linux-amd64` / `windows-amd64` from cfg markers

## Phase 3: SQLite DB + Event Pipeline

- [x] 3.1 Create `db/migrations.rs` — `create_tables()` with 3 CREATE TABLE IF NOT EXISTS (transfers, mounts, app_config)
- [x] 3.2 Create `db/models.rs` — `Transfer`, `Mount`, `AppConfig` structs + basic CRUD impls
- [x] 3.3 Create `rclone/events.rs` — `start_event_stream()`: BufReader stdout/stderr, regex compile for `Transferred: …%`, emit `rclone:progress`, `rclone:log`, event stream
- [x] 3.4 RED test: unit tests for `parse_progress_line()` — 9 fixture line tests, all passing
- [x] 3.5 RED test: unit test for `db::migrations::create_tables()` — open `:memory:` SQLite, run migrations, assert all 3 tables exist

## Fixed: Binary Discovery Path Fix

- [x] F.1 Fix `lib.rs` — use `find_binary()` fallback chain (resource_dir → cargo_manifest → cwd → exe ancestors)
- [x] F.2 Add `tauri.conf.json` bundle resources for rclone-bin production packaging
- [x] F.3 Update wiki docs with search strategy

## Phase 4: Tauri Commands + Wiring

- [x] 4.1 Create `commands/rclone_cmds.rs` — 7 `#[tauri::command]` fns: `rclone_version`, `rclone_config_list`, `rclone_exec`, `rclone_stop`, `rclone_mount`, `rclone_unmount`, `rclone_mount_list`
- [x] 4.2 Modify `lib.rs` — remove greet, register modules (rclone, db, commands, state), register state with setup(), register all 7 commands in invoke_handler, add `#![deny(unsafe_code)]`, attach cleanup on Exit
- [x] 4.3 No changes needed — `capabilities/default.json` already has `core:default` + `opener:default` which is sufficient for Tauri 2 command invoke
- [x] 4.4 RED test: wrote 4 tests — `test_get_rclone_path_none_returns_error`, `test_get_rclone_path_with_value`, `test_mount_list_empty_when_no_mounts`, `test_mount_list_returns_stored_mounts`

## Phase 5: Frontend

- [x] 5.1 Create `src/types.ts` — `Remote`, `TransferRecord`, `MountRecord`, `ProgressPayload` interfaces
- [x] 5.2 Create `src/ConfigPanel.tsx` — remote list with type badges, invoke `rclone_config_list` on mount
- [x] 5.3 Create `src/TransferPanel.tsx` — source/dest selectors, progress bar, speed, stop button, history table
- [x] 5.4 Create `src/MountPanel.tsx` — mount list, mount/unmount buttons, status indicators, invoke `rclone_mount_list`
- [x] 5.5 Modify `App.tsx` — replace greet form with tab router (Config / Transfer / Mounts panels)
- [x] 5.6 Update `App.css` — add styles for tab layout, progress bar, status badges

## Phase 6: Testing + Verification

- [x] 6.1 Add integration test: spawn `echo` via ProcessManager, assert PID tracked, stop, assert cleanup
- [x] 6.2 Verify spec scenarios: `resolve_platform()` returns valid string, binary-not-found returns error (not panic)
- [x] 6.3 Verify event stream: 9 existing parse_progress_line tests pass (confirmed)
- [x] 6.4 Verify mount lifecycle: mount started → Running in state → unmount → Released
- [x] 6.5 Run `cargo build` and `pnpm build` and `cargo clippy` — zero errors with `#![deny(unsafe_code)]`

## Phase 7: Wiki Documentation

- [x] 7.1 Update `docs/wiki/Architecture_Overview.md` — promote planned layers (Process, Event, Integration) to active with module paths
- [x] 7.2 Create or update `docs/wiki/Rclone_Integration.md` — document all 7 commands, 6 events, module structure
- [x] 7.3 Update `docs/wiki/Index.md` — reflect new modules, commands, and frontend panels

## Total: 28 tasks across 7 phases

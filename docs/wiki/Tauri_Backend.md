# Tauri_Backend

**Özet:** Rust ile yazılmış Tauri v2 backend katmanı. 17 Tauri komutunu (`#[tauri::command]`) tanımlar, process yönetimini, SQLite veritabanını, cron-tabanlı görev zamanlayıcıyı, sistem tepsisini ve diyalog pencerelerini bu katmanda barındırır.

**Kütüphaneler:** tauri 2, serde 1, serde_json 1, tauri-plugin-opener 2, tauri-plugin-dialog 2, tokio, chrono, uuid, cron

**Bağlantılar:** [[Project_Overview]], [[React_Frontend]], [[Rclone_Integration]], [[Process_Manager]], [[State_Management]], [[Event_Stream]]

---

## Mimari Katman

```rust
// src-tauri/src/main.rs  →  giriş noktası (windows_subsystem)
// src-tauri/src/lib.rs   →  Tauri builder, komutlar, plugin'ler, tray, cleanup
```

## Mevcut Yapı

```
src-tauri/src/
21: src-tauri/src/
22: ├── main.rs                 → #![deny(unsafe_code)], windows_subsystem, giriş
23: ├── lib.rs                  → Tauri builder, setup, 17 komut, tray, cleanup, dialog eklentisi
24: ├── state.rs                → AppState (processes, db, mounts, rclone_path, task_repo, scheduler, task_pids)
25: ├── tray.rs                 → System tray icon (Show Window / Quit + left-click)
26: ├── commands/
27: │   ├── mod.rs              → pub mod rclone_cmds, task_cmds
28: │   ├── rclone_cmds.rs      → 9 #[tauri::command] — rclone işlemleri (klasör listeleme dahil)
29: │   └── task_cmds.rs        → 8 #[tauri::command] — görev CRUD + scheduler
30: ├── rclone/
31: │   ├── discovery.rs        → Platform tespiti, binary bulma
32: │   ├── process.rs          → ProcessManager (spawn, stop, cleanup_all)
33: │   ├── events.rs           → Regex parser + event emit pipeline
34: │   └── config.rs           → rclone config dump JSON parse
35: ├── scheduler/
36: │   ├── cron.rs             → next_cron_time() — cron ayrıştırma
37: │   ├── engine.rs           → execute_task() — rclone spawn + progress/yakalama
38: │   └── scheduler.rs        → TaskScheduler — cron döngüleri
39: └── db/
40:     ├── migrations.rs       → create_tables (transfers, mounts, app_config, tasks)
41:     └── task_repo.rs        → Task modeli + CRUD
```

- **`main.rs`**: `#![deny(unsafe_code)]` + `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
- **`lib.rs`**: `tauri::Builder::default()` ile 17 komut kaydı, SQLite init, binary discovery, TaskScheduler başlatma, tray ikonu, `tauri-plugin-dialog` ve `tauri-plugin-opener` yüklemesi, CloseRequested ile pencere gizleme (tray) ve Exit ile genel temizlik
- **`state.rs`**: `AppState` — `processes`, `rclone_path`, `mounts`, `task_repo`, `scheduler`, `task_pids`
- **`tray.rs`**: System tray — "Show Window" ve "Quit" menüsü, sol tıkla pencere açma

## Tauri Yapılandırması

- **App identifier**: `com.beyhan.rclonegui`
- **Window**: 800×600, başlık "rclonegui"
- **CSP**: `null` (dev mode)
- **Permissions**: `core:default`, `opener:default`, `dialog:default` (yerel klasör seçimi için)
- **Bundle resources**: `rclone-bin/{platform}/rclone`(.exe) paketlenir

## Komut Listesi (17 adet)

### rclone işlemleri (`commands/rclone_cmds.rs`)

| Komut | İşlev |
|---|---|
| `rclone_version` | Binary versiyonu |
| `rclone_config_list` | Remote listesi (config dump) |
| `rclone_config_create` | Yeni remote oluşturma (`--non-interactive`) |
| `rclone_exec` | Rclone çalıştır + event stream |
| `rclone_stop` | Process UUID ile durdur |
| `rclone_mount` | Remote mount et |
| `rclone_unmount` | Mount'ı çöz |
| `rclone_mount_list` | Aktif mount'ları listele |
| `rclone_list_dirs` | Dizin listele (yerel/uzak) `rclone lsf --dirs-only` |

### Task/Scheduler işlemleri (`commands/task_cmds.rs`)

| Komut | İşlev |
|---|---|
| `task_list` | Tüm görevleri listele |
| `task_create` | Yeni cron görevi oluştur |
| `task_update` | Görev parametrelerini güncelle |
| `task_delete` | Görevi sil |
| `task_toggle` | Görevi etkinleştir/devre dışı bırak |
| `task_run_now` | Görevi schedule dışında hemen çalıştır |
| `task_stop` | Çalışan görev process'ini PID ile öldür |
| `rclone_providers` | `rclone config providers` JSON çıktısı |

## Setup Akışı

```
Tauri Builder
  └── plugin(opener)
  └── plugin(dialog)
  └── invoke_handler(17 komut)
  └── setup()
      ├── SQLite: app_data_dir/rclonegui.db → migrations
      ├── Binary: resource_dir → locate_binary() → find_binary()
      ├── TaskRepo: ikinci SQLite bağlantısı → Arc<Mutex<TaskRepo>>
      ├── TaskScheduler: rclone_path + task_repo + app.handle()
      ├── AppState: task_repo + scheduler
      ├── tray::build_tray()
      └── scheduler.start() — 500ms gecikmeli async spawn
```

## Cleanup ve Pencere Yönetim Akışı

```
CloseRequested → api.prevent_close() → window.hide() (Arka planda çalışmaya devam eder)

Tray Menüsü "Quit" / app.exit(0) → Exit Event Tetiklenir:
  1. ProcessManager.cleanup_all() — state.processes'teki tüm child'ları öldür
  2. task_pids'teki tüm PID'leri taskkill/kill -9 ile öldür
  3. scheduler.stop() — cancel_tokens ile tüm cron döngülerini durdur
```

## Güvenlik

- `#![deny(unsafe_code)]` tüm crate'lerde aktif
- `capabilities/default.json` ile izin yönetimi (`opener` ve `dialog` izinleri sınırlandırılmıştır)
- Binary yolu `find_binary()` ile çoklu katmanda güvenli aranır
- Tüm `rusqlite` sorguları `params!` ile parametrize

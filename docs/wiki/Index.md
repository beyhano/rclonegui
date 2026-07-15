# RcloneGUI — Mimarî Bilgi Grafiği

**Proje:** Tauri v2 + Rust + React masaüstü uygulaması — rclone binary'si için grafiksel arayüz
**Platform:** Windows, Linux & macOS (çapraz platform)
**Durum:** Rclone entegrasyonu + cron-tabanlı görev zamanlayıcı tamamlandı — tüm katmanlar aktif

---

## 🧱 Aktif Bileşenler (Mevcut Kod Tabanı)

| Düğüm | Açıklama | Durum |
|---|---|---|
| [[Project_Overview]] | Proje vizyonu, hedefler ve kısıtlamalar | ✅ Aktif |
| [[Tauri_Backend]] | Rust backend — Tauri v2 komutları, state, process yönetimi | ✅ Aktif |
| [[Rclone_Integration]] | rclone binary keşfi, versiyon kontrolü, config yönetimi | ✅ Aktif |
| [[Process_Manager]] | Async süreç yaşam döngüsü (spawn, izleme, temiz sonlandırma) | ✅ Aktif |
| [[State_Management]] | Tauri State ile çalışan süreçlerin PID, mount, task_repo, scheduler ve task_pids yönetimi | ✅ Aktif |
| [[Event_Stream]] | stdout/stderr ayrıştırma ve frontend'e gerçek zamanlı emit | ✅ Aktif |
| [[React_Frontend]] | React 19 + TypeScript UI katmanı (4 panel) | ✅ Aktif |
| [[Build_Config]] | Vite, pnpm, Cargo derleme araç zinciri | ✅ Aktif |
| [[Architecture_Overview]] | Tüm sistemin katmanlı mimarî şeması | ✅ Aktif |

## Görev Zamanlayıcı (Yeni)

| Modül | Dosya | İşlev |
|---|---|---|
| Task CRUD | `commands/task_cmds.rs` | 8 Tauri komutu — task_list/create/update/delete/toggle/run_now/stop/running_list/rclone_providers |
| Cron | `scheduler/cron.rs` | `next_cron_time()` — cron ifadesi ayrıştırma |
| Engine | `scheduler/engine.rs` | `execute_task()` — rclone spawn, progress, PID takibi |
| Scheduler | `scheduler/scheduler.rs` | `TaskScheduler` — cron döngüleri, start/stop/add/remove/update/run_now |
| Tray | `tray.rs` | System tray ikonu (Show Window / Quit) |
| Task DB | `db/task_repo.rs` | Task modeli + CRUD (tasks tablosu) |

## 📦 Frontend Panelleri

| Panel | Dosya | İşlev |
|---|---|---|
| Config | `src/ConfigPanel.tsx` | Remote listeleme, tür badge'leri |
| Transfer | `src/TransferPanel.tsx` | Copy/sync başlatma, progress bar, hız, ETA, geçmiş |
| Mounts | `src/MountPanel.tsx` | Mount bağlama/çözme, durum göstergeleri |
| Scheduler | `src/components/SchedulerPage.tsx` | Görev zamanlama, listeleme, manuel tetikleme, durdurma, remote klasör tarayıcı |

## ⚙️ Tauri Komutları (17 adet)

### rclone işlemleri (`commands/rclone_cmds.rs`)

| Komut | İşlev |
|---|---|
| `rclone_version` | `rclone version` çıktısını döner |
| `rclone_config_list` | Yapılandırılmış remote'ları listeler |
| `rclone_config_create` | `rclone config create --non-interactive` ile yeni remote ekler |
| `rclone_exec` | Rclone süreci başlatır (copy/sync) |
| `rclone_stop` | Süreci UUID ile durdurur |
| `rclone_mount` | Remote dosya sistemi bağlar |
| `rclone_unmount` | Mount'ı UUID ile çözer |
| `rclone_mount_list` | Aktif mount'ları listeler |
| `rclone_list_dirs` | Dizin yapısını listeler (yerel veya uzak sunucu) |

### Task/Scheduler işlemleri (`commands/task_cmds.rs`)

| Komut | İşlev |
|---|---|
| `task_list` | Tüm görevleri listeler |
| `task_create` | Yeni cron görevi oluşturur |
| `task_update` | Görev parametrelerini günceller |
| `task_delete` | Görevi siler |
| `task_toggle` | Görevi etkinleştirir/devre dışı bırakır |
| `task_run_now` | Görevi schedule dışında hemen çalıştırır |
| `task_stop` | Çalışan görev process'ini PID ile öldürür |
| `rclone_providers` | `rclone config providers` JSON çıktısını döner |

## 🗄️ SQLite Veritabanı

`src-tauri/src/db/` içinde:

| Tablo | Açıklama |
|---|---|
| `transfers` | Copy/sync işlem geçmişi ve durumu (task_id ile scheduler kayıtları) |
| `mounts` | Mount süreç kayıtları |
| `app_config` | Key-value uygulama ayarları |
| `tasks` | Cron görev tanımları (13 kolon: cron_expr, exclude_patterns, operation, vs.) |

## 📁 Backend Modül Yapısı

```
src-tauri/src/
├── main.rs
├── lib.rs              ← Builder, setup, 17 komut, tray, cleanup, dialog
├── state.rs            ← AppState (processes, rclone_path, mounts, task_repo, scheduler, task_pids)
├── tray.rs             ← System tray
├── commands/
│   ├── mod.rs
│   ├── rclone_cmds.rs  ← 9 Tauri komutu
│   └── task_cmds.rs    ← 8 Tauri komutu
├── rclone/
│   ├── mod.rs
│   ├── discovery.rs    ← Platform algılama, binary keşfi
│   ├── process.rs      ← ProcessManager
│   ├── events.rs       ← Event pipeline, regex
│   └── config.rs       ← Config dump, Remote modeli
├── scheduler/
│   ├── mod.rs
│   ├── cron.rs         ← next_cron_time()
│   ├── engine.rs       ← execute_task()
│   └── scheduler.rs    ← TaskScheduler
└── db/
    ├── mod.rs
    ├── migrations.rs   ← create_tables
    └── task_repo.rs    ← Task CRUD
```

---

## 🔗 Bağlantı Haritası

```
[[Project_Overview]]
├── [[Build_Config]]
├── [[Tauri_Backend]]
│   ├── [[Rclone_Integration]]
│   │   ├── rclone/discovery.rs
│   │   └── rclone/config.rs
│   ├── [[Process_Manager]]
│   │   └── rclone/process.rs
│   ├── [[Scheduler]] (yeni)
│   │   ├── scheduler/cron.rs
│   │   ├── scheduler/engine.rs
│   │   └── scheduler/scheduler.rs
│   ├── [[State_Management]]
│   │   └── state.rs
│   ├── [[Event_Stream]]
│   │   └── rclone/events.rs
│   ├── [[Tray]]
│   │   └── tray.rs
│   └── Database
│       ├── db/migrations.rs
│       └── db/task_repo.rs
├── [[React_Frontend]]
│   └── Architecture_Overview
├── Commands
│   ├── commands/rclone_cmds.rs
│   └── commands/task_cmds.rs
└── Task Scheduler
    └── commands/task_cmds.rs + scheduler/
```

---

## 📐 Tasarım İlkeleri

- **Sıfır `unsafe`**: Tüm Rust kodu `#![deny(unsafe_code)]` ile güvence altında
- **Sadece Cargo**: Sistem kütüphanesi bağımlılığı yok, pure-Rust crate'ler tercih edilir
- **Çapraz platform**: `std::path::PathBuf`, platform-agnostik sinyal yönetimi
- **Event-driven**: Uzun süren işlemler frontend'e Tauri event'leri ile iletilir
- **TDD**: Strict TDD mode aktif — her değişiklik testlerle doğrulanır
- **Parametrize SQL**: Tüm `rusqlite` sorguları `params!` ile injection korumalı

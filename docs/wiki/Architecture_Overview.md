# Architecture_Overview

**Özet:** RcloneGUI uygulamasının tam katmanlı mimarî şeması. Tauri v2'nin iki-proses mimarisi (Rust backend + WebView frontend) üzerine kurulmuştur. Tüm rclone entegrasyon katmanları ve cron-tabanlı görev zamanlayıcı aktiftir.

**Kütüphaneler:** Tauri v2, Rust, React 19, TypeScript, Vite 7, tokio, serde, regex, rusqlite, chrono, uuid, cron

**Bağlantılar:** [[Project_Overview]], [[Tauri_Backend]], [[React_Frontend]], [[Rclone_Integration]], [[Process_Manager]], [[State_Management]], [[Event_Stream]]

---

## Mimarî Şema

```mermaid
graph TB
    subgraph "Process Boundary"
        subgraph "Rust Backend (Tauri Core)"
            LIB[lib.rs<br/>Tauri Builder & Setup]
            CMD_RC[commands/rclone_cmds.rs<br/>8 rclone komutu]
            CMD_TK[commands/task_cmds.rs<br/>8 task komutu]
            ST[State Management<br/>state.rs]
            PM[Process Manager<br/>rclone/process.rs]
            EP[Event Pipeline<br/>rclone/events.rs]
            CFG[Config Management<br/>rclone/config.rs]
            DSC[Binary Discovery<br/>rclone/discovery.rs]
            DB[SQLite Persistence<br/>db/]
            SCHED[scheduler/<br/>cron.rs, engine.rs, scheduler.rs]
            TRAY[tray.rs<br/>System Tray]
            
            LIB --> CMD_RC
            LIB --> CMD_TK
            LIB --> ST
            LIB --> DB
            LIB --> SCHED
            LIB --> TRAY
            CMD_RC --> DSC
            CMD_RC --> CFG
            CMD_RC --> PM
            CMD_RC --> EP
            CMD_TK --> SCHED
            CMD_TK --> ST
            SCHED --> EP
            PM --> EP
            EP -->|app_handle.emit| FE
        end
        
        subgraph "WebView Frontend"
            FE[App.tsx<br/>Tab Router]
            CP[ConfigPanel.tsx<br/>Remote List]
            TP[TransferPanel.tsx<br/>Progress + History]
            MP[MountPanel.tsx<br/>Mount Management]
            TY[types.ts<br/>TypeScript Interfaces]
            
            FE --> CP
            FE --> TP
            FE --> MP
        end
        
        subgraph "Persistence"
            SQL[(SQLite<br/>rclonegui.db)]
            SQLT[transfers tablosu]
            SQLM[mounts tablosu]
            SQLC[app_config tablosu]
            SQL --> SQLT
            SQL --> SQLM
            SQL --> SQLC
        end
        
        subgraph "External"
            RCLONE[rclone binary<br/>rclone-bin/{platform}/]
            FS[Dosya Sistemi<br/>mount noktaları]
        end
        
        PM -->|stdout/stderr| RCLONE
        PM -->|stdin/process control| RCLONE
        SCHED -->|execute_task| RCLONE
        RCLONE -->|copy/sync| FS
        RCLONE -->|mount| FS
        DB --> SQL
    end
    
    CMD_RC -->|invoke| FE
    CMD_TK -->|invoke| FE
    FE -->|invoke| CMD_RC
    FE -->|invoke| CMD_TK
```

## Katmanlar

### 1. Sunum Katmanı (Frontend — React 19)

```
src/
├── App.tsx              ← Ana bileşen, tab yönlendirici
├── ConfigPanel.tsx      ← Remote listeleme paneli
├── TransferPanel.tsx    ← Dosya transfer + ilerleme çubuğu
├── MountPanel.tsx       ← Mount yönetim paneli
├── types.ts             ← TypeScript arayüz tanımları
├── main.tsx             ← ReactDOM giriş
├── App.css              ← Global stiller (tab, progress bar, badge)
```

- **Sorumluluk**: Kullanıcı arayüzü, event dinleme (`listen()`), backend komut çağrısı (`invoke()`)
- **İletişim**: `invoke()` ile Tauri komutları, `listen()` ile event'ler
- **Panel**: Config → Remote listesi, Transfer → Kopyalama/sync işlemleri, Mounts → Bağlama yönetimi
- **Bağlantı**: [[React_Frontend]]

### 2. Uygulama Katmanı (Backend — Rust)

```
src-tauri/src/
├── main.rs              ← Windows subsystem, giriş
├── lib.rs               ← Tauri builder, setup, komut kaydı, setup, tray, cleanup
├── state.rs             ← AppState (processes, rclone_path, db, mounts, task_repo, scheduler, task_pids)
├── tray.rs              ← System tray ikonu (minimize to tray)
├── commands/
│   ├── mod.rs           ← Re-export
│   ├── rclone_cmds.rs   ← 8 adet #[tauri::command] (rclone_version .. rclone_config_create)
│   └── task_cmds.rs     ← 8 adet #[tauri::command] (task_list .. rclone_providers)
├── rclone/
│   ├── mod.rs           ← Re-export
│   ├── discovery.rs     ← Platform algılama, binary keşfi, doğrulama
│   ├── process.rs       ← ProcessManager: spawn, stop, cleanup_all
│   ├── events.rs        ← Event stream, progress regex, emit
│   └── config.rs        ← rclone config dump → Remote listesi
├── scheduler/
│   ├── mod.rs           ← Re-export
│   ├── cron.rs          ← next_cron_time() — cron ayrıştırma
│   ├── engine.rs        ← execute_task() — rclone spawn + progress/yakalama
│   └── scheduler.rs     ← TaskScheduler — cron döngüleri, start/stop/add/remove
└── db/
    ├── mod.rs           ← Re-export
    ├── migrations.rs    ← create_tables (transfers, mounts, app_config + tasks)
    └── task_repo.rs     ← Task varlığı + CRUD
```

- **Sorumluluk**: Tauri komutlarını tanımlama, state yönetimi, process kontrolü, event yayını, cron zamanlayıcı

### 3. Scheduler Katmanı (Aktif/Gerçekleşen)

**Dosyalar**: `src-tauri/src/scheduler/{cron.rs, engine.rs, scheduler.rs}`

- **cron.rs**: `cron::Schedule` parse eder, `next_cron_time()` ile bir sonraki UTC zamanı hesaplar
- **engine.rs**: `execute_task()` — rclone binary'sini `{operation, source, dest, --exclude, --progress}` argümanlarıyla spawn eder, stdout/stderr'den progress/log event'leri emit eder, PID takibi yapar, `TaskResult` döner
- **scheduler.rs**: `TaskScheduler` — `start()` tüm enabled task'lar için cron döngüsü başlatır, `add_task/remove_task/update_task` ile canlı güncelleme, `run_now()` ile elle tetikleme, `stop()` ile temiz kapanış, overlap koruması

### 4. Process Katmanı (Aktif/Gerçekleşen)

**Dosya**: `src-tauri/src/rclone/process.rs`

- **Sorumluluk**: Rclone süreçlerini spawn etme (`tokio::process::Command`), izleme, sonlandırma (`kill_on_drop(true)`)
- **ProcessManager**: `spawn()` → UUID döner, `stop()` → child.kill, `cleanup_all()` → Exit handler
- **Bağlantı**: [[Process_Manager]]

### 5. Event Katmanı (Aktif/Gerçekleşen)

**Dosya**: `src-tauri/src/rclone/events.rs`

- **Sorumluluk**: rclone stdout/stderr çıktısını `BufReader` ile oku, regex ile ayrıştır, Tauri `emit()` ile frontend'e ilet
- **Regex**: `Transferred: 1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s`
- **Event'ler**: `rclone:progress`, `rclone:log`, `rclone:process-started`, `rclone:process-completed`, `rclone:mount-status`
- **Bağlantı**: [[Event_Stream]]

### 6. Entegrasyon Katmanı (Aktif/Gerçekleşen)

**Dosyalar**: `src-tauri/src/rclone/discovery.rs`, `src-tauri/src/rclone/config.rs`

- **Binary Keşfi**: `resolve_platform()` → platform belirle, `locate_binary()` → `rclone-bin/{platform}/rclone`
- **Config**: `config_list()` → `rclone config dump` JSON çıktısını parse et, `Vec<Remote>` dön
- **Bağlantı**: [[Rclone_Integration]]

### 7. Veritabanı Katmanı

**Dosyalar**: `src-tauri/src/db/migrations.rs`, `src-tauri/src/db/task_repo.rs`

- **SQLite**: `rusqlite` bundled, otomatik oluşturulur (`app_data_dir/rclonegui.db`)
- **Tablo**: `transfers` (9 kolon), `mounts` (6 kolon), `app_config` (key-value), `tasks` (13 kolon — cron ifadesi, exclude pattern, vs.)
- **CRUD**: `TaskRepo` — `list()`, `get_enabled()`, `get_by_id()`, `create()`, `update()`, `delete()`

## Veri Akışı (Scheduler Task)

```
TaskScheduler::start()
  → repo.get_enabled() — DB'den tüm görevler okunur
  → her task için spawn_task_loop()
    → next_cron_time(task.cron_expr) ile bir sonraki çalışma zamanı hesaplanır
    → tokio::time::sleep ile bekle
    → execute_task()
      → tokio::process::Command("rclone", ["copy", src, dest, "--exclude", "*.tmp", "--progress"])
      → PID → state.task_pids'e kaydet
      → stdout: BufReader → parse_progress_line → emit("rclone:progress")
      → stderr: BufReader → emit("rclone:log")
      → child.wait() → exit code
      → PID temizle
    → TaskResult → DB'ye transfers tablosuna kaydet
    → emit("task:completed" / "task:error")
    → running listesinden çıkar → döngü başa döner
```

## Güvenlik Katmanı

- **`#![deny(unsafe_code)]`**: Tüm Rust kodu unsafe içermez
- **Tauri Capabilities**: `core:default`, `opener:default`
- **Binary Path Validation**: `verify_executable()` — Unix'te exec biti, Windows'ta `.exe` uzantısı
- **Sensitive Field Stripping**: `Remote` yapısı yalnızca `name` ve `type` döner, token/password içermez
- **Process Isolation**: Her rclone süreci ayrı child process'te çalışır
- **Cleanup**: Uygulama kapanışında `RunEvent::Exit` + `CloseRequested` → `cleanup_all()` + scheduler stop + PID kill
- **SQL Injection**: Prepared statement (`rusqlite::params!`) ile tüm sorgular parametrize edilir

## Bağımlılık Grafiği

```
Project_Overview
├── Build_Config (araç zinciri)
├── Tauri_Backend (çekirdek)
│   ├── Rclone_Integration (binary keşfi + config yönetimi)
│   │   ├── rclone/discovery.rs
│   │   └── rclone/config.rs
│   ├── Process_Manager (süreç yaşam döngüsü)
│   │   └── rclone/process.rs
│   ├── Scheduler (cron zamanlayıcı)
│   │   ├── scheduler/cron.rs
│   │   ├── scheduler/engine.rs
│   │   └── scheduler/scheduler.rs
│   ├── State_Management (durum yönetimi)
│   │   └── state.rs
│   ├── Event_Stream (gerçek zamanlı iletişim)
│   │   └── rclone/events.rs
│   ├── Tray (sistem tepsisi)
│   │   └── tray.rs
│   └── Database (kalıcı depolama)
│       └── db/migrations.rs + db/task_repo.rs
├── React_Frontend (kullanıcı arayüzü)
│   └── Architecture_Overview (şema)
└── Commands (Tauri komut katmanı)
    ├── commands/rclone_cmds.rs
    └── commands/task_cmds.rs
```

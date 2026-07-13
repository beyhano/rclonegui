# Architecture_Overview

**Özet:** RcloneGUI uygulamasının tam katmanlı mimarî şeması. Tauri v2'nin iki-proses mimarisi (Rust backend + WebView frontend) üzerine kurulmuştur. Tüm rclone entegrasyon katmanları aktiftir.

**Kütüphaneler:** Tauri v2, Rust, React 19, TypeScript, Vite 7, tokio, serde, regex, rusqlite, chrono, uuid

**Bağlantılar:** [[Project_Overview]], [[Tauri_Backend]], [[React_Frontend]], [[Rclone_Integration]], [[Process_Manager]], [[State_Management]], [[Event_Stream]]

---

## Mimarî Şema

```mermaid
graph TB
    subgraph "Process Boundary"
        subgraph "Rust Backend (Tauri Core)"
            LIB[lib.rs<br/>Tauri Builder & Setup]
            CMD[Commands<br/>commands/rclone_cmds.rs]
            ST[State Management<br/>state.rs]
            PM[Process Manager<br/>rclone/process.rs]
            EP[Event Pipeline<br/>rclone/events.rs]
            CFG[Config Management<br/>rclone/config.rs]
            DSC[Binary Discovery<br/>rclone/discovery.rs]
            DB[SQLite Persistence<br/>db/]
            
            LIB --> CMD
            LIB --> ST
            LIB --> DB
            CMD --> DSC
            CMD --> CFG
            CMD --> PM
            CMD --> EP
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
        RCLONE -->|copy/sync| FS
        RCLONE -->|mount| FS
        DB --> SQL
    end
    
    CMD -->|invoke| FE
    FE -->|invoke| CMD
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
├── lib.rs               ← Tauri builder, setup, komut kaydı, cleanup
├── state.rs             ← AppState (processes, rclone_path, db, mounts)
├── commands/
│   ├── mod.rs           ← Re-export
│   └── rclone_cmds.rs   ← 7 adet #[tauri::command]
├── rclone/
│   ├── mod.rs           ← Re-export
│   ├── discovery.rs     ← Platform algılama, binary keşfi, doğrulama
│   ├── process.rs       ← ProcessManager: spawn, stop, cleanup_all
│   ├── events.rs        ← Event stream, progress regex, emit
│   └── config.rs        ← rclone config dump → Remote listesi
└── db/
    ├── mod.rs           ← Re-export
    ├── migrations.rs    ← create_tables (transfers, mounts, app_config)
    └── models.rs        ← CRUD yapıları (Transfer, Mount, AppConfig)
```

- **Sorumluluk**: Tauri komutlarını tanımlama, state yönetimi, process kontrolü, event yayını

### 3. Process Katmanı (Aktif)

**Dosya**: `src-tauri/src/rclone/process.rs`

- **Sorumluluk**: Rclone süreçlerini spawn etme (`tokio::process::Command`), izleme, sonlandırma (`kill_on_drop(true)`)
- **ProcessManager**: `spawn()` → UUID döner, `stop()` → child.kill, `cleanup_all()` → Exit handler
- **Bağlantı**: [[Process_Manager]]

### 4. Event Katmanı (Aktif)

**Dosya**: `src-tauri/src/rclone/events.rs`

- **Sorumluluk**: rclone stdout/stderr çıktısını `BufReader` ile oku, regex ile ayrıştır, Tauri `emit()` ile frontend'e ilet
- **Regex**: `Transferred: 1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s`
- **Event'ler**: `rclone:progress`, `rclone:log`, `rclone:process-started`, `rclone:mount-status`
- **Bağlantı**: [[Event_Stream]]

### 5. Entegrasyon Katmanı (Aktif)

**Dosyalar**: `src-tauri/src/rclone/discovery.rs`, `src-tauri/src/rclone/config.rs`

- **Binary Keşfi**: `resolve_platform()` → platform belirle, `locate_binary()` → `rclone-bin/{platform}/rclone`
- **Config**: `config_list()` → `rclone config dump` JSON çıktısını parse et, `Vec<Remote>` dön
- **Bağlantı**: [[Rclone_Integration]]

### 6. Veritabanı Katmanı

**Dosyalar**: `src-tauri/src/db/migrations.rs`, `src-tauri/src/db/models.rs`

- **SQLite**: `rusqlite` bundled, otomatik oluşturulur (`app_data_dir/rclonegui.db`)
- **Tablo**: `transfers` (9 kolon), `mounts` (6 kolon), `app_config` (key-value)
- **CRUD**: `insert_transfer`, `update_transfer_status`, `get_transfer_history`, `insert_mount`, `update_mount_status`, `get_mounts`, `set_config`, `get_config`

## Veri Akışı (Copy/Sync)

```
Kullanıcı → TransferPanel (source, dest gir)
  → invoke("rclone_exec", { args: ["copy", src, dest] })
    → rclone_cmds::rclone_exec()
      → tokio::process::Command("rclone", args)
        → stdout/stderr piped → BufReader
      → UUID oluştur → state.processes'a ekle
      → start_event_stream() spawn
        → her satır için regex test
          → match → emit("rclone:progress", ProgressPayload)
          → no match → emit("rclone:log", { line })
      → emit("rclone:process-started", { process_id, command })
  → Frontend listen("rclone:progress")
    → setProgress(percent, speed, eta) → UI güncellemesi
```

## Veri Akışı (Mount)

```
Kullanıcı → MountPanel (remote, mount_point gir)
  → invoke("rclone_mount", { remote, mount_point })
    → rclone_cmds::rclone_mount()
      → tokio::process::Command("rclone", ["mount", remote, mount_point])
      → state.processes + state.mounts'a ekle
      → emit("rclone:mount-status", { mount_id, status: "running" })
  → Frontend refresh mount list

Kullanıcı → Unmount butonu
  → invoke("rclone_unmount", { mount_id })
    → ProcessManager::stop(id) → child.kill
    → state.mounts.remove(id)
  → Frontend refresh mount list
```

## Güvenlik Katmanı

- **`#![deny(unsafe_code)]`**: Tüm Rust kodu unsafe içermez
- **Tauri Capabilities**: `core:default`, `opener:default`
- **Binary Path Validation**: `verify_executable()` — Unix'te exec biti, Windows'ta `.exe` uzantısı
- **Sensitive Field Stripping**: `Remote` yapısı yalnızca `name` ve `type` döner, token/password içermez
- **Process Isolation**: Her rclone süreci ayrı child process'te çalışır
- **Cleanup**: Uygulama kapanışında `RunEvent::Exit` → `cleanup_all()` → tüm child process'ler temizlenir
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
│   ├── State_Management (durum yönetimi)
│   │   └── state.rs
│   ├── Event_Stream (gerçek zamanlı iletişim)
│   │   └── rclone/events.rs
│   └── Database (kalıcı depolama)
│       └── db/migrations.rs + db/models.rs
├── React_Frontend (kullanıcı arayüzü)
│   └── Architecture_Overview (şema)
└── Commands (Tauri komut katmanı)
    └── commands/rclone_cmds.rs
```

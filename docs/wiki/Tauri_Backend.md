# Tauri_Backend

**Özet:** Rust ile yazılmış Tauri v2 backend katmanı. 7 Tauri komutunu (`#[tauri::command]`) tanımlar, process yönetimini, SQLite veritabanını ve state'i bu katmanda barındırır. Tüm rclone entegrasyonu buradan yönetilir.

**Kütüphaneler:** tauri 2, serde 1, serde_json 1, tauri-plugin-opener 2, tokio (planlanan)

**Bağlantılar:** [[Project_Overview]], [[React_Frontend]], [[Rclone_Integration]], [[Process_Manager]], [[State_Management]], [[Event_Stream]]

---

## Mimari Katman

```rust
// src-tauri/src/main.rs  →  giriş noktası (windows_subsystem)
// src-tauri/src/lib.rs   →  Tauri builder, komutlar, plugin'ler
```

## Mevcut Yapı

```
src-tauri/src/
├── main.rs                 → #![deny(unsafe_code)], windows_subsystem, giriş
├── lib.rs                  → Tauri builder, setup, 7 komut, cleanup
├── state.rs                → AppState (processes, db, mounts, rclone_path)
├── commands/
│   └── rclone_cmds.rs      → 7 #[tauri::command] fonksiyonu
├── rclone/
│   ├── discovery.rs        → Platform tespiti, binary bulma
│   ├── process.rs          → ProcessManager (spawn, stop, cleanup_all)
│   ├── events.rs           → Regex parser + event emit pipeline
│   └── config.rs           → rclone config dump JSON parse
└── db/
    ├── migrations.rs       → create_tables (3 tablo)
    └── models.rs           → CRUD operasyonları
```

- **`main.rs`**: `#![deny(unsafe_code)]` + `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
- **`lib.rs`**: `tauri::Builder::default()` ile 7 komut kaydı, SQLite init, binary discovery, cleanup on Exit
- **`state.rs`**: `AppState` — `Mutex<HashMap<Uuid, ProcessHandle>>`, `Mutex<Connection>`, `Mutex<HashMap<Uuid, MountInfo>>`, `Mutex<Option<PathBuf>>`

## Tauri Yapılandırması

- **App identifier**: `com.beyhan.rclonegui`
- **Window**: 800×600, başlık "rclonegui"
- **CSP**: `null` (dev mode)
- **Permissions**: `core:default`, `opener:default`
- **Bundle resources**: `rclone-bin/{platform}/rclone`(.exe) paketlenir

## Komut Listesi (7 adet)

| Komut | İşlev |
|---|---|
| `rclone_version` | Binary versiyonu |
| `rclone_config_list` | Remote listesi (config dump) |
| `rclone_exec` | Rclone çalıştır + event stream |
| `rclone_stop` | Process UUID ile durdur |
| `rclone_mount` | Remote mount et |
| `rclone_unmount` | Mount'ı çöz |
| `rclone_mount_list` | Aktif mount'ları listele |

## Güvenlik

- `#![deny(unsafe_code)]` tüm crate'lerde aktif
- `capabilities/default.json` ile izin yönetimi
- Binary yolu `find_binary()` ile çoklu katmanda güvenli aranır
- Tüm `rusqlite` sorguları `params!` ile parametrize

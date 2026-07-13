# Tauri_Backend

**Özet:** Rust ile yazılmış Tauri v2 backend katmanı. Tauri komutlarını (`#[tauri::command]`) tanımlar, process yönetimini ve state'i bu katmanda barındırır. Şu an sadece `greet` komutu ile temel Tauri yapılandırması mevcuttur.

**Kütüphaneler:** tauri 2, serde 1, serde_json 1, tauri-plugin-opener 2, tokio (planlanan)

**Bağlantılar:** [[Project_Overview]], [[React_Frontend]], [[Rclone_Integration]], [[Process_Manager]], [[State_Management]], [[Event_Stream]]

---

## Mimari Katman

```rust
// src-tauri/src/main.rs  →  giriş noktası (windows_subsystem)
// src-tauri/src/lib.rs   →  Tauri builder, komutlar, plugin'ler
```

## Mevcut Yapı

- **`main.rs`**: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` ile Windows konsol penceresini gizler; `rclonegui_lib::run()` çağırır.
- **`lib.rs`**: `tauri::Builder::default()` ile uygulamayı başlatır:
  - `tauri_plugin_opener::init()` (varsayılan Tauri plugin'i)
  - `greet` komutu (geçici/should be replaced)
- **`build.rs`**: `tauri_build::build()` — standart Tauri build script'i

## Tauri Yapılandırması

- **App identifier**: `com.beyhan.rclonegui`
- **Window**: 800×600, başlık "rclonegui"
- **CSP**: `null` (dev mode)
- **Permissions**: `core:default`, `opener:default`

## Planlanan Genişleme

- Yeni `#[tauri::command]` fonksiyonları: `rclone_exec`, `rclone_stop`, `rclone_config_list`
- [[Process_Manager]] için `tokio::process::Command` kullanımı
- [[State_Management]] için `tauri::State` ile `Arc<Mutex<RcloneState>>`
- [[Event_Stream]] için `app_handle.emit("progress", payload)`

## Güvenlik

- `capabilities/default.json` ile izin yönetimi
- Gelecekte: rclone binary path'i güvenlik duvarından geçirilmeli

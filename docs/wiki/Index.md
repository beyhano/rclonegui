# RcloneGUI — Mimarî Bilgi Grafiği

**Proje:** Tauri v2 + Rust + React masaüstü uygulaması — rclone binary'si için grafiksel arayüz
**Platform:** Windows, Linux & macOS (çapraz platform)
**Durum:** Rclone entegrasyonu tamamlandı — tüm katmanlar aktif

---

## 🧱 Aktif Bileşenler (Mevcut Kod Tabanı)

| Düğüm | Açıklama | Durum |
|---|---|---|
| [[Project_Overview]] | Proje vizyonu, hedefler ve kısıtlamalar | ✅ Aktif |
| [[Tauri_Backend]] | Rust backend — Tauri v2 komutları, state, process yönetimi | ✅ Aktif |
| [[Rclone_Integration]] | rclone binary keşfi, versiyon kontrolü, config yönetimi | ✅ Aktif |
| [[Process_Manager]] | Async süreç yaşam döngüsü (spawn, izleme, temiz sonlandırma) | ✅ Aktif |
| [[State_Management]] | Tauri State ile çalışan süreçlerin PID, mount ve DB yönetimi | ✅ Aktif |
| [[Event_Stream]] | stdout/stderr ayrıştırma ve frontend'e gerçek zamanlı emit | ✅ Aktif |
| [[React_Frontend]] | React 19 + TypeScript UI katmanı (3 panel) | ✅ Aktif |
| [[Build_Config]] | Vite, pnpm, Cargo derleme araç zinciri | ✅ Aktif |
| [[Architecture_Overview]] | Tüm sistemin katmanlı mimarî şeması | ✅ Aktif |

## 📦 Frontend Panelleri

| Panel | Dosya | İşlev |
|---|---|---|
| Config | `src/ConfigPanel.tsx` | Remote listeleme, tür badge'leri |
| Transfer | `src/TransferPanel.tsx` | Copy/sync başlatma, progress bar, hız, ETA, geçmiş |
| Mounts | `src/MountPanel.tsx` | Mount bağlama/çözme, durum göstergeleri |

## ⚙️ Tauri Komutları (7 adet)

Tüm komutlar `src-tauri/src/commands/rclone_cmds.rs` içinde:

| Komut | İşlev |
|---|---|
| `rclone_version` | `rclone version` çıktısını döner |
| `rclone_config_list` | Yapılandırılmış remote'ları listeler |
| `rclone_exec` | Rclone süreci başlatır (copy/sync) |
| `rclone_stop` | Süreci UUID ile durdurur |
| `rclone_mount` | Remote dosya sistemi bağlar |
| `rclone_unmount` | Mount'ı UUID ile çözer |
| `rclone_mount_list` | Aktif mount'ları listeler |

## 🗄️ SQLite Veritabanı

`src-tauri/src/db/` içinde 3 tablo:

| Tablo | Açıklama |
|---|---|
| `transfers` | Copy/sync işlem geçmişi ve durumu |
| `mounts` | Mount süreç kayıtları |
| `app_config` | Key-value uygulama ayarları |

## 📁 Backend Modül Yapısı

```
src-tauri/src/
├── main.rs
├── lib.rs              ← Builder, setup, komut kaydı, cleanup
├── state.rs            ← AppState (processes, rclone_path, db, mounts)
├── commands/
│   ├── mod.rs
│   └── rclone_cmds.rs  ← 7 Tauri komutu
├── rclone/
│   ├── mod.rs
│   ├── discovery.rs    ← Platform algılama, binary keşfi
│   ├── process.rs      ← ProcessManager
│   ├── events.rs       ← Event pipeline, regex
│   └── config.rs       ← Config dump, Remote modeli
└── db/
    ├── mod.rs
    ├── migrations.rs   ← create_tables
    └── models.rs       ← CRUD operasyonları
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
│   ├── [[State_Management]]
│   │   └── state.rs
│   ├── [[Event_Stream]]
│   │   └── rclone/events.rs
│   └── Database
│       └── db/migrations.rs + db/models.rs
├── [[React_Frontend]]
│   └── Architecture_Overview
└── Commands
    └── commands/rclone_cmds.rs
```

---

## 📐 Tasarım İlkeleri

- **Sıfır `unsafe`**: Tüm Rust kodu `#![deny(unsafe_code)]` ile güvence altında
- **Sadece Cargo**: Sistem kütüphanesi bağımlılığı yok, pure-Rust crate'ler tercih edilir
- **Çapraz platform**: `std::path::PathBuf`, platform-agnostik sinyal yönetimi
- **Event-driven**: Uzun süren işlemler frontend'e Tauri event'leri ile iletilir
- **TDD**: Strict TDD mode aktif — her değişiklik testlerle doğrulanır
- **Parametrize SQL**: Tüm `rusqlite` sorguları `params!` ile injection korumalı

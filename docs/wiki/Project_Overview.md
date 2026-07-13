# Project_Overview

**Özet:** RcloneGUI, hem Windows hem de Linux platformlarında çalışan, rclone binary'sini yönetmek için Tauri v2 + React ile geliştirilen bir masaüstü GUI uygulamasıdır. Rclone entegrasyonu tamamlanmıştır: binary keşfi, async process yönetimi, event streaming, SQLite kalıcı depolama ve görsel UI panelleri aktiftir.

**Kütüphaneler:** Tauri v2, Rust (2021 edition), React 19, TypeScript 5.8, Vite 7, pnpm

**Bağlantılar:** [[Tauri_Backend]], [[React_Frontend]], [[Build_Config]], [[Architecture_Overview]]

---

## Vizyon

Kullanıcıların rclone işlemlerini (config, copy/sync, mount) komut satırı kullanmadan, görsel bir arayüz üzerinden yönetebilmesini sağlamak.

## Mevcut Durum

- ✅ Rclone entegrasyonu tamamlandı — binary keşfi, async process, event stream
- ✅ 7 Tauri komutu aktif (rclone_version, config_list, exec, stop, mount, unmount, mount_list)
- ✅ 3 panelli React UI (Config, Transfer, Mounts)
- ✅ SQLite kalıcı depolama (transfers, mounts, app_config)
- ✅ 57 test geçiyor, `cargo build` + `pnpm build` + `cargo clippy` 0 hata
- ✅ `/docs/wiki/` güncel Obsidian Knowledge Graph

## Kısıtlamalar (Kesin — Uygulanıyor)

1. **Sıfır `unsafe` kod** — `#![deny(unsafe_code)]` tüm crate'lerde aktif
2. **Yalnızca Cargo bağımlılıkları** — `cargo add` ile eklendi, elle müdahale yok
3. **Çapraz platform** — `std::path::PathBuf`, platform-agnostik sinyal yönetimi
4. **Event-driven** — Uzun süren işlemler Tauri `emit` ile frontend'e aktarılır

## Aktif Bileşenler

- [[Rclone_Integration]] — binary keşfi, config, exec, mount
- [[Process_Manager]] — async süreç yaşam döngüsü
- [[State_Management]] — PID, mount, DB state yönetimi
- [[Event_Stream]] — gerçek zamanlı progress/speed/ETA event'leri
- [[Tauri_Backend]] — 7 komut, lib.rs wiring, cleanup
- [[React_Frontend]] — 3 panel, event listener, progress bar
- [[Build_Config]] — Cargo, Vite, pnpm toolchain
- [[Architecture_Overview]] — katmanlı mimari şeması

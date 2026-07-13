# Project_Overview

**Özet:** RcloneGUI, hem Windows hem de Linux platformlarında çalışan, rclone binary'sini yönetmek için Tauri v2 + React ile geliştirilen bir masaüstü GUI uygulamasıdır. Şu an Tauri iskelet (scaffold) aşamasındadır; temel gereksinimler `Belge/Prompt.md`'de tanımlanmıştır.

**Kütüphaneler:** Tauri v2, Rust (2021 edition), React 19, TypeScript 5.8, Vite 7, pnpm

**Bağlantılar:** [[Tauri_Backend]], [[React_Frontend]], [[Build_Config]], [[Architecture_Overview]]

---

## Vizyon

Kullanıcıların rclone işlemlerini (config, copy/sync, mount) komut satırı kullanmadan, görsel bir arayüz üzerinden yönetebilmesini sağlamak.

## Mevcut Durum

- Tauri v2 + React + TypeScript şablonu kurulu
- Rust tarafında sadece `greet` komutu var (boilerplate)
- Frontend'de sadece varsayılan karşılama ekranı
- `Belge/Prompt.md` dosyasında tam gereksinim listesi tanımlanmış

## Kısıtlamalar (Kesin)

1. **Sıfır `unsafe` kod** — `#![deny(unsafe_code)]` aktif
2. **Yalnızca Cargo bağımlılıkları** — `Cargo.toml`'a elle müdahale yok, `cargo add` kullanılır
3. **Çapraz platform** — Windows & Linux uyumlu path/signal yönetimi
4. **Event-driven** — Uzun süren işlemler Tauri `emit` ile frontend'e aktarılır

## Hedef Mimari Bileşenler

- [[Rclone_Integration]] — binary keşfi ve config yönetimi
- [[Process_Manager]] — asenkron süreç yaşam döngüsü
- [[State_Management]] — PID ve durum takibi
- [[Event_Stream]] — gerçek zamanlı ilerleme ve hız verisi

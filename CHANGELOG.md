# Changelog

## [0.1.9] — 2025-07-15

### ✨ Yeni

- **Karadelik (Black Hole)** — Zamanlanmış görevlerde hedef olarak `/dev/null`/`NUL`. Dosyaları okur, yok eder. Hız testi ve pipeline doğrulama için.
- **Uzak Sunucu Düzenleme/Silme** — ConfigPanel'de her remote için ✏️ Düzenle ve 🗑️ Sil butonları.
- **Sağlayıcı Seçici iyileştirmesi** — Arama filtreli input + scroll edilebilir listbox.
- **SweetAlert2** — Tüm tehlikeli işlemlerde (karadelik, silme) checkbox zorunlu onay dialog'u.
- **Türkçe UI** — Tüm kullanıcı arayüzü Türkçe.

### 🐛 Düzeltmeler

- Linux'ta WebKitGTK DMABUF çakışması (`WEBKIT_DISABLE_DMABUF_RENDERER=1`)
- Sistem tepsisi oluşturulamazsa çökme yerine log + devam
- Beyaz ekran (Linux Nvidia + WebKitGTK)
- PID tracking fix: task_pids ile process sonlandırma
- Tab switch fix: Scheduler'a dönünce running task'lar geri yükleniyor
- Task edit fix: Edit butonu showForm=true yapmıyordu
- Exclude fix: `--delete-excluded` kaldırıldı
- Binary discovery fix: `resource_dir()` yetmezse `find_binary()` fallback

### 🔧 Diğer

- SAST güvenlik taraması (13 kategori, 2 Medium bulgu)
- Tray minimize & close intercept (tüm platformlar)
- Otomatik güncelleme (`tauri_plugin_updater`)
- Remote Browser (SFTP/FTP alt dizin gezgini)
- Gizli klasör filtresi
- CSS/dark mode iyileştirmeleri
- Single instance lock
- Linux sürüm yayınlama betiği (`rclone-setup.sh`)
- 81 test, 0 hata, 0 uyarı
- Dead code cleanup (703 satır silindi)

---

## [0.1.8] — 2025-07-10

### ✨ Yeni

- Cron tabanlı görev zamanlayıcı (Scheduler)
- SQLite tasks tablosu + TaskRepo CRUD
- TaskFormModal — 3 adımlı wizard
- SchedulerPage, TaskCard, CronInput bileşenleri
- ConfigFormModal — Uzak sunucu ekleme modal'ı
- ProviderSelector + ProviderConfigForm — Dinamik sağlayıcı formu
- Remote Browser (uzak klasör gezgini)

### 🐛 Düzeltmeler

- Windows test fix: echo built-in → `cmd.exe /c echo`
- Migration guard: crash'te veri kaybı önlendi
- Runtime fix: `tokio::spawn` → `tauri::async_runtime::spawn`

---

## [0.1.7] — 2025-06-28

İlk SDD pipeline'ı tamamlandı.

### ✨ Yeni

- Rclone entegrasyonu (binary keşfi, async process, event stream)
- 3 panelli React UI (Config, Transfer, Mounts)
- SQLite kalıcı depolama (transfers, mounts, app_config)
- Tauri Command katmanı (8 rclone komutu)
- System tray + minimize to tray
- Event-driven progress/speed/ETA

### 🐛 Düzeltmeler

- Platform-agnostik path yönetimi
- Cross-platform signal handling
- `#![deny(unsafe_code)]`

---

## [0.1.0] — 2025-06-15

### ✨ Yeni

- İlk sürüm. Tauri v2 + React + TypeScript iskeleti.
- Temel rclone binary keşfi ve doğrulama.
- Proje wiki'si (10 Obsidian sayfası, Knowledge Graph).
- SDD (Spec-Driven Development) süreci başlatıldı.

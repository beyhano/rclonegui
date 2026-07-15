# RcloneGUI — Yapılanlar

## 1. Proje Analizi ve Wiki Mimarisi (INGEST)

- Tüm proje analiz edildi (Rust backend, React frontend, build toolchain)
- `/docs/wiki/` altında **Obsidian Knowledge Graph** kuruldu — 10 wiki sayfası
  - `Index.md` — ana harita (tüm [[link]]'ler)
  - `Project_Overview.md` — proje vizyonu, kısıtlamalar
  - `Architecture_Overview.md` — katmanlı mimari şeması
  - `Tauri_Backend.md`, `React_Frontend.md`, `Build_Config.md`
  - `Rclone_Integration.md`, `Process_Manager.md`, `State_Management.md`, `Event_Stream.md`

## 2. SDD (Spec-Driven Development) Süreci

İlk SDD pipeline'ı:
- **Preflight**: Etkileşimli mod, hybrid depolama, exception-ok PR, 800 satır review bütçesi
- **SDD Init**: Proje bağlamı, test yetenekleri, skill registry, openspec/config.yaml
- **Proposal**: Rclone Integration değişiklik önerisi — 6 yeni yetenek
- **Spec**: 6 yetenek için 23 gereksinim, 33 Given/When/Then senaryosu
- **Design**: 11 yeni Rust dosyası, 4 frontend dosyası, SQLite veri modeli (3 tablo)
- **Tasks**: 28 task, 7 faz halinde organize edildi

İkinci SDD cycle — Task Scheduler:
- **Preflight**: Etkileşimli mod, engram depolama, exception-ok PR, 800 satır
- **Proposal**: Cron tabanlı görev zamanlayıcı
- **Spec**: tasks tablosu, scheduler, CRUD, provider config
- **Design**: SQLite + tokio + cron crate mimarisi
- **Tasks**: 18 task (Rust backend 9 + frontend 8 + test fix 1)
- **Apply**: Subagent-Driven Development ile 18 task implementasyonu

## 3. Rust Backend (src-tauri/src/)

### Foundation (Phase 1)
- `cargo add` ile bağımlılıklar: tokio, regex, uuid, chrono, rusqlite (bundled), cron
- `state.rs` — AppState (processes HashMap, db Connection, mount listesi)
- `rclone/mod.rs` — modül iskeleti (discovery, process, events, config, slug)
- `db/mod.rs`, `commands/mod.rs` — modül iskeletleri
- `main.rs`'ye `#![deny(unsafe_code)]` eklendi

### Core Backend (Phase 2)
- **`rclone/discovery.rs`** — Platform tespiti, binary konumu, executable doğrulama, çoklu yol arama
- **`rclone/process.rs`** — `ProcessManager` (stop/kill, cleanup_all)
- **`rclone/config.rs`** — `rclone config dump` JSON parse
- **`rclone/events.rs`** — Progress parse + event emit (`rclone:progress`, `rclone:log`)
- **`rclone/slug.rs`** — Türkçe karakter desteğiyle slug oluşturma

### Task Scheduler (src-tauri/src/scheduler/)
- **`cron.rs`** — Cron ifadesi parse + sonraki zaman hesaplama
- **`engine.rs`** — Görev çalıştırma (rclone spawn, progress event emit, sonuç döndürme)
- **`scheduler.rs`** — Per-task tokio loop, cron tetikleme, çakışma önleme (overlap skip)

### Database (src-tauri/src/db/)
- **`migrations.rs`** — 4 tablo (transfers, mounts, app_config, tasks) + safe migration guard
- **`task_repo.rs`** — Task model + CRUD (list, get_by_id/slug, create, update, delete, get_enabled)
- `models.rs` kaldırıldı (dead code)

### Tauri Commands
- **`commands/rclone_cmds.rs`** — 8 komut:
  - `rclone_version`, `rclone_config_list`, `rclone_exec`, `rclone_stop`
  - `rclone_mount`, `rclone_unmount`, `rclone_mount_list`
  - `rclone_config_create` — remote oluşturma
- **`commands/task_cmds.rs`** — 9 komut:
  - `task_list`, `task_create`, `task_update`, `task_delete`, `task_toggle`, `task_run_now`, `task_stop`
  - `rclone_providers` — rclone backend listesi
  - `task_running_list` — anlık çalışan task PID'leri

### Sistem Tray (src-tauri/src/tray.rs)
- Tray icon + menu (Show Window, Quit)
- Pencere kapatılınca tray'e küçülme (arka planda scheduler çalışmaya devam)
- Cross-platform (Windows, Linux, macOS)

## 4. React Frontend (src/)

### Mevcut
- **`types.ts`** — Remote, TransferRecord, MountRecord, ProgressPayload, Task, Provider, ProviderOption
- **`ConfigPanel.tsx`** — Remote listesi + "+ Add Remote" butonu
- **`TransferPanel.tsx`** — Transfer paneli (progress bar, event listener)
- **`MountPanel.tsx`** — Mount yönetimi
- **`App.tsx`** — 4-sekmeli tab router (Config / Transfer / Mounts / Scheduler)
- **`App.css`** — Tüm stiller + dark mode

### Task Scheduler UI
- **`SchedulerPage.tsx`** — Görev listesi, ekle/düzenle/sil/çalıştır, progress takibi
- **`TaskCard.tsx`** — Görev kartı (ad, schedule, operation, progress bar, Edit/Run/Toggle/Stop/Delete)
- **`TaskFormModal.tsx`** — 3-step wizard (ad+slug → kaynak/hedef path → operation+exclude+cron)
- **`ConfigFormModal.tsx`** — 2-step remote ekleme (provider seç → parametre gir)
- **`ProviderSelector.tsx`** — rclone backend seçme dropdown
- **`ProviderConfigForm.tsx`** — Dinamik form (seçilen provider'ın opsiyonlarına göre)
- **`CronInput.tsx`** — Cron ifadesi giriş + preset butonları

## 5. Testing

- **81 test** (unit + integration) — hepsi passing (4 Windows echo test fix'li)
- `cargo test`, `cargo build`, `pnpm build`, `cargo clippy` — 0 error, 0 warning
- Strict TDD uygulandı: RED → GREEN
- Test kapsamı: discovery (14), process (6), config (5), events (9), migrations (8), models (12), task_repo (12), engine (2), commands (6), state (2)

## 6. SAST Security Assessment

- 13 güvenlik taraması tamamlandı (IDOR, SQLi, SSRF, XSS, RCE, XXE, File Upload, Path Traversal, SSTI, JWT, Missing Auth, Business Logic, GraphQL)
- Final rapor: `sast/final-report.md`
- **2 Medium bulgu**:
  - `task_run_now` enabled flag'i kontrol etmiyor
  - Provider name flag injection riski (`--dry-run` vb. enjekte edilebilir)

## 7. Dead Code Cleanup

- **703 satır silindi**, 0 warning
- Kaldırılanlar: db/models.rs (Transfer/Mount/AppConfig CRUD), verify_executable, format_next_run, ProcessManager::spawn, ProcessHandle alanları (pid/command/started_at), AppState.db

## 8. Düzeltmeler

- **Binary Discovery Fix**: `resource_dir()` yetmezse `find_binary()` fallback'i
- **process-completed event fix**: Backend artık rclone çıkışında event emit ediyor
- **Migration guard**: Her startup'ta full table copy yapmaz, crash'te veri kaybı olmaz
- **Runtime fix**: `tokio::spawn` → `tauri::async_runtime::spawn` (setup'ta panic)
- **Windows test fix**: echo built-in → `cmd.exe /c echo`
- **Wiki Güncelleme**: Tüm wiki sayfaları güncellendi
- **Task edit fix**: Edit butonu showForm=true yapmıyordu, düzeltildi
- **PID tracking fix**: task_pids (task_id → PID) ile çalışan task'lar takip ediliyor, kapatınca/stop'ta öldürülüyor
- **Exclude fix**: `--delete-excluded` kaldırıldı, sadece `--exclude` ile yüklemeyi engelle
- **Stop button**: ⏹ Stop butonu — taskkill/PID ile process sonlandırma
- **Tab switch fix**: Scheduler sekmeye geri dönünce `task_running_list` ile çalışan task'lar geri yükleniyor
- **Tray Minimization & Close Intercept**: X butonuna basıldığında uygulama kapatılmak yerine sisteme gizlenir. Windows/macOS'ten sonra Linux'ta da sistem tepsisi ve kapatınca gizleme özelliği tamamen aktif hale getirildi.
- **Linux Sürüm Yayınlama Betiği**: `rclone-setup.sh` eklenerek Linux üzerinden yerel derleme, paketleme, otomatik imzalama ve GitHub Release süreçleri otomatize edildi.
- **Tauri Dialog Entegrasyonu**: `@tauri-apps/plugin-dialog` eklentisi kurulup konfigüre edilerek yerel klasörleri gözle seçme özelliği getirildi.
- **Uzak Klasör Tarayıcısı (Remote Browser)**: SFTP, FTP veya diğer uzak sunucuların alt dizinlerini `rclone lsf` ile listeleyen backend komutu (`rclone_list_dirs`) ve frontend `RemoteBrowserModal` gezgini eklendi.
- **Gizli Klasör Filtresi**: Uzak klasör tarayıcısında noktayla başlayan gizli klasörlerin listelenmesini açıp kapatan "Show hidden folders" onay kutusu yerleştirildi.
- **CSS ve Koyu Mod İyileştirmeleri**: Seçim kutuları (`select`) için `appearance: none` uygulanarak Linux temalarındaki beyaz kalma hatası giderildi, özel SVG ok işaretleri yerleştirildi ve dikey hizalamalar eşitlendi.

## 9. Değişen / Eklenen Dosyalar

### Yeni Dosyalar
```
rclone-setup.sh                    → Linux build/publish betiği
docs/wiki/                         → 10 Obsidian wiki sayfası
docs/superpowers/specs/            → Task Scheduler design doc
docs/superpowers/plans/            → Implementation plan (18 task)
sast/                              → SAST raporları (architecture, 13 tarama, final report)
sast-files/                        → SAST skill'leri
src-tauri/src/tray.rs              → Sistem tray icon + menu
src-tauri/src/scheduler/           → cron.rs, engine.rs, scheduler.rs
src-tauri/src/rclone/slug.rs       → Slug generation
src-tauri/src/db/task_repo.rs      → Task model + CRUD
src-tauri/src/commands/task_cmds.rs→ Task Tauri komutları
src/components/SchedulerPage.tsx   → Scheduler ana sayfa
src/components/TaskCard.tsx        → Görev kartı
src/components/TaskFormModal.tsx   → Görev ekleme/düzenleme wizard
src/components/ConfigFormModal.tsx → Remote ekleme modal
src/components/ProviderSelector.tsx→ Provider seçme dropdown
src/components/ProviderConfigForm.tsx→ Dinamik provider form
src/components/CronInput.tsx       → Cron giriş
```

### Değişen Dosyalar
```
package.json                       → +@tauri-apps/plugin-dialog eklentisi
src-tauri/Cargo.toml               → +cron, +tray-icon, +tauri-plugin-dialog eklentisi
src-tauri/Cargo.lock               → Bağımlılık ağacı güncellendi
src-tauri/capabilities/default.json→ +dialog:default izni
src-tauri/src/lib.rs               → scheduler, tray, close-to-tray, tauri-dialog kaydı
src-tauri/src/state.rs             → +task_repo, +scheduler
src-tauri/src/commands/rclone_cmds.rs→ +rclone_config_create, +rclone_list_dirs (klasör listeleme)
src-tauri/src/rclone/events.rs     → +process-completed event
src-tauri/src/db/migrations.rs     → +tasks tablosu, migration guard
src/App.tsx                        → 4. sekme (Scheduler)
src/App.css                        → UI, modal, select, remote browser ve ok stilleri
src/ConfigPanel.tsx                → +Add Remote butonu
src/types.ts                       → +Task, Provider, ProviderOption
```

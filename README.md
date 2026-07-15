# RcloneGUI

**rclone için Tauri v2 masaüstü GUI** — Windows, Linux ve macOS'ta rclone işlemlerini görsel arayüzden yönetin.

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)
![Rust](https://img.shields.io/badge/Rust-2021-edition-orange)
![React](https://img.shields.io/badge/React-19-61dafb)
![License](https://img.shields.io/badge/License-GPLv3-red)

---

## ⚠️ Uyarı

**Bu program ALFA sürümündedir.** Henüz kararlı değildir, beklenmedik hatalar içerebilir.

**Kullanımdan doğacak her türlü hasar, veri kaybı veya sorundan geliştirici sorumlu tutulamaz.** Programı kullanarak bu riski kabul etmiş olursunuz.

Özellikle:
- **Move/Sync** işlemleri dosyalarınızı **kalıcı olarak silebilir**
- **Karadelik (Black Hole)** özelliği dosyaları geri dönüşümsüz yok eder
- Cron görevleri beklemediğiniz zamanlarda tetiklenebilir
- Veritabanı bozulması durumunda görev kayıtlarınız kaybolabilir

**Yedek almayı ihmal etmeyin.** Üretim ortamında kullanmadan önce mutlaka test edin.

---

## Özellikler

- **Uzak Sunucu Yönetimi** — rclone remote'larını ekleme, düzenleme, silme (Google Drive, S3, Dropbox, SFTP, 50+ sağlayıcı)
- **Zamanlanmış Görevler** — Cron tabanlı görev zamanlayıcı (copy/sync/move/bisync), manuel tetikleme, durdurma
- **🐛 Karadelik (Black Hole)** — Dosyaları `/dev/null`/`NUL`'a yönlendirerek bağlantı hızı testi (bkz: [docs/wiki/karadelik.md](docs/wiki/karadelik.md))
- **Gerçek Zamanlı İlerleme** — Transfer hızı, yüzde, ETA — event-driven Tauri emit
- **SQLite Kalıcı Depolama** — Görevler, transfer geçmişi, mount kayıtları
- **Sistem Tepsisi** — Pencere kapatılınca tepsiye küçülür, scheduler arka planda çalışmaya devam eder
- **Tek Instance** — İkinci instance açılmaz, mevcut pencere odaklanır
- **Otomatik Güncelleme** — Tauri updater ile arka planda güncelleme denetimi
- **Çapraz Platform** — Windows, Linux, macOS

---

## Ekran Görüntüleri

| Uzak Sunucular | Zamanlanmış Görevler |
|---|---|
| Remote listesi, ekle/düzenle/sil | Cron görevleri, progress bar, manuel çalıştır |

---

## Kurulum

### Ön Koşullar

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/) (nightly veya stable 2021 edition)
- [Tauri v2 CLI](https://v2.tauri.app/start/prerequisites/)

### Geliştirme

```bash
# Bağımlılıkları yükle
pnpm install

# Geliştirme modunda çalıştır
pnpm tauri dev

# Production build
pnpm tauri build
```

### Platforma Özel Gereksinimler

**Linux:**
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Windows:**
- Microsoft Visual Studio C++ Build Tools
- WebView2 (Windows 10 1803+ ile gelir)

---

## Kullanım

### Uzak Sunucu Ekleme

1. **Uzak Sunucular** sekmesine git
2. "+ Uzak Sunucu Ekle" tıkla
3. Sağlayıcı seç (arama yapılabilir)
4. Gerekli parametreleri gir → kaydet

### Zamanlanmış Görev Oluşturma

1. **Zamanlanmış Görevler** sekmesine git
2. "+ Yeni Görev" tıkla
3. 3 adımlı wizard:
   - **Adım 1:** Görev adı ve slug
   - **Adım 2:** Kaynak ve hedef seç (Karadelik dahil)
   - **Adım 3:** İşlem türü, hariç tutma kalıpları, cron ifadesi
4. Kaydet → scheduler otomatik başlar

### Karadelik Kullanımı

Test ve hız ölçümü için hedef olarak Karadelik seçilebilir. Dosyalar okunur ve `/dev/null`/`NUL`'a yazılır, hiçbir şey kalıcı olmaz. Move/Sync ile kullanıldığında kaynak dosyalar silinir — dikkatli olun.

Detaylı bilgi: [docs/wiki/karadelik.md](docs/wiki/karadelik.md)

---

## Mimari

```
src/                          # React frontend
├── App.tsx                   # Sekme yönlendirici (2 sekme)
├── ConfigPanel.tsx           # Uzak sunucu listesi
├── components/
│   ├── SchedulerPage.tsx     # Görev listesi
│   ├── TaskCard.tsx          # Görev kartı
│   ├── TaskFormModal.tsx     # Görev oluşturma/düzenleme wizard
│   ├── ConfigFormModal.tsx   # Remote ekleme/düzenleme
│   ├── CronInput.tsx         # Cron ifadesi girişi
│   ├── ProviderSelector.tsx  # Sağlayıcı seçici (arama filtreli)
│   ├── ProviderConfigForm.tsx# Sağlayıcı konfigürasyon formu
│   └── RcloneUpdate.tsx      # Güncelleme kontrolü
├── types.ts                  # TypeScript arayüzleri
└── App.css                   # Global stiller

src-tauri/src/                # Rust backend
├── lib.rs                    # Tauri builder, setup, komut kaydı
├── state.rs                  # AppState (processes, task_repo, scheduler)
├── tray.rs                   # Sistem tepsisi
├── commands/
│   ├── rclone_cmds.rs        # 10 rclone komutu
│   └── task_cmds.rs          # 8 task/scheduler komutu
├── rclone/
│   ├── discovery.rs          # Binary keşfi, platform algılama
│   ├── process.rs            # ProcessManager (spawn/stop/cleanup)
│   ├── events.rs             # Progress regex + event emit
│   └── config.rs             # Config dump parse
├── scheduler/
│   ├── cron.rs               # next_cron_time()
│   ├── engine.rs             # execute_task() + Karadelik handler
│   └── scheduler.rs          # TaskScheduler (cron döngüleri)
└── db/
    ├── migrations.rs         # Tablo oluşturma (4 tablo)
    └── task_repo.rs          # Task CRUD

docs/wiki/                    # Obsidian wiki
├── Index.md                  # Ana bilgi grafiği
├── karadelik.md              # Karadelik detaylı doküman
└── ...                       # 10 wiki sayfası
```

Detaylı mimari: [docs/wiki/Architecture_Overview.md](docs/wiki/Architecture_Overview.md)

---

## Teknolojiler

| Katman | Teknoloji |
|--------|-----------|
| Frontend | React 19, TypeScript 5.8, Vite 7 |
| Backend | Rust 2021, Tauri v2 |
| Process | tokio (async process, io-util, sync) |
| DB | SQLite (rusqlite bundled) |
| Scheduler | cron crate, tokio time |
| GUI | Tauri tray-icon, dialog, updater |
| Paket | pnpm |

### Güvenlik

- ✅ `#![deny(unsafe_code)]` — tüm crate'lerde
- ✅ Parametrize SQL — `rusqlite::params!` ile injection koruması
- ✅ SAST taraması — 13 güvenlik kategorisi
- ✅ Binary path validation
- ✅ Process isolation
- ✅ Cleanup: Exit handler + PID kill

---

## Test

```bash
# Rust testleri
cd src-tauri && cargo test

# Clippy
cd src-tauri && cargo clippy

# Frontend build
pnpm build
```

---

## Geliştirme

```bash
# Geliştirme sunucusu
pnpm tauri dev

# Sadece frontend
pnpm dev

# Build
pnpm tauri build
```

### Kod Standartları

- Strict TDD: RED → GREEN döngüsü
- Conventional commits
- Sıfır unsafe kod
- Çapraz platform uyumu (`std::path::PathBuf`, platform-agnostik sinyaller)

---

## Lisans

**GNU Genel Kamu Lisansı v3.0** — [LICENSE](LICENSE)

```
RcloneGUI - rclone için masaüstü GUI
Copyright (C) 2024 Beyhan Oğur

Bu program özgür bir yazılımdır; Free Software Foundation tarafından 
yayımlanan GNU Genel Kamu Lisansı'nın 3. sürümü veya (isteğe bağlı olarak) 
daha yeni bir sürümü koşulları altında dağıtabilir ve/veya değiştirebilirsiniz.

Bu program, yararlı olması umuduyla dağıtılmıştır, ancak HİÇBİR GARANTİSİ YOKTUR;
Ticari Olarak Satılabilirlik veya BELİRLİ BİR AMACA UYGUNLUK için bile zımni
bir garanti vermez. Ayrıntılar için GNU Genel Kamu Lisansı'na bakın.
```

---

## İlgili Projeler

- [rclone/rclone](https://github.com/rclone/rclone) — rsync for cloud storage
- [rclone-ui/rclone-ui](https://github.com/rclone-ui/rclone-ui) — Web UI for rclone

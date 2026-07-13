# Task Scheduler — RcloneGUI Tasarım Dokümanı

## Özet

RcloneGUI'ye kullanıcı tanımlı görevleri zamanlayarak çalıştıran bir task scheduler sistemi ekleniyor. Kullanıcı rclone storage provider'ları arasından kaynak ve hedef seçer, işlem tipini (copy/sync/move/bisync) belirler, exclude pattern'ları girer, cron ifadesiyle zamanlamasını yapar. Sistem arka planda çalışır, vadesi gelen görevi otomatik tetikler ve sonucu kaydeder.

## Veri Modeli

### tasks tablosu (yeni)

```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,                    -- UUID v4
    name TEXT NOT NULL,                      -- Kullanıcı tarafından verilen görev adı (serbest metin)
    slug TEXT NOT NULL UNIQUE,               -- Programatik ID ("yedekleme-isi-2")
    source_provider TEXT NOT NULL,           -- Kaynak provider tipi (drive, s3, vs.)
    source_config TEXT NOT NULL,             -- Kaynak parametreler (JSON)
    dest_provider TEXT NOT NULL,             -- Hedef provider tipi
    dest_config TEXT NOT NULL,               -- Hedef parametreler (JSON)
    operation TEXT NOT NULL,                 -- copy | sync | move | bisync
    exclude_patterns TEXT NOT NULL,          -- JSON array ["*.tmp", "node_modules"]
    cron_expr TEXT NOT NULL,                 -- Cron ifadesi ("0 15 * * * *")
    enabled INTEGER NOT NULL DEFAULT 1,      -- 1=aktif, 0=pasif
    created_at TEXT NOT NULL,                -- ISO 8601
    updated_at TEXT NOT NULL                 -- ISO 8601
);
```

### transfers tablosu (mevcut — güncellenecek)

```sql
-- Mevcut sütunlara ek olarak:
-- task_id TEXT, -- FOREIGN KEY → tasks.id (nullable, manuel transferlerde null)
```

## Backend Mimarisi

### Yeni modüller

```
src-tauri/src/
├── scheduler/
│   ├── mod.rs           # TaskScheduler: cron yönetimi, tetikleme, süreç listesi
│   ├── engine.rs        # Görev çalıştırma (rclone exec wrapper + sonuç kaydı)
│   └── cron.rs          # Cron ifadesi parse + sonraki zaman hesaplama
├── db/
│   ├── migrations.rs    # + tasks tablosu migration'ı
│   ├── models.rs        # + task CRUD fonksiyonları
│   ├── task_repo.rs     # TaskRepo: tasklara özel veri erişim katmanı
└── commands/
    └── task_cmds.rs     # Task CRUD + scheduler kontrol Tauri komutları
```

### TaskScheduler çalışma prensibi

1. **App startup:** `AppState::setup()` içinde `TaskScheduler::new(db, rclone_path)` oluşturulur
2. **Görev yükleme:** DB'de `enabled = 1` olan tüm görevler yüklenir
3. **Per-görev tokio task:** Her görev için ayrı bir tokio task spawn edilir:
   - Cron expression'dan bir sonraki çalışma zamanı hesaplanır
   - `tokio::time::sleep_until()` ile o ana kadar beklenir
   - Vakit gelince `engine.rs` aracılığıyla rclone çalıştırılır
   - Çakışma önleme: görev zaten çalışıyorsa bu cycle atlanır (overlap skip)
   - Engine, rclone çıkışında `rclone:process-completed` / `rclone:process-error` event'lerini emit eder (mevcut eksiklik giderilir)
   - Sonuç `transfers` tablosuna kaydedilir
   - Frontend'e event gönderilir (`task:completed` / `task:error`)
   - Bir sonraki çalışma için döngü başa sarılır
4. **Yönetim:** Yeni görev eklendiğinde/silinecek/düzenlendiğinde scheduler dinamik olarak güncellenir

### Tauri Komutları

| Komut | Yöntem | Açıklama |
|---|---|---|
| `task_list` | invoke | Tüm görevleri listeler |
| `task_create` | invoke | Yeni görev oluşturur, scheduler'a ekler |
| `task_update` | invoke | Görev düzenler, scheduler'ı günceller |
| `task_delete` | invoke | Görev siler, scheduler'dan çıkarır |
| `task_toggle` | invoke | enabled/disabled geçişi |
| `task_run_now` | invoke | Sıradaki zamanı beklemeden hemen çalıştırır |
| `task:completed` | event | Görev başarıyla tamamlandı |
| `task:error` | event | Görev hatayla sonuçlandı |

### Bağımlılıklar

- `cron` crate — cron expression parse + next time hesaplama
- Tablo migration versiyonlama (manuel veya basit versiyon kontrolü)

## Frontend Mimarisi

### Yeni dosyalar

```
src/
├── components/
│   ├── SchedulerPage.tsx      # Ana scheduler sayfası (görev listesi + yeni görev)
│   ├── TaskCard.tsx           # Tek görev kartı (ad, schedule, durum, son çalışma)
│   ├── TaskFormModal.tsx      # Yeni görev formu (step-by-step wizard)
│   ├── ProviderSelector.tsx   # Provider seçme dropdown (rclone backends listesi)
│   ├── ProviderConfigForm.tsx # Dinamik form: seçilen provider'ın parametreleri
│   └── CronInput.tsx          # Cron ifadesi giriş + özet gösterimi
├── App.tsx                    # + Scheduler sekmesi (4. sekme)
├── App.css                    # + Yeni görev stilleri
└── types.ts                   # + Task, TaskForm, CronSchedule tipleri
```

### TaskFormModal akışı (step-by-step)

```
Step 1: Görev adı + slug (ad otomatik slug'a dönüşür, kullanıcı düzenleyebilir)
Step 2: Provider seçimi (kaynak ve hedef)
Step 3: Provider parametreleri (dinamik form)
Step 4: Exclude pattern'lar (opsiyonel, birden çok)
Step 5: Schedule — cron ifadesi
```

### Provider Config dinamik form

Frontend `rclone config providers` JSON'ından gelen `Options[]` dizisini kullanarak her provider için dinamik form oluşturur:
- `Type: string` → `<input type="text">`
- `Type: bool` → `<input type="checkbox">`
- `Type: int` → `<input type="number">`
- `Required: true` → zorunlu alan işareti
- `Examples` → `<select>` dropdown önerisi
- `IsPassword: true` → `<input type="password">`
- `Advanced: true` → "Gelişmiş" bölümü altında göster

## Planlanan Uygulama Sırası

### Faz 1: Rust Backend (DB + Scheduler Core)
- Bağımlılık ekleme (`cron` crate)
- `tasks` tablosu migration
- TaskRepo CRUD
- Cron parser + sonraki zaman hesaplama
- TaskScheduler (per-task tokio loop)
- Engine (rclone exec wrapper + sonuç kaydı)
- Task Tauri komutları

### Faz 2: Frontend (Form + Liste)
- TypeScript tipleri
- ProviderSelector (rclone backends listesi)
- ProviderConfigForm (dinamik form)
- CronInput
- TaskFormModal (step wizard)
- TaskCard
- SchedulerPage
- App.tsx'e 4. sekme ekleme

### Faz 3: Integration + Test
- Testler (Rust tarafı)
- Event sistemi entegrasyonu
- Clippy warning temizliği
- Mevcut 4 Windows test düzeltmesi

## Kapsam Dışı

- Görevler arası bağımlılık / DAG
- E-posta/bildirim gönderme
- Görev tahmini süre hesaplama
- Provider credential yönetimi (rclone config'dan gelir)

## Slug Oluşturma Kuralı

- Ad küçük harfe çevrilir
- Boşluklar `-` tireye dönüşür
- Türkçe karakterler ascii karşılığına dönüşür (ş→s, ı→i, ü→u, ö→o, ç→c, ğ→g)
- Noktalama/semboller temizlenir
- Birden çok tire tek tireye indirgenir
- Başta/sonda tire varsa kırpılır
- Kullanıcı otomatik oluşan slug'ı manuel değiştirebilir (benzersiz olmalı)

## Altyapı Kullanımı

- Mevcut SQLite veritabanı kullanılır
- Mevcut process manager (`rclone/process.rs`) ile entegre çalışır
- Mevcut event stream (`rclone/events.rs`) tamamlanır (process-completed/error event'leri eklenir)
- Mevcut `lib.rs`'deki setup akışına TaskScheduler init eklenir

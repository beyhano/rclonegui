# React_Frontend

**Özet:** React 19 + TypeScript ile yazılmış, Vite 7 tarafından derlenen frontend katmanı. Tauri backend ile `@tauri-apps/api` üzerinden haberleşir. 4 panelli arayüz: Config (remote listesi), Transfer (progress bar + geçmiş), Mounts (bağlama/çözme) ve Scheduler (cron tabanlı görev zamanlayıcı).

**Kütüphaneler:** react 19, react-dom 19, @tauri-apps/api 2, @tauri-apps/plugin-opener 2, @tauri-apps/plugin-dialog 2 (klasör seçici için), @vitejs/plugin-react, TypeScript 5.8

**Bağlantılar:** [[Tauri_Backend]], [[Build_Config]], [[Event_Stream]], [[Architecture_Overview]]

---

## Mevcut Yapı

```
src/
├── App.tsx               → 4-sekmeli tab router (Config, Transfer, Mounts, Scheduler)
├── App.css               → Tab nav, progress bar, badge'ler, modal, dark mode stilleri
├── ConfigPanel.tsx       → Remote listesi + tür badge'i
├── TransferPanel.tsx     → Progress bar, speed/ETA, stop, history
├── MountPanel.tsx        → Mount/unmount, status badge'leri
├── types.ts              → TypeScript interface'leri
├── main.tsx              → ReactDOM.createRoot giriş noktası
├── vite-env.d.ts         → Vite tip tanımları
├── components/           → Alt bileşenler
│   ├── SchedulerPage.tsx   → Scheduler ana sayfası ve listesi
│   ├── TaskCard.tsx        → Zamanlanmış görev kartı
│   ├── TaskFormModal.tsx   → 3-step görev ekleme/düzenleme sihirbazı
│   ├── ConfigFormModal.tsx → Remote ekleme modalı
│   ├── ProviderSelector.tsx→ Rclone sağlayıcı seçici
│   ├── ProviderConfigForm.tsx→ Dinamik sağlayıcı parametre formu
│   └── CronInput.tsx       → Cron giriş alanı ve hazır preset butonları
└── assets/
    └── react.svg
```

## Backend ile İletişim

- **Komut çağrısı**: `import { invoke } from "@tauri-apps/api/core"` → `invoke("rclone_config_list")`
- **Event dinleme**: `import { listen } from "@tauri-apps/api/event"` → `listen("rclone:progress", callback)`

## Paneller ve Bileşenler

### ConfigPanel
- `invoke("rclone_config_list")` ile remote'ları listeler
- Her remote için tür badge'i (drive, s3, vs.)
- Loading / error / empty durum yönetimi
- "+ Add Remote" butonu ile `ConfigFormModal`'ı açar.

### TransferPanel
- Kaynak ve hedef input alanları
- "Start Transfer" butonu → `invoke("rclone_exec")`
- Progress bar (yüzde dolum animasyonlu)
- Anlık hız (MiB/s) ve ETA göstergesi
- Stop butonu → `invoke("rclone_stop")`
- Geçmiş tablosu (lokal state)
- `rclone:progress`, `rclone:process-completed`, `rclone:process-error` event listener'ları

### MountPanel
- `invoke("rclone_mount_list")` ile aktif mount'lar
- Remote seçici + mount point input
- Mount/unmount butonları
- Durum badge'leri (🟢 running / 🔴 stopped / 🟡 error)
- `rclone:mount-status` event listener'ı

### Scheduler (Zamanlayıcı)
- `SchedulerPage` üzerinden cron görevlerini listeler, etkinleştirir/devre dışı bırakır (`task_toggle`), siler (`task_delete`), manuel çalıştırır (`task_run_now`) veya durdurur (`task_stop`).
- **TaskFormModal (3 Adımlı Kurulum):**
  1. *Adım 1:* Görev adı ve otomatik slug oluşturucu.
  2. *Adım 2:* Kaynak ve Hedef dizin seçimi. Yerel klasörler için `tauri-plugin-dialog` klasör diyaloğu açılır. Uzak sunucu klasörleri için `rclone_list_dirs` komutuyla uzak sunucuyu gezen `RemoteBrowserModal` açılır.
  3. *Adım 3:* İşlem türü (copy, sync, move, bisync), hariç tutma kuralları ve cron zamanlaması (`CronInput`).
- **RemoteBrowserModal:** Uzak sunucudaki dizin yapısını listeler. Klasör gezinimi (alt dizine girme / üst dizine çıkma) ve noktayla başlayan gizli dosyaları gizleme/gösterme (`Show hidden folders` filtresi) desteği sunar.

## Tasarım Kararları

- **State yönetimi**: Her panel kendi `useState`'ini yönetir.
- **Event akışı**: Event listener'lar `useEffect` içinde, stale closure koruması için `cancelled` flag + `useRef` kullanılır.
- **Duyarlılık ve Görsellik**: CSS `prefers-color-scheme` ile tam koyu mod desteği. İşletim sistemi yerel select stilleri `appearance: none` ve özel SVG oklarla standartlaştırılmıştır. Dar ekranlar için grid/flex yapıları alt alta gelecek şekilde responsive yapılmıştır.
- **Stil**: Minimalist vanilya CSS, harici framework yoktur. Girdiler `height: 38px` ve `box-sizing: border-box` ile dikeyde hizalanmıştır.

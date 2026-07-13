# React_Frontend

**Özet:** React 19 + TypeScript ile yazılmış, Vite 7 tarafından derlenen frontend katmanı. Tauri backend ile `@tauri-apps/api` üzerinden haberleşir. 3 panelli arayüz: Config (remote listesi), Transfer (progress bar + geçmiş), Mounts (bağlama/çözme).

**Kütüphaneler:** react 19, react-dom 19, @tauri-apps/api 2, @tauri-apps/plugin-opener 2, @vitejs/plugin-react, TypeScript 5.8

**Bağlantılar:** [[Tauri_Backend]], [[Build_Config]], [[Event_Stream]], [[Architecture_Overview]]

---

## Mevcut Yapı

```
src/
├── App.tsx               → 3-sekmeli tab router
├── App.css               → Tab nav, progress bar, badge'ler, dark mode
├── ConfigPanel.tsx       → Remote listesi + tür badge'i
├── TransferPanel.tsx     → Progress bar, speed/ETA, stop, history
├── MountPanel.tsx        → Mount/unmount, status badge'leri
├── types.ts              → TypeScript interface'leri
├── main.tsx              → ReactDOM.createRoot giriş noktası
├── vite-env.d.ts         → Vite tip tanımları
└── assets/
    └── react.svg
```

## Backend ile İletişim

- **Komut çağrısı**: `import { invoke } from "@tauri-apps/api/core"` → `invoke("rclone_config_list")`
- **Event dinleme**: `import { listen } from "@tauri-apps/api/event"` → `listen("rclone:progress", callback)`

## Paneller

### ConfigPanel
- `invoke("rclone_config_list")` ile remote'ları listeler
- Her remote için tür badge'i (drive, s3, vs.)
- Loading / error / empty durum yönetimi

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

## Tasarım Kararları

- **State yönetimi**: Her panel kendi `useState`'ini yönetir. Gelecekte Zustand'a geçilebilir
- **Event akışı**: Event listener'lar `useEffect` içinde, stale closure koruması için `cancelled` flag + `useRef` kullanılır
- **Stil**: CSS `prefers-color-scheme` ile dark mode, minimal CSS framework yok

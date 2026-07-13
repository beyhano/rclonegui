# React_Frontend

**Özet:** React 19 + TypeScript ile yazılmış, Vite 7 tarafından derlenen frontend katmanı. Tauri backend ile `@tauri-apps/api` üzerinden haberleşir. Şu an varsayılan Tauri şablonu içeriğine sahiptir.

**Kütüphaneler:** react 19, react-dom 19, @tauri-apps/api 2, @tauri-apps/plugin-opener 2, @vitejs/plugin-react, TypeScript 5.8

**Bağlantılar:** [[Tauri_Backend]], [[Build_Config]], [[Event_Stream]], [[Architecture_Overview]]

---

## Mevcut Yapı

```
src/
├── App.tsx          → Ana bileşen (varsayılan greet formu)
├── App.css          → Stil (light/dark mode, responsive)
├── main.tsx         → ReactDOM.createRoot giriş noktası
├── vite-env.d.ts    → Vite tip tanımları
└── assets/
    └── react.svg    → React logosu
```

## Backend ile İletişim

- **Komut çağrısı**: `import { invoke } from "@tauri-apps/api/core"` → `invoke("greet", { name })`
- **Event dinleme** (planlanan): `import { listen } from "@tauri-apps/api/event"` → `listen("rclone-progress", callback)`

## UI Durumu

- **Şu an**: Tek sayfa, greet input + buton, stateless
- **Planlanan**:
  - Rclone config yönetimi sayfası
  - Dosya transfer paneli (ilerleme çubuğu, hız göstergesi)
  - Mount yönetimi
  - Koyu/açık tema desteği (CSS `prefers-color-scheme` zaten hazır)

## Tasarım Kararları

- **State yönetimi**: Şu an React `useState` kullanılıyor; planlanan karmaşıklık için [[State_Management]] düğümüne bakın
- **Event akışı**: Tauri event'leri frontend'de `listen` ile yakalanacak — [[Event_Stream]]

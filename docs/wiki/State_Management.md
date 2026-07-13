# State_Management

**Özet:** Çalışan rclone süreçlerinin ve uygulama durumunun Tauri'nin `State` yapısı ile yönetimi. **Henüz implemente edilmedi.**

**Kütüphaneler:** serde, tokio (planlanan)

**Bağlantılar:** [[Tauri_Backend]], [[Process_Manager]], [[Rclone_Integration]], [[Architecture_Overview]]

---

## Tauri State Deseni

Tauri v2'de state, `tauri::Builder::default().manage()` ile kaydedilir ve `State<'_, T>` ile komutlarda erişilir.

## Planlanan State Yapısı

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use serde::Serialize;

#[derive(Clone, Serialize)]
struct RcloneProcessInfo {
    id: Uuid,
    pid: u32,
    command: String,
    status: ProcessStatus,
    started_at: String,
}

#[derive(Clone, Serialize)]
enum ProcessStatus {
    Running,
    Stopping,
    Completed(i32),     // exit code
    Failed(String),     // error message
}

struct AppState {
    processes: Arc<Mutex<HashMap<Uuid, RcloneProcessInfo>>>,
    rclone_path: Option<PathBuf>,
}
```

## State'i Kaydetme

```rust
fn main() {
    tauri::Builder::default()
        .manage(AppState {
            processes: Arc::new(Mutex::new(HashMap::new())),
            rclone_path: None,
        })
        .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Thread Güvenliği

- `Arc<Mutex<T>>`: State paylaşımı için standart desen
- Tokio `Mutex` yerine `std::sync::Mutex` de kullanılabilir (kısa kilitlenmeler için)
- IPC'den gelen her komut kendi async context'inde çalışır

## Frontend State

Frontend'de React state yönetimi (henüz karar verilmedi):
- **Seçenek 1**: React Context + useReducer — basit başlangıç
- **Seçenek 2**: Zustand — orta ölçekli projeler için ideal (önerilen)
- **Seçenek 3**: TanStack Query — Tauri invoke çağrıları için uygun

> **Karar**: Şu an sadece `useState` kullanılıyor. Karmaşıklık arttıkça Context veya Zustand'a geçiş yapılabilir.

# Event_Stream

**Özet:** Rclone süreçlerinin stdout/stderr çıktısını regex ile ayrıştırarak Tauri event'leri (`emit`) ile frontend'e gerçek zamanlı ileten pipeline. Aktif ve çalışır durumdadır.

**Kütüphaneler:** tokio, regex, serde, serde_json

**Bağlantılar:** [[Tauri_Backend]], [[Process_Manager]], [[React_Frontend]], [[Rclone_Integration]]

---

## Gerçekleşen Pipeline

Proje dosyası: `src-tauri/src/rclone/events.rs`

```
stdout/stderr (tokio::process::Command)
    │
    ▼
BufReader (tokio::io::BufReader) — line-by-line
    │
    ▼
parse_progress_line() — OnceLock<Regex> ile derlenmiş
    │
    ▼
app_handle.emit("rclone:progress", ProgressPayload)
    │
    ▼
Frontend: listen("rclone:progress", callback)
```

## Regex Deseni

Regex `OnceLock` içinde bir kere derlenir (lazy_static yerine Rust 1.80+ standardı):

```rust
use std::sync::OnceLock;

fn progress_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"Transferred:\s+(?P<transferred>[\d.]+\s*\w*)\s+/\s+(?P<total>[\d.]+\s*\w*),\s+(?P<percent>\d+)%,\s+(?P<speed>[\d.]+\s*\w*/s)(?:,\s+ETA\s+(?P<eta>[\w\d-]+))?"
        ).expect("invalid regex")
    })
}
```

ETA alanı `[\w\d-]` ile genişletilmiştir — rclone bazen `ETA -` (bilinmiyor) döndürür.

## Event Payload'ları

Events.rs'de tanımlı `ProgressPayload`:

```rust
#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub process_id: String,
    pub transferred: String,
    pub total: String,
    pub percent: u8,
    pub speed: String,
    pub eta: String,
}
```

## Frontend'de Dinleme (TransferPanel.tsx)

```typescript
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
    let cancelled = false;
    const unlisten = listen<ProgressPayload>("rclone:progress", (event) => {
        if (cancelled) return;
        if (event.payload.process_id !== currentPidRef.current) return;
        setProgress(event.payload.percent);
        setSpeed(event.payload.speed);
    });
    return () => { cancelled = true; unlisten.then(fn => fn()); };
}, []);
```

## Event Listesi

| Event | Payload | Açıklama |
|---|---|---|
| `rclone:progress` | `ProgressPayload` | İlerleme yüzdesi, hız, ETA |
| `rclone:process-started` | `{process_id}` | Yeni süreç başladı |
| `rclone:process-completed` | `{process_id, exit_code}` | Süreç tamamlandı |
| `rclone:process-error` | `{process_id, stderr_lines}` | Hata durumu |
| `rclone:mount-status` | `{mount_id, status}` | Mount durum değişikliği |
| `rclone:binary-missing` | `{platform}` | Binary bulunamadı |

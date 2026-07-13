# Event_Stream

**Özet:** Rclone süreçlerinin stdout/stderr çıktısını regex ile ayrıştırarak Tauri event'leri (`emit`) ile frontend'e gerçek zamanlı ileten pipeline. **Henüz implemente edilmedi.**

**Kütüphaneler:** tokio (planlanan), regex (planlanan), serde, serde_json

**Bağlantılar:** [[Tauri_Backend]], [[Process_Manager]], [[React_Frontend]], [[Rclone_Integration]]

---

## Planlanan Pipeline

```
rclone stdout/stderr
    │
    ▼
AsyncReader (tokio::io::BufReader)
    │
    ▼
Line Parser (her satırı regex ile eşleştir)
    │
    ▼
Event Payload (serde_json)
    │
    ▼
app_handle.emit("rclone:progress", payload)  →  Frontend listen()
```

## Regex Desenleri

Rclone'un ilerleme çıktısı (`--progress` flag'i ile) şu formattadır:

```
Transferred:   	  1.190 GiB / 1.190 GiB, 100%, 12.034 MiB/s, ETA 0s
```

Planlanan regex:

```rust
lazy_static! {
    static ref PROGRESS_RE: Regex = Regex::new(
        r"(?x)
        Transferred:\s+
        (?P<transferred>[\d.]+ \s* \w+)\s+/\s+
        (?P<total>[\d.]+ \s* \w+),\s+
        (?P<percent>\d+)%,\s+
        (?P<speed>[\d.]+\s*\w+/s)
        "
    ).unwrap();
}
```

## Event Payload'ları

```rust
#[derive(Clone, Serialize)]
struct ProgressPayload {
    transferred: String,   // "1.190 GiB"
    total: String,         // "1.190 GiB"
    percent: u8,           // 100
    speed: String,         // "12.034 MiB/s"
    eta: String,           // "0s"
}

#[derive(Clone, Serialize)]
struct ProcessEvent {
    process_id: Uuid,
    event_type: ProcessEventType,
    message: Option<String>,
}

enum ProcessEventType {
    Started,
    Progress(ProgressPayload),
    StdoutLine(String),
    StderrLine(String),
    Completed(i32),
    Failed(String),
}
```

## Frontend'de Dinleme

```typescript
import { listen } from "@tauri-apps/api/event";

// Component mount'da
const unlisten = await listen<RcloneProgress>("rclone:progress", (event) => {
    setProgress(event.payload.percent);
    setSpeed(event.payload.speed);
});

// Component unmount'da
unlisten();
```

## Event İsimlendirme Kuralı

Tüm event'ler `rclone:` namespace'i altında:
- `rclone:progress` — ilerleme durumu
- `rclone:process-started` — yeni süreç başladı
- `rclone:process-completed` — süreç tamamlandı
- `rclone:process-error` — hata durumu
- `rclone:log` — genel log satırı
